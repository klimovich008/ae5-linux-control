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
