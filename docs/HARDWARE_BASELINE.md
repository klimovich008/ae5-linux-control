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

## Native 44.1 and 96 kHz digital path

Later physical What U Hear captures closed the digital part of that rate gap.
With output processing disabled and PCM at its 0 dB value, direct ALSA and
PipeWire matched by `0.00 dB` at 1 kHz and by `0.00 dB` maximum normalized
response delta at both 44.1 and 96 kHz. Digital-silence captures were
byte-identical for each API at each rate.

PipeWire's live AE-5 sink reported the requested hardware rate during both
streams. A separate controlled run reproduced a uniform 0.8 dB offset when
the PCM mixer was set to its saved `-0.80 dB` value, then removed it by
returning PCM to 0 dB. This distinguishes normal desktop mixer gain from a
sample-rate or response defect.

Every temporary mixer and PipeWire change was restored exactly and the kernel
journal remained clean. The full method, hashes, tables, and remaining analog
and Windows limits are in
[`PIPEWIRE_RATE_PARITY.md`](PIPEWIRE_RATE_PARITY.md).

## Playback mixer write and mute matrix

A guarded hardware matrix exercised every generic playback mixer control
exposed by the app while no PipeWire stream was active. A fresh validated
47-control recovery profile was installed as an exit trap, and Master was
muted before the other controls changed.

- Master read back raw levels 0, 49, and 99 exactly.
- PCM read back 0, 128, and 255 on both channels exactly.
- Front and Surround read back 0, 49, and 99 on both channels exactly.
- Center and LFE read back 0, 49, and 99 exactly.
- Front, Surround, Center, LFE, IEC958, and IEC958 Default PCM each read back
  on and off exactly.

The retained physical What U Hear capture with Master muted contains zero
minimum, maximum, and RMS amplitude across 384,000 samples, while the
corresponding Master-on capture is non-zero. Separate native-rate captures
measured PCM raw 251 versus 255 at `-0.81 dB` at 44.1 kHz and `-0.80 dB` at
96 kHz, exactly matching the control's four 0.20 dB steps.

The recovery profile restored all 47 controls and the complete mixer hash
`7a61ac34dbca132e929806a1198a61f9334c5241bcb83e9da205152008ffea6e`.
No stream remained and no matching kernel warning appeared.

What U Hear is tapped before the analog output attenuators, so this evidence
does not claim that Master, Front, Surround, Center, or LFE produce their
advertised analog dB changes. That remaining gate requires a safely attenuated
physical output-to-line-input capture; IEC958 requires an optical receiver.

### External headphone level, mute, and gain

A later guarded Linux `6.18.40-ae5-lts-rgb+` cycle measured the physical
headphone output through the host's Fifine microphone. The headphones were not
worn. Direct ALSA played a two-second 997 Hz signed-32-bit stereo fixture at
`-18 dBFS`; output processing was off, Front was at its 0 dB value, and the
headphone gain started at Low. Each reported result is a Hann-windowed
tone-specific RMS measurement over the same 1.5-second capture interval.

| Master raw value | Reported dB | Mean 997 Hz RMS | Repeat spread |
|---:|---:|---:|---:|
| 55 | -44.00 dB | -91.85 dBFS | 0.24 dB |
| 60 | -39.00 dB | -86.64 dBFS | 0.18 dB |
| 65 | -34.00 dB | -82.30 dBFS | 0.58 dB |

The two five-step changes measured `+5.21 dB` and `+4.34 dB`, within `0.66 dB`
of the advertised `+5.00 dB`. At Master 65, muting Master reduced the tone to
`-105.96 dBFS`, within `0.88 dB` of the quiet baseline at `-106.84 dBFS`.
A separately repeated Front-muted negative control kept both Front switches
off before and after playback and reduced the tone by more than 34 dB.

At the more attenuated Master 55 setting, the three guarded gain choices were
also externally distinguishable:

| Headphone gain | Mean 997 Hz RMS | Delta from Low | Repeat spread |
|---|---:|---:|---:|
| Low | -91.85 dBFS | 0.00 dB | 0.24 dB |
| Medium | -90.57 dBFS | +1.28 dB | 0.14 dB |
| High | -84.81 dBFS | +7.04 dB | 0.93 dB |

An 18 kHz acoustic probe did not validate the DAC filters. The quiet baseline
was `-131.24 dBFS`; Slow and Fast averaged only 1.33 and 5.10 dB above it, and
the two Minimum Phase samples differed by 14.44 dB. That path is below a
useful signal-to-noise ratio for filter comparison. The filter gate therefore
requires an attenuated electrical capture or analyzer rather than a louder
near-ultrasonic headphone test.

The guest mixer returned to its exact known SHA-256, with one DSP
initialization, no open PCM, no failed unit, and no relevant kernel warning.
After shutdown, all host application controls and all three WirePlumber files
matched their saved state, the no-stream host mixer hash returned, the
AE-5/Fifine defaults returned, and the raw recordings were deleted after their
hashes and derived measurements were retained privately.

## Isolated output effects

Separate physical What U Hear captures verified repeatable DSP changes from
Surround, Crystalizer, Dialog Plus, Smart Volume, and X-Bass while every other
output effect was disabled. Static effects repeated within 0.01 dB; the
stateful Smart Volume sequence repeated within 0.42 dB. Equalizer Flat matched
neutral by 0.00 dB.

A later counterbalanced Smart Volume mode matrix established that Loud and
Night use the driver's fixed DSP values rather than the exposed level slider.
After a complete global OutFX reset, duplicate neutral references matched
exactly. Loud raised 1 kHz by 13.59 and 13.99 dB; Normal raised it by 7.40 and
5.72 dB. Night changed the first response by up to 5.00 dB but matched neutral
on the repeat, so Night determinism remains open. The GUI now disables only
the ineffective level slider in Loud/Night while preserving the switch, mode,
and profile value.

All ten EQ controls targeted the expected fixture bands. A +12 dB request
measured within 0.86 dB on Bands 1 through 8, while the 31 Hz and 16 kHz edge
bands fell short by 1.90 and 1.80 dB. The full method, result tables, safety
checks, and limits are in
[`DSP_EFFECT_MEASUREMENT.md`](DSP_EFFECT_MEASUREMENT.md).

All ten factory EQ presets were also captured independently. Every non-Flat
choice produced a distinct DSP response, with maximum normalized response
changes from 2.23 to 7.52 dB versus Flat. Left and right measurements matched,
the exact complete mixer hash was restored after every capture, and the kernel
journal remained clean.
