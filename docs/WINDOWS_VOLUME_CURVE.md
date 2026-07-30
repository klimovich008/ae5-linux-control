# Windows-equivalent AE-5 volume curve

Status on 2026-07-29: **the installed Windows formula is recovered, tested,
and active only on the original AE-5 analog PipeWire node**. The first
headphone A/B acceptance remains pending.

## Recovered Windows implementation

Sound Blaster Command divides its displayed master-volume value by 100 and
passes the result unchanged to
`IAudioEndpointVolume::SetMasterVolumeLevelScalar`. The remaining taper is in
Windows Audio, not in the Creative UI.

The matching Microsoft symbols for the installed Windows `audiosrv.dll`
identify `CVolumeUnit::SetWiper`, `TaperFromScalar`, `ScalarFromTaper`,
`ConvertScalarToDb`, and `GetWiper`. Disassembly proves these constants:

- default endpoint range: `-96..0 dB`;
- taper exponent: `1.75`;
- base: `10`;
- decibel multiplier: `20`; and
- default step: `1.5 dB`.

The analyzed `audiosrv.dll` SHA-256 is
`92cc5b7b85ce9870f0f94c6a5a7bba535539d08059c55fcfee3a4d61711c3ae4`.
Its matching PDB GUID is
`30772B45-0F0A-D93D-5C22-47C0A84574EE`.

Creative's installed `ctxhda.inf` independently agrees with the result:
its commented 50% default is `-10.50 dB`, and its commented 20% headphone
default is `-24.30 dB`, which are the formula's quantized values.

No Microsoft or Creative binary is distributed by this repository.

## Formula

For displayed fraction `p` in `0..1`:

```text
minimum_taper = 10^(-96 / 35)
taper         = minimum_taper + p × (1 - minimum_taper)
dB            = 35 × log10(taper)
sample_gain   = taper^1.75
```

Zero remains digital silence instead of the formula's `-96 dB` floor.

PipeWire and Pulse-compatible clients store a displayed fraction as `p³`.
The patch leaves that public value unchanged, takes its cube root immediately
before channel mixing, and applies the recovered Windows taper. Plasma,
GNOME, `wpctl`, and applications therefore continue to display the requested
percentage.

| Display | Normal PipeWire | Patched AE-5 |
|---:|---:|---:|
| 0% | silence | silence |
| 5% | -78.06 dB | -45.02 dB |
| 20% | -41.94 dB | -24.35 dB |
| 30% | -31.37 dB | -18.24 dB |
| 43% | -21.99 dB | -12.79 dB |
| 50% | -18.06 dB | -10.51 dB |
| 100% | 0 dB | 0 dB |

The Rust implementation and its reference-point tests are in
[`volume_curve.rs`](../src/volume_curve.rs). The processing patch and its C
test are in
[`ae5-windows-volume-curve.patch`](../pipewire/ae5-windows-volume-curve.patch).

## AE-5-only boundary

The patched SPA plugin defaults to PipeWire's original cubic behavior. It
changes behavior only when a node explicitly has:

```text
channelmix.volume-curve = "windows-audio-taper"
```

The WirePlumber rule sets that property only when all of these match:

- an ALSA output node;
- an analog device profile; and
- codec components beginning `HDA:11020011,11020051`.

`11020051` is the exact original AE-5 subsystem identity supported by this
project. The motherboard, Fifine USB device, HDMI outputs, inputs, and other
cards remain cubic.

The plugin is an overlay, not a replacement Fedora package. PipeWire searches
the AE5 Control directory first and then the normal system SPA directory.
Removing the service drop-in and overlay restores the stock implementation.

## Build, install, and rollback

The build script downloads the source RPM matching the installed PipeWire,
applies the patch, builds only the required plugin and tests, and refuses to
overwrite an existing result:

```sh
scripts/build-pipewire-volume-plugin.sh
scripts/install-pipewire-volume-plugin.sh \
  dist/pipewire-1.6.8-ae5/libspa-audioconvert.so
```

Restart PipeWire and WirePlumber only while audio is idle:

```sh
systemctl --user restart pipewire.service wireplumber.service
```

Rollback is explicit:

```sh
scripts/install-pipewire-volume-plugin.sh --uninstall
systemctl --user restart pipewire.service wireplumber.service
```

After installation, verify the exact AE-5 output reports
`windows-audio-taper` while every other sink reports `cubic`:

```sh
wpctl inspect @DEFAULT_AUDIO_SINK@
pw-cli enum-params @DEFAULT_AUDIO_SINK@ Props
```

The current host passed the formula test, the unchanged channel-mixer and
audioconvert suites, a clean source-RPM rebuild, plugin-load verification,
and the exact-node scope check. The AE-5 is left at 30% and muted.

## Remaining acceptance

The binary formula removes the need for a 101-point Windows capture as an
implementation prerequisite. The silent collector remains useful as an
independent check of a particular Windows endpoint range and driver revision.

Formula correctness does not by itself prove equal perceived loudness,
analog noise, or headphone gain. The first physical comparison must start
muted, use Low headphone gain, keep the user-facing value at or below 20%,
and compare matched Windows and Linux settings. Hardware OutFX remains
blocked; it is not part of this volume patch.
