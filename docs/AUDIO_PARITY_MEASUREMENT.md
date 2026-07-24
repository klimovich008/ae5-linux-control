# Windows/Linux audio parity measurement

Subjective sound differences are not enough evidence for a kernel change. This
procedure measures the first three parity targets with the same source
material:

- output level within 0.5 dB at 1 kHz;
- relative frequency-response delta within 1 dB from 31 Hz to 16 kHz;
- noise-floor delta within 3 dB or the repeatability limit of the setup.

It does not yet measure distortion or prove the analog performance of the
AE-5. Those require a capture interface whose own limits are known.

## Required physical path

The preferred path is:

```text
AE-5 line/headphone output -> safe fixed attenuation -> separate line input
```

Use the same cable, capture interface, input gain, output connector, and volume
on Windows and Linux. Do not connect a powered speaker output to a microphone
input. Do not use the FiFine USB microphone on the current test system as a
line input.

The AE-5 `What U Hear` device can isolate the digital DSP path, and the
motherboard line input can provide an initial analog check. Neither replaces a
calibrated independent interface for final noise or distortion claims.

## Generate one shared reference set

SoX 14.4 or later is required for this development-only harness:

```sh
bash scripts/audio-parity.sh --self-test
bash scripts/audio-parity.sh generate /path/to/ae5-reference
```

The command creates 48 kHz, 24-bit stereo tone, sweep, level-step, and digital
silence files plus `SHA256SUMS`. It refuses to overwrite any existing fixture.
Copy these exact generated files to Windows and verify their hashes there; do
not generate a second set.

The tone and sweep fixtures begin with a loud 997 Hz synchronization marker.
The analyzer removes leading capture latency from that marker before measuring.

## Capture matrix

Start with all output DSP processing disabled or flat, a fixed safe output
level, and a fixed capture gain. Record at 48 kHz, 24-bit stereo:

1. Windows playback through the Creative driver.
2. Linux direct ALSA `hw:` playback.
3. Linux normal PipeWire playback.
4. Digital silence through the same three paths.

Do not change gain between captures. Record at least half a second before
starting playback and at least half a second after it ends. Preserve the
original WAV files and record the operating system, driver/kernel, selected
output, volume, gain, DSP state, DAC filter, playback API, capture device, and
cable in a text notebook beside them.

The exact playback and capture commands depend on the selected external
interface and are deliberately not automated yet. Opening the wrong ALSA
device or using an unsafe analog connection should remain an explicit human
decision.

## Analyze and compare

Inspect one tone capture:

```sh
bash scripts/audio-parity.sh analyze-tones windows-tones.wav
```

Compare Windows and Linux:

```sh
bash scripts/audio-parity.sh compare-tones \
  windows-tones.wav linux-alsa-tones.wav
```

The comparison reports:

- the absolute Linux-minus-Windows level delta at every band;
- each band's response delta after normalizing both captures to 1 kHz;
- `pass` only when the 1 kHz level and maximum response delta meet the targets.

Compare captures of `parity-silence.wav`:

```sh
bash scripts/audio-parity.sh compare-noise \
  windows-silence.wav linux-alsa-silence.wav
```

If the synchronization marker is recorded below -40 dBFS or the noise floor
triggers alignment too early, set a suitable SoX amplitude threshold:

```sh
AE5_SYNC_THRESHOLD=0.3% \
  bash scripts/audio-parity.sh analyze-tones quiet-capture.wav
```

Do not normalize, denoise, resample, or encode the captures before comparison.
If direct ALSA matches Windows but PipeWire does not, investigate PipeWire
format/rate policy before touching the kernel. If both Linux paths diverge in
the same way, compare DSP state and CA0132 initialization next.

## Target-card Linux digital baseline

On 2026-07-24, the generated tone fixture was played through both direct ALSA
and the normal PipeWire AE-5 sink while the physical card's `CA0132 What U
Hear` PCM recorded 48 kHz, 32-bit stereo.

With the saved effects profile active, PipeWire differed from direct ALSA by
`-0.01 dB` at 1 kHz and at most `0.20 dB` in relative response. With only
`Enable OutFX` disabled, every measured band was flat relative to 1 kHz and
the two Linux paths matched by `0.00 dB` in both reported metrics.

The active effects profile was itself measurable: enabling it changed the
1 kHz level by `7.70 dB` and relative response by as much as `9.05 dB`.
Separate 997 Hz-left/1503 Hz-right probes showed more than 42 dB separation
from the opposite band for both playback APIs. Every temporary control change
restored the exact complete mixer snapshot, and no audio-driver warning or
error appeared in the kernel journal.

This baseline isolates PipeWire from the current sound difference at 48 kHz:
the desktop path is equivalent to direct ALSA when measured through the card's
digital loopback. It does not establish Windows parity or analog output
performance. Exact target-system values and limits are recorded in
[`HARDWARE_BASELINE.md`](HARDWARE_BASELINE.md).

The same loopback path was subsequently used to isolate Surround, Crystalizer,
Dialog Plus, Smart Volume, X-Bass, Equalizer Flat, and all ten individual EQ
bands. Results, repeat deltas, and the two edge-band gain shortfalls are in
[`DSP_EFFECT_MEASUREMENT.md`](DSP_EFFECT_MEASUREMENT.md).
