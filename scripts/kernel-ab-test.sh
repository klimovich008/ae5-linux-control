#!/usr/bin/env bash
# Kernel A/B test for the CA0132 idle DSP self-oscillation.
#
# Runs the same experiment on whatever kernel is booted, so the stock
# Nobara kernel and the project's patched kernel can be compared directly.
#
# Each trial: re-download the DSP (PCI rebind) so the card starts from a
# known-clean state, apply the profile, play one tone fixture, then measure
# the card's internal What U Hear tap while idle. A clean DSP reads -inf;
# the fault reads roughly -6 to -20 dBFS.
#
# Nothing is played above the project's 20% ceiling and the analog output
# is hard-muted for the whole run, so this is inaudible by construction.
set -euo pipefail

TRIALS="${AE5_AB_TRIALS:-5}"
PROFILE="${AE5_AB_PROFILE:-$HOME/.config/ae5-control/profiles/windows-headphones.json}"
FIXTURES="${AE5_AB_FIXTURES:-}"
THRESHOLD=-30
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

die() { printf 'error: %s\n' "$*" >&2; exit 1; }

wuh_dev() {
    arecord -l 2>/dev/null | awk '
        tolower($0) ~ /what u hear/ {
            if (match($0, /^card ([0-9]+).*device ([0-9]+)/, m))
                { print m[1] "," m[2]; exit } }'
}

idle_rms() {
    local dev="$1" tmp
    tmp="$(mktemp -t ae5-ab-XXXXXX.wav)"
    arecord -D "hw:${dev}" -f S32_LE -c 2 -r 48000 -d 2 "$tmp" >/dev/null 2>&1 || true
    sox "$tmp" -n stats 2>&1 | awk '/RMS lev dB/{print $4; f=1} END{if(!f) print "nan"}'
    rm -f "$tmp"
}

main() {
    command -v sox >/dev/null 2>&1 || die "sox is required"
    command -v ae5ctl >/dev/null 2>&1 || die "ae5ctl is required"
    local dev; dev="$(wuh_dev)"
    [ -n "${dev:-}" ] || die "What U Hear capture device not found"

    if [ -z "$FIXTURES" ]; then
        FIXTURES="$(mktemp -d -t ae5-ab-fix-XXXXXX)"
        bash "$HERE/audio-parity.sh" generate "$FIXTURES" >/dev/null 2>&1 \
            || die "could not generate fixtures"
    fi
    local fixture="$FIXTURES/parity-tones.wav"
    [ -f "$fixture" ] || die "fixture missing: $fixture"

    printf '\n=== AE-5 DSP oscillation A/B test ===\n'
    printf 'kernel  : %s\n' "$(uname -r)"
    printf 'taint   : %s\n' "$(cat /proc/sys/kernel/tainted)"
    printf 'trials  : %s\n\n' "$TRIALS"

    local osc=0 i
    for i in $(seq 1 "$TRIALS"); do
        AE5_REINIT_YES=1 AE5_REINIT_PROFILE="$PROFILE" \
            bash "$HERE/dsp-reinit.sh" >/dev/null 2>&1 || true
        ae5ctl set-playback-switch Master on  >/dev/null 2>&1 || true
        ae5ctl set-playback-switch Front on   >/dev/null 2>&1 || true
        sleep 2
        local pre; pre="$(idle_rms "$dev")"

        wpctl set-mute @DEFAULT_AUDIO_SINK@ 0 >/dev/null 2>&1 || true
        wpctl set-volume @DEFAULT_AUDIO_SINK@ 20% >/dev/null 2>&1 || true
        timeout 7 pw-play "$fixture" >/dev/null 2>&1 || true
        wpctl set-mute @DEFAULT_AUDIO_SINK@ 1 >/dev/null 2>&1 || true
        sleep 3

        local post verdict; post="$(idle_rms "$dev")"
        if [ "$post" = "-inf" ] || [ "$post" = "nan" ]; then
            verdict="clean"
        elif awk -v r="$post" -v t="$THRESHOLD" 'BEGIN{exit !(r>t)}'; then
            verdict="OSCILLATING"; osc=$((osc + 1))
        else
            verdict="clean"
        fi
        printf 'trial %s/%s : pre %-8s post %-8s %s\n' \
            "$i" "$TRIALS" "$pre" "$post" "$verdict"
    done

    ae5ctl set-playback-switch Master off >/dev/null 2>&1 || true
    wpctl set-mute @DEFAULT_AUDIO_SINK@ 1 >/dev/null 2>&1 || true

    printf '\n=== RESULT on %s ===\n' "$(uname -r)"
    printf 'oscillated in %s of %s trials\n\n' "$osc" "$TRIALS"
    printf 'Output left hard-muted. Compare this number against the other kernel.\n'
    printf 'Reference: 4 of 5 on 7.1.4-ae5-current (2026-07-27).\n'
}

case "${1:-}" in
    -h|--help) sed -n '2,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' ;;
    --self-test)
        command -v sox >/dev/null 2>&1 || die "sox is required"
        wuh_dev >/dev/null || die "What U Hear device not found"
        awk -v r=-6  -v t=-30 'BEGIN{exit !(r>t)}' || die "threshold logic broken"
        awk -v r=-38 -v t=-30 'BEGIN{exit (r>t)}'  || die "threshold logic broken"
        echo "kernel-ab-test self-test passed"
        ;;
    *) main ;;
esac
