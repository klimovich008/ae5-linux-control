#!/usr/bin/env bash
# Acoustic review of the AE-5 analog output using the external Fifine
# microphone and, in parallel, the card's internal What U Hear tap.
#
# The pair is the useful part: What U Hear proves what reaches the DSP
# output digitally, the Fifine proves what actually leaves the analog
# headphone stage. A fault visible on one and not the other localises
# itself immediately.
#
# Safety: this script never raises the PipeWire sink and refuses to run
# above the project ceiling. Acoustic captures require the headphones to
# be OFF the user's head and placed next to the microphone.
set -euo pipefail

CEILING_PERCENT=20
FIFINE_HINT="fifine"
WUH_HINT="What U Hear"
DURATION="${AE5_REVIEW_SECONDS:-4}"
OUTDIR="${AE5_REVIEW_DIR:-}"
AE5CTL="${AE5CTL:-ae5ctl}"

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
note() { printf '%s\n' "$*"; }

usage() {
    cat <<'EOF'
usage:
  acoustic-review.sh baseline            capture with the output hard-muted
  acoustic-review.sh measure LABEL       capture in the current state
  acoustic-review.sh ab                  A/B the hardware Master switch
  acoustic-review.sh compare A.wav B.wav report level and band deltas
  acoustic-review.sh --self-test

Environment:
  AE5_REVIEW_SECONDS  capture length in seconds (default 4)
  AE5_REVIEW_DIR      output directory (default a fresh mktemp -d)
  AE5CTL              path to ae5ctl when not on PATH
EOF
}

find_card() {
    # $1: hint matched against `arecord -l` card description
    arecord -l 2>/dev/null \
        | awk -v hint="$1" 'tolower($0) ~ tolower(hint) {
              if (match($0, /^card ([0-9]+).*device ([0-9]+)/, m))
                  { print m[1] "," m[2]; exit }
          }'
}

require_safe_sink() {
    command -v wpctl >/dev/null 2>&1 || return 0
    local raw pct
    raw="$(wpctl get-volume @DEFAULT_AUDIO_SINK@ 2>/dev/null | awk '{print $2}')" || return 0
    [ -n "$raw" ] || return 0
    pct="$(awk -v v="$raw" 'BEGIN{printf "%d", v*100}')"
    [ "$pct" -le "$CEILING_PERCENT" ] \
        || die "PipeWire sink is ${pct}%; the project ceiling is ${CEILING_PERCENT}%"
}

capture() {
    # $1: output basename
    local base="$1" fif wuh
    fif="$(find_card "$FIFINE_HINT")" || true
    wuh="$(find_card "$WUH_HINT")" || true
    [ -n "${fif:-}" ] || die "Fifine microphone not found in 'arecord -l'"

    arecord -D "hw:${fif}" -f S16_LE -c 2 -r 48000 -d "$DURATION" \
        "${base}-fifine.wav" >/dev/null 2>&1 &
    local fpid=$!
    local wpid=
    if [ -n "${wuh:-}" ]; then
        arecord -D "hw:${wuh}" -f S32_LE -c 2 -r 48000 -d "$DURATION" \
            "${base}-wuh.wav" >/dev/null 2>&1 &
        wpid=$!
    fi
    wait "$fpid" || die "Fifine capture failed"
    [ -z "$wpid" ] || wait "$wpid" || true
}

level() {
    sox "$1" -n stats 2>&1 \
        | awk '/RMS lev dB/{r=$4} /Pk lev dB/{p=$4} END{printf "peak %s dB, RMS %s dB", p, r}'
}

report() {
    local base="$1" label="$2"
    printf '%-28s Fifine: %s\n' "$label" "$(level "${base}-fifine.wav")"
    [ -f "${base}-wuh.wav" ] \
        && printf '%-28s WUH   : %s\n' "" "$(level "${base}-wuh.wav")"
}

band_compare() {
    python3 - "$1" "$2" <<'PY'
import sys, wave, numpy as np

def load(path):
    with wave.open(path, 'rb') as w:
        sr = w.getframerate()
        width = w.getsampwidth()
        data = w.readframes(w.getnframes())
        ch = w.getnchannels()
    dtype = '<i2' if width == 2 else '<i4'
    x = np.frombuffer(data, dtype=dtype).reshape(-1, ch)[:, 0]
    return x.astype(np.float64) / float(2 ** (8 * width - 1)), sr

def psd(path):
    x, sr = load(path)
    n = 1 << 14
    if len(x) < n:
        raise SystemExit(f'capture too short: {path}')
    acc = np.zeros(n // 2 + 1)
    count = 0
    for start in range(0, len(x) - n, n):
        acc += np.abs(np.fft.rfft(x[start:start + n] * np.hanning(n))) ** 2
        count += 1
    return acc / max(count, 1), np.fft.rfftfreq(n, 1 / sr)

a, f = psd(sys.argv[1])
b, _ = psd(sys.argv[2])
delta = 10 * np.log10((b + 1e-30) / (a + 1e-30))
print('  band delta (second minus first):')
for lo, hi in ((20, 60), (60, 120), (120, 300), (300, 1000),
               (1000, 3000), (3000, 8000), (8000, 16000), (16000, 22000)):
    m = (f >= lo) & (f < hi)
    print(f'    {lo:6d}-{hi:6d} Hz : {delta[m].mean():+6.1f} dB')
top = sorted((i for i in np.argsort(delta)[::-1][:6] if f[i] >= 20),
             key=lambda i: -delta[i])
if top:
    print('  strongest lines:')
    for i in top:
        print(f'    {f[i]:8.1f} Hz : {delta[i]:+6.1f} dB')
PY
}

self_test() {
    command -v sox >/dev/null 2>&1 || die "sox is required"
    command -v arecord >/dev/null 2>&1 || die "arecord is required"
    command -v python3 >/dev/null 2>&1 || die "python3 is required"
    python3 -c 'import numpy' 2>/dev/null || die "python3 numpy is required"
    local tmp
    tmp="$(mktemp -d)"
    sox -n -r 48000 -c 2 -b 16 "$tmp/q.wav" synth 1 sine 1000 vol 0.01
    sox -n -r 48000 -c 2 -b 16 "$tmp/l.wav" synth 1 sine 1000 vol 0.10
    level "$tmp/q.wav" >/dev/null
    band_compare "$tmp/q.wav" "$tmp/l.wav" >/dev/null
    rm -rf "$tmp"
    note "acoustic-review self-test passed"
}

main() {
    local cmd="${1:-}"
    case "$cmd" in
        --self-test) self_test; return 0 ;;
        -h|--help|"") usage; return 0 ;;
    esac

    command -v sox >/dev/null 2>&1 || die "sox is required"
    [ -n "$OUTDIR" ] || OUTDIR="$(mktemp -d -t ae5-acoustic-XXXXXX)"
    mkdir -p "$OUTDIR"

    case "$cmd" in
        baseline)
            require_safe_sink
            "$AE5CTL" set-playback-switch Master off >/dev/null 2>&1 || true
            sleep 1
            capture "$OUTDIR/baseline"
            report "$OUTDIR/baseline" "baseline (Master off)"
            note "artifacts in $OUTDIR"
            ;;
        measure)
            local label="${2:-measure}"
            require_safe_sink
            capture "$OUTDIR/$label"
            report "$OUTDIR/$label" "$label"
            note "artifacts in $OUTDIR"
            ;;
        ab)
            require_safe_sink
            note "Headphones must be OFF your head and next to the microphone."
            "$AE5CTL" set-playback-switch Master off >/dev/null 2>&1 || true
            sleep 1; capture "$OUTDIR/a-off"
            report "$OUTDIR/a-off" "A  Master off"
            "$AE5CTL" set-playback-switch Master on >/dev/null 2>&1 || true
            sleep 1; capture "$OUTDIR/b-on"
            report "$OUTDIR/b-on" "B  Master on"
            "$AE5CTL" set-playback-switch Master off >/dev/null 2>&1 || true
            band_compare "$OUTDIR/a-off-fifine.wav" "$OUTDIR/b-on-fifine.wav"
            note "restored: Master off"
            note "artifacts in $OUTDIR"
            ;;
        compare)
            [ $# -eq 3 ] || die "compare needs two capture files"
            band_compare "$2" "$3"
            ;;
        *) usage; return 1 ;;
    esac
}

main "$@"
