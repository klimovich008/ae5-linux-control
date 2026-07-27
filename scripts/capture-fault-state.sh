#!/usr/bin/env bash
# Capture everything that could distinguish a faulty audio state from a good
# one, in one shot, fast enough to run while the fault is audible.
#
# Every investigation in this project so far has measured the aftermath: by
# the time the state was captured the fault had cleared, or a recovery step
# had already reset it. The distinguishing evidence lives in the live fault,
# and it spans layers no single tool covers — ALSA controls, the PCM's
# negotiated parameters, PipeWire's graph and quantum, the card's internal
# tap, and the analog output.
#
# Usage:  capture-fault-state.sh <label>
# Then:   capture-fault-state.sh --diff <labelA> <labelB>
set -euo pipefail

ROOT="${AE5_FAULT_ROOT:-$HOME/.local/state/ae5-fault-captures}"
SECONDS_AUDIO="${AE5_FAULT_SECONDS:-4}"

die() { printf 'error: %s\n' "$*" >&2; exit 1; }

find_dev() {
    arecord -l 2>/dev/null | awk -v hint="$1" '
        tolower($0) ~ tolower(hint) {
            if (match($0, /^card ([0-9]+).*device ([0-9]+)/, m))
                { print m[1] "," m[2]; exit } }'
}

capture() {
    local label="$1"
    local dir="$ROOT/$label"
    mkdir -p "$dir"

    # Audio first: it is the only part that cannot be re-derived later.
    local wuh fif
    wuh="$(find_dev 'what u hear')" || true
    fif="$(find_dev fifine)" || true
    [ -n "${wuh:-}" ] && arecord -D "hw:${wuh}" -f S32_LE -c 2 -r 48000 \
        -d "$SECONDS_AUDIO" "$dir/tap.wav" >/dev/null 2>&1 &
    [ -n "${fif:-}" ] && arecord -D "hw:${fif}" -f S16_LE -c 2 -r 48000 \
        -d "$SECONDS_AUDIO" "$dir/mic.wav" >/dev/null 2>&1 &

    # Everything else while the audio records.
    amixer -c 0 contents > "$dir/mixer.txt" 2>&1 || true
    for f in /proc/asound/card0/pcm*p/sub*/hw_params \
             /proc/asound/card0/pcm*p/sub*/sw_params \
             /proc/asound/card0/pcm*p/sub*/status; do
        [ -f "$f" ] && printf '== %s\n%s\n\n' "$f" "$(cat "$f")" >> "$dir/pcm.txt"
    done
    command -v pw-dump >/dev/null 2>&1 && pw-dump > "$dir/pw-dump.json" 2>/dev/null || true
    command -v wpctl   >/dev/null 2>&1 && wpctl status > "$dir/wpctl.txt" 2>&1 || true
    command -v pactl   >/dev/null 2>&1 && pactl list sinks > "$dir/sinks.txt" 2>&1 || true
    timeout 4 pw-top -b -n 2 > "$dir/pw-top.txt" 2>&1 || true
    journalctl -k -n 120 --no-pager > "$dir/kernel.txt" 2>&1 || true
    journalctl --user -n 120 --no-pager > "$dir/user.txt" 2>&1 || true
    { uptime; echo; cat /proc/asound/card0/pcm0p/sub0/hw_params 2>/dev/null; } \
        > "$dir/system.txt" 2>&1 || true
    wait 2>/dev/null || true

    printf 'captured "%s" -> %s\n' "$label" "$dir"
    for w in tap mic; do
        [ -f "$dir/$w.wav" ] && printf '  %-4s %s\n' "$w" \
            "$(sox "$dir/$w.wav" -n stats 2>&1 | grep -E 'Pk lev dB|RMS lev dB' | tr '\n' ' ')"
    done
}

diff_states() {
    local a="$ROOT/$1" b="$ROOT/$2"
    [ -d "$a" ] && [ -d "$b" ] || die "need two captured labels"
    python3 - "$a" "$b" "$1" "$2" <<'PY'
import re, sys, os

a, b, na, nb = sys.argv[1:5]

def controls(path):
    out, name, pending = {}, None, False
    try:
        lines = open(os.path.join(path, 'mixer.txt'), errors='ignore')
    except OSError:
        return out
    for raw in lines:
        s = raw.rstrip('\n')
        m = re.match(r"numid=\d+,iface=\w+,name='(.+)'\s*$", s)
        if m:
            name, pending = m.group(1), True
            continue
        if pending and name and re.match(r"\s+: values=", s):
            out[name] = s.split(': values=', 1)[1].strip()
            pending = False
    return out

ca, cb = controls(a), controls(b)
diff = [(k, ca.get(k), cb.get(k)) for k in sorted(set(ca) | set(cb)) if ca.get(k) != cb.get(k)]
print(f'ALSA controls: {len(diff)} of {len(set(ca) | set(cb))} differ')
for k, x, y in diff:
    print(f'  {k}\n      {na}: {x}\n      {nb}: {y}')

def block(path, fname):
    try:
        return open(os.path.join(path, fname), errors='ignore').read()
    except OSError:
        return ''

for fname, label in (('pcm.txt', 'PCM parameters'), ('system.txt', 'system')):
    xa = [l for l in block(a, fname).splitlines() if l.strip()]
    xb = [l for l in block(b, fname).splitlines() if l.strip()]
    d = [(x, y) for x, y in zip(xa, xb) if x != y]
    print(f'\n{label}: {len(d)} differing lines')
    for x, y in d[:20]:
        print(f'      {na}: {x}\n      {nb}: {y}')
PY
}

case "${1:-}" in
    --diff) shift; diff_states "$@" ;;
    --self-test)
        command -v sox >/dev/null 2>&1 || die "sox is required"
        command -v arecord >/dev/null 2>&1 || die "arecord is required"
        find_dev 'what u hear' >/dev/null || true
        echo "capture-fault-state self-test passed"
        ;;
    '') die "usage: capture-fault-state.sh <label> | --diff <a> <b>" ;;
    *) capture "$1" ;;
esac
