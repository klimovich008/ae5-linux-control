#!/usr/bin/env bash
# Watch for the CA0132 idle DSP self-oscillation and record when it starts.
#
# Background: with global OutFX enabled the AE-5 DSP can enter a state where
# it emits a continuous ~61-65 Hz harmonic stack with no stream playing. It
# survives an effect-parameter reset, a PipeWire restart, and an OutFX
# toggle, so it appears to require a DSP re-download to clear. This monitor
# samples the card's internal What U Hear tap periodically and logs the
# level so the onset can be correlated with whatever triggered it.
#
# The tap sits after the hardware Master switch, so Master must be on for
# detection to work. Sampling is passive: it opens a short capture and
# changes no playback state.
set -euo pipefail

INTERVAL="${AE5_MONITOR_INTERVAL:-60}"
SAMPLE_SECONDS="${AE5_MONITOR_SAMPLE:-1}"
# RMS dBFS above which the tap is considered to be oscillating. The clean
# floor measured with OutFX off is about -38 dB; the fault sits near -6 dB.
THRESHOLD="${AE5_MONITOR_THRESHOLD:--25}"
LOG="${AE5_MONITOR_LOG:-$HOME/.local/state/ae5-dsp-monitor.csv}"

die() { printf 'error: %s\n' "$*" >&2; exit 1; }

wuh_device() {
    arecord -l 2>/dev/null | awk '
        tolower($0) ~ /what u hear/ {
            if (match($0, /^card ([0-9]+).*device ([0-9]+)/, m))
                { print m[1] "," m[2]; exit }
        }'
}

sample_rms() {
    local dev="$1" tmp
    tmp="$(mktemp -t ae5-mon-XXXXXX.wav)"
    if arecord -D "hw:${dev}" -f S32_LE -c 2 -r 48000 \
            -d "$SAMPLE_SECONDS" "$tmp" >/dev/null 2>&1; then
        sox "$tmp" -n stats 2>&1 | awk '/RMS lev dB/{print $4; found=1} END{if(!found) print "nan"}'
    else
        echo "nan"
    fi
    rm -f "$tmp"
}

card_state() {
    local outfx pcm master
    outfx="$(amixer -c 0 sget 'Enable OutFX' 2>/dev/null \
        | grep -oE '\[(on|off)\]' | head -1 | tr -d '[]')"
    master="$(amixer -c 0 sget Master 2>/dev/null \
        | grep -oE '\[(on|off)\]' | head -1 | tr -d '[]')"
    pcm="closed"
    for s in /proc/asound/card0/pcm*p/sub*/status; do
        [ -f "$s" ] || continue
        if [ "$(sed -n '1p' "$s")" != "closed" ]; then pcm="running"; break; fi
    done
    printf '%s,%s,%s' "${outfx:-?}" "${master:-?}" "$pcm"
}

main() {
    command -v arecord >/dev/null 2>&1 || die "arecord is required"
    command -v sox >/dev/null 2>&1 || die "sox is required"
    local dev
    dev="$(wuh_device)" || true
    [ -n "${dev:-}" ] || die "What U Hear capture device not found"

    mkdir -p "$(dirname "$LOG")"
    [ -s "$LOG" ] || echo "timestamp,uptime_s,rms_dbfs,outfx,master,pcm,verdict" >> "$LOG"

    printf 'monitoring hw:%s every %ss, threshold %s dBFS\n' \
        "$dev" "$INTERVAL" "$THRESHOLD"
    printf 'logging to %s (Ctrl-C to stop)\n' "$LOG"

    local announced=0
    while true; do
        local rms state verdict up
        rms="$(sample_rms "$dev")"
        state="$(card_state)"
        up="$(awk '{printf "%d", $1}' /proc/uptime)"
        verdict="clean"
        if [ "$rms" != "nan" ] \
           && awk -v r="$rms" -v t="$THRESHOLD" 'BEGIN{exit !(r>t)}'; then
            verdict="OSCILLATING"
            if [ "$announced" -eq 0 ]; then
                announced=1
                printf '\n*** oscillation first detected at uptime %ss (RMS %s dBFS) ***\n' \
                    "$up" "$rms"
            fi
        fi
        printf '%s,%s,%s,%s,%s\n' \
            "$(date -Is)" "$up" "$rms" "$state" "$verdict" >> "$LOG"
        printf '%s  uptime %6ss  RMS %8s dBFS  [%s]  %s\n' \
            "$(date +%H:%M:%S)" "$up" "$rms" "$state" "$verdict"
        sleep "$INTERVAL"
    done
}

case "${1:-}" in
    -h|--help)
        cat <<'EOF'
usage: dsp-oscillation-monitor.sh

Samples the AE-5 What U Hear tap on an interval and logs whether the idle
DSP oscillation is present. Requires hardware Master on (the tap is after
that switch). Start it right after a reboot and use the machine normally;
the log records the uptime at which the fault first appears.

Environment:
  AE5_MONITOR_INTERVAL   seconds between samples (default 60)
  AE5_MONITOR_SAMPLE     capture length per sample (default 1)
  AE5_MONITOR_THRESHOLD  RMS dBFS onset threshold (default -25)
  AE5_MONITOR_LOG        CSV path (default ~/.local/state/ae5-dsp-monitor.csv)
EOF
        ;;
    --self-test)
        command -v arecord >/dev/null 2>&1 || die "arecord is required"
        command -v sox >/dev/null 2>&1 || die "sox is required"
        card_state >/dev/null
        awk -v r=-6 -v t=-25 'BEGIN{exit !(r>t)}' || die "threshold logic broken"
        awk -v r=-38 -v t=-25 'BEGIN{exit (r>t)}' || die "threshold logic broken"
        echo "dsp-oscillation-monitor self-test passed"
        ;;
    *) main ;;
esac
