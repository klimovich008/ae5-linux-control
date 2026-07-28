#!/usr/bin/env bash
# Measure harmonic distortion at the analog output.
#
# Music is non-stationary, so comparing two music captures cannot separate a
# real fault from the track simply moving on — and comparing the internal tap
# against a microphone cannot separate distortion from the microphone's own
# response. A steady tone removes both problems: whatever appears at 2f, 3f,
# 4f that was not sent is distortion, and the same measure applied to the
# card's internal tap says whether it was added before or after the DAC.
#
# Needs the headphones next to the microphone and nothing else playing.
set -euo pipefail

TONE_HZ="${AE5_THD_TONE:-1000}"
SECONDS_EACH="${AE5_THD_SECONDS:-6}"
LEVEL_DBFS="${AE5_THD_LEVEL:--18}"
WORK="$(mktemp -d -t ae5-thd-XXXXXX)"

die() { printf 'error: %s\n' "$*" >&2; exit 1; }

find_dev() {
    arecord -l 2>/dev/null | awk -v hint="$1" '
        tolower($0) ~ tolower(hint) {
            if (match($0, /^card ([0-9]+).*device ([0-9]+)/, m))
                { print m[1] "," m[2]; exit } }'
}

command -v sox >/dev/null 2>&1 || die "sox is required"
command -v pw-play >/dev/null 2>&1 || die "pw-play is required"
python3 -c 'import numpy' 2>/dev/null || die "python3 numpy is required"

FIFINE="$(find_dev fifine)" || true
WUH="$(find_dev 'what u hear')" || true
[ -n "${FIFINE:-}" ] || die "Fifine microphone not found"

# A tone well below full scale: distortion products stay visible without the
# level itself being the cause of them.
sox -n -r 48000 -c 2 -b 16 "$WORK/tone.wav" synth "$SECONDS_EACH" sine "$TONE_HZ" \
    vol "$(python3 -c "print(10**($LEVEL_DBFS/20))")"

printf 'playing %s Hz at %s dBFS for %ss\n' "$TONE_HZ" "$LEVEL_DBFS" "$SECONDS_EACH"
arecord -D "hw:${FIFINE}" -f S16_LE -c 2 -r 48000 -d "$SECONDS_EACH" "$WORK/mic.wav" >/dev/null 2>&1 &
MIC=$!
if [ -n "${WUH:-}" ]; then
    arecord -D "hw:${WUH}" -f S32_LE -c 2 -r 48000 -d "$SECONDS_EACH" "$WORK/tap.wav" >/dev/null 2>&1 &
    TAP=$!
fi
sleep 0.3
timeout "$((SECONDS_EACH + 2))" pw-play "$WORK/tone.wav" >/dev/null 2>&1 || true
wait "$MIC" 2>/dev/null || true
[ -n "${TAP:-}" ] && { wait "$TAP" 2>/dev/null || true; }

python3 - "$WORK" "$TONE_HZ" <<'PY'
import sys, os, wave
import numpy as np

work, tone = sys.argv[1], float(sys.argv[2])

def load(path):
    with wave.open(path, 'rb') as w:
        sr, width, ch = w.getframerate(), w.getsampwidth(), w.getnchannels()
        raw = w.readframes(w.getnframes())
    dtype = '<i2' if width == 2 else '<i4'
    x = np.frombuffer(raw, dtype=dtype).reshape(-1, ch)[:, 0]
    return x.astype(np.float64) / float(2 ** (8 * width - 1)), sr

def thd(path, label):
    if not os.path.exists(path):
        return
    x, sr = load(path)
    n = 1 << 15
    if len(x) < n:
        print(f'  {label}: capture too short')
        return
    seg = x[len(x) // 2 - n // 2: len(x) // 2 + n // 2] * np.hanning(n)
    sp = np.abs(np.fft.rfft(seg))
    fr = np.fft.rfftfreq(n, 1 / sr)

    def peak(target, width=25.0):
        m = (fr > target - width) & (fr < target + width)
        return sp[m].max() if m.any() else 0.0

    f0 = peak(tone)
    if f0 <= 0:
        print(f'  {label}: fundamental not found — is anything reaching the output?')
        return
    harmonics = [peak(tone * k) for k in range(2, 7) if tone * k < sr / 2]
    ratio = float(np.sqrt(sum(h * h for h in harmonics)) / f0)
    print(f'  {label:<22} THD {ratio * 100:6.2f}%  ({20 * np.log10(max(ratio, 1e-9)):+6.1f} dB)')
    for k, h in enumerate(harmonics, start=2):
        if h > 0:
            print(f'      {k}f = {tone * k:>7.0f} Hz  {20 * np.log10(h / f0):+7.1f} dB')

print('\nharmonic distortion relative to the fundamental:')
thd(os.path.join(work, 'tap.wav'), 'internal tap (pre-DAC)')
thd(os.path.join(work, 'mic.wav'), 'Fifine (post-analog)')
print('\nA tap that is clean while the microphone shows harmonics puts the')
print('distortion after the DAC, in the analog output stage.')
PY

rm -rf "$WORK"
