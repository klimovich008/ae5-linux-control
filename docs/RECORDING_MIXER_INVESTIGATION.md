# AE-5 recording mixer investigation

This investigation separates three different recording layers on the audited
Creative `1102:0012/1102:0051` card:

- the CA0132 analog capture PCM and its source selector;
- the CA0132 digital `What U Hear` PCM;
- PipeWire ACP ports presented to the desktop.

The tests ran on Nobara kernel `7.1.4-200.nobara.fc44.x86_64`. Every guarded
write restored the complete starting profile. No kernel module was loaded or
replaced.

## What U Hear volume and mute do not affect the AE-5 stream

The driver exposes `What U Hear Capture Volume` and
`What U Hear Capture Switch` as standard HDA input-amplifier controls on node
`0x0a`. The codec reports a stereo amplifier with 100 steps from -90 through
+9 dB and a mute bit. Writes and reads are internally consistent:

| Requested state | Node `0x0a` amplifier value |
|---|---|
| level 90, enabled | `[0x5a 0x5a]` |
| level 0, enabled | `[0x00 0x00]` |
| level 0, muted | `[0x80 0x80]` |

The captured signal does not traverse that amplifier on the AE-5.

An initial four-second matrix at levels 90 and 86 and with the switch muted
produced the same `0.003900` whole-file RMS in all three captures. Output
effects were active, so a second test disabled them and counterbalanced the
control order.

For the second test:

- PipeWire had no playback or recording stream;
- `Front Playback Switch` was off, keeping the physical headphone jack silent;
- `Enable OutFX` was off;
- a generated 48 kHz, signed 32-bit stereo 997 Hz fixture at -30 dBFS was
  played through direct ALSA `hw:0,0`;
- direct ALSA `hw:0,2` recorded the physical `CA0132 What U Hear` PCM;
- one second from the steady part of the left channel was measured;
- the complete mixer profile was restored before analysis.

| Order | Control state | Peak | RMS | Rough frequency |
|---|---|---:|---:|---:|
| 1 | level 90, enabled | 0.031260 | 0.022104 | 996 Hz |
| 2 | level 0, enabled | 0.031260 | 0.022104 | 996 Hz |
| 3 | level 90, enabled | 0.031260 | 0.022104 | 996 Hz |
| 4 | muted | 0.031260 | 0.022104 | 996 Hz |
| 5 | level 90, enabled | 0.031260 | 0.022104 | 996 Hz |

The fixture SHA-256 was
`a02eb1f0e640cb4b8ae8ac494d17ed0dc87e6170a94c6d9f6f2848a81be9d84d`.
The captures have different file hashes because their leading stream-start
regions differ, but their steady signal is identical. The generated raw files
were discarded after deriving these measurements.

The exact upstream source still defines these as ordinary HDA controls in
`desktop_mixer`, while the DSP sends What U Hear to the fixed internal source
selected through module `0x31`. No public source identifies a separate,
verified DSP gain control for that loopback.

The candidate
[`ca0132-ae5-hide-ineffective-wuh-controls.patch`](../kernel/ca0132-ae5-hide-ineffective-wuh-controls.patch)
therefore removes only the two ineffective AE-5 controls. It retains the
`CA0132 What U Hear` PCM and leaves every other CA0132 quirk unchanged. This
follows the driver's existing policy of not exposing mixer elements that
cannot affect hardware:
[upstream precedent](https://github.com/torvalds/linux/commit/c41999a23929f30808bae6009d8065052d4d73fd).

AE-5 Control also treats these controls as read-only on unpatched kernels. It
explains the driver defect instead of accepting ineffective writes, omits the
values from newly captured profiles, and ignores them in legacy profiles. The
same profiles therefore work before and after the kernel candidate removes the
mixer elements.

## Exact PipeWire input routes

The generic ACP profile initially exposed only Microphone and Line In. Its
active desktop port was Line In while a later direct profile restore had set
the ALSA `Input Source` to Microphone. The generic front-microphone path did
not probe because the AE-5 has an `Input Source` enum rather than a separate
`Front Mic` mixer element.

The AE-5 profile now supplies three exact paths:

- `sound-blaster-ae5-input-microphone`;
- `sound-blaster-ae5-input-front-microphone`;
- `sound-blaster-ae5-input-line-in`.

After a WirePlumber restart, all three appeared with the intended names. A
guarded no-stream matrix proved that selecting each PipeWire port writes the
matching ALSA enum:

| PipeWire port | ALSA `Input Source` |
|---|---|
| Microphone | `Microphone` |
| Front Microphone | `Front Microphone` |
| Line In | `Line In` |

The profile was restored to Microphone. The active fixed headphone route,
Front DAC, 43% sink volume, and mute state were unchanged.

AE-5 Control now selects these ports through WirePlumber rather than writing
the ALSA enum behind the session manager. Rebuilt CLI and native-profile
matrices synchronized every port with its enum and restored both the complete
mixer and retained route-state hashes.

## Remaining recording gates

- Connect a controlled signal to rear microphone, front microphone, and line
  input and verify each physical path.
- Measure analog `Capture` level, mute, and microphone boost for gain, noise
  floor, and clipping. Do not infer their behavior from the digital loopback.
- Repeat the What U Hear fixture under the patched kernel. Its first physical
  boot already confirmed that the ineffective mixer control disappears while
  the PCM and every analog capture control remain, with clean DSP
  initialization; playback, routing, suspend/resume, and profile compatibility
  still require patched-kernel exercise.
- Add an explicit software volume/mute substitute only at the recording-stream
  layer; do not relabel it as hardware gain.
