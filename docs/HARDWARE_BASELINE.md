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
