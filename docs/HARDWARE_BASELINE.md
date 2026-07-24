# Linux hardware baseline

Collected on 2026-07-24 from the target machine without root access.

## Hardware identity

- PCI controller: Creative CA0132 `1102:0012`
- PCI subsystem: Creative `1102:0051`
- HDA codec: Creative Sound BlasterX AE-5 `1102:0011`
- Codec subsystem: `1102:0051`
- ALSA card at collection time: `HDA Creative` (card indexes are not stable)

## Linux audio stack

- Distribution: Nobara Linux 44
- Kernel: `7.1.4-200.nobara.fc44.x86_64`
- PipeWire: 1.6.8
- Kernel modules: `snd_hda_intel` and in-tree `snd_hda_codec_ca0132`
- Driver firmware status: CA0132 DSP downloaded and running

The driver exposes analog and digital playback, analog and What U Hear capture,
speaker/headphone and input selection, headphone gain, DAC filter selection,
speaker configuration, bass redirection, ten EQ bands, output effects, and
microphone effects.

At collection time, `Output Select` was `Headphone` and
`HP/Speaker Auto Detect` was off. No mixer values were changed during the
audit.

The audit also found a separate upstream CA0132 control bug: raw ALSA reports
`Wedge Angle Capture Volume` as `10` even though the control declares a
`20..180` range. The driver initializes the DSP to 30 degrees but caches the
lookup-table index instead of the public value. The independently reviewable
one-line fix and validation procedure are in
[`kernel/README.md`](../kernel/README.md).

The full generated report is intentionally ignored by Git because diagnostics
can contain local machine details.

## Reversible channel-balance acceptance

Both typed per-channel ALSA paths were tested on the physical AE-5. For
playback, the `Front` switch was off:

1. Readback reported `Front Left=90, Front Right=90`.
2. `set-playback-channel-level Front "Front Right" 89` read back
   `Front Left=90, Front Right=89`.
3. Restoring the right channel to 90 produced the exact original complete
   control snapshot.

An exit trap guaranteed restoration if an intermediate command failed. The
left channel never changed, the control remained playback-off throughout, and
the kernel journal recorded no warning or error during the transaction.

The same transaction passed for capture: `What U Hear` changed from
`Front Left=90, Front Right=90` to `Front Left=90, Front Right=89` and back to
the exact original snapshot. Its capture switch remained on, and the kernel
journal again recorded no warning or error.

## Native profile round trip

A named profile captured 47 valid controls from the physical card; the invalid
running Wedge Angle value was excluded as designed. Separate `ae5ctl`
processes then:

1. found the profile in the standard per-user library;
2. loaded and validated every control against the live AE-5;
3. applied all 47 controls with hardware readback verification.

The profile's SHA-256 and the complete `controls` output were identical before
and after apply. The kernel journal recorded no warning or error. The test
profile was then moved to recoverable desktop Trash, and a fresh library scan
again reported no saved profiles.

## What U Hear and Linux digital-path acceptance

The physical `CA0132 What U Hear` PCM was recorded at 48 kHz, 32-bit stereo
while the same quiet channel-identity fixture was played first through direct
ALSA and then through the normal PipeWire AE-5 sink. A 997 Hz signal sent only
to the left channel and a 1503 Hz signal sent only to the right channel were
each more than 42 dB above the opposite-band leakage in the recorded channel.

The complete mixer snapshot had SHA-256
`02530d87f6ce78e00f213bfa25f53174e8bfea1778f94b83bc0c9d32278c89f6`
before and after every probe. The source fixture and captures were not
normalized or resampled after recording. The generated reference set, raw
captures, and measurement notebook are retained privately outside the source
repository for the later Windows comparison.

The full parity-tone fixture produced these results:

- with the saved output-effects profile enabled, direct ALSA and PipeWire
  differed by `-0.01 dB` at 1 kHz and by at most `0.20 dB` in relative
  response;
- disabling only `Enable OutFX` made every measured band from 31 Hz through
  16 kHz exactly flat relative to 1 kHz;
- with `Enable OutFX` disabled, direct ALSA and PipeWire matched by `0.00 dB`
  at 1 kHz and `0.00 dB` maximum relative-response delta;
- enabling the saved effects profile changed the 1 kHz level by `7.70 dB` and
  relative response by as much as `9.05 dB`, consistent with its enabled
  Crystalizer, Smart Volume, and X-Bass controls.

`Master` was temporarily reduced from 76 to 20 during the longer playback
fixtures and restored by an exit guard. `Enable OutFX` was likewise restored
to on. The exact complete mixer snapshot was recovered after each test, and
the kernel journal contained no CA0132, HDA, ALSA, or DSP warning or error.

This proves the target card's What U Hear channel identity, master output
processing switch, and neutral 48 kHz direct-ALSA/PipeWire digital equivalence.
It does not compare Windows, measure the analog output, or cover 44.1 and
96 kHz.

## Isolated output effects

Separate physical What U Hear captures verified repeatable DSP changes from
Surround, Crystalizer, Dialog Plus, Smart Volume, and X-Bass while every other
output effect was disabled. Static effects repeated within 0.01 dB; the
stateful Smart Volume sequence repeated within 0.42 dB. Equalizer Flat matched
neutral by 0.00 dB.

All ten EQ controls targeted the expected fixture bands. A +12 dB request
measured within 0.86 dB on Bands 1 through 8, while the 31 Hz and 16 kHz edge
bands fell short by 1.90 and 1.80 dB. The full method, result tables, safety
checks, and limits are in
[`DSP_EFFECT_MEASUREMENT.md`](DSP_EFFECT_MEASUREMENT.md).
