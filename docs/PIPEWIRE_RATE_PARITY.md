# PipeWire sample-rate parity

This investigation separates AE-5 hardware capability from desktop audio-graph
policy. It does not claim that a sample-rate change alone reproduces Windows
sound or improves every recording.

## Target evidence

Collected on 2026-07-24 with Linux `7.1.4-200.nobara.fc44.x86_64` and PipeWire
`1.6.8`.

The analog AE-5 PCM at `hw:1,0` reported:

- interleaved and memory-mapped access;
- `S16_LE` and `S32_LE` formats;
- two through six channels;
- rates from 16 through 96 kHz.

One-second silent direct-ALSA streams succeeded at 44.1, 48, and 96 kHz. A
96 kHz `S32_LE` stream also succeeded:

```sh
aplay -D hw:1,0 -q -d 1 -f S16_LE -c 2 -r 44100 /dev/zero
aplay -D hw:1,0 -q -d 1 -f S16_LE -c 2 -r 48000 /dev/zero
aplay -D hw:1,0 -q -d 1 -f S16_LE -c 2 -r 96000 /dev/zero
aplay -D hw:1,0 -q -d 1 -f S32_LE -c 2 -r 96000 /dev/zero
```

The initial PipeWire metadata allowed only 48 kHz. A 96 kHz `S32_LE` PipeWire
stream therefore left the AE-5 node at `S32LE/48000`, proving that the desktop
path resampled that stream.

Temporarily allowing 44.1, 48, and 96 kHz made the same node negotiate
`S32LE/44100` and `S32LE/96000` for matching silent streams. The temporary
metadata was then restored to its original `[ 48000 ]` value.

## Physical native-rate parity

The earlier checks proved format negotiation but did not compare audio at the
two alternative rates. A later guarded test generated separate 44.1 and
96 kHz versions of the project's tone and digital-silence fixtures, played
them through direct ALSA and PipeWire, and recorded the physical `CA0132 What
U Hear` PCM at the matching rate in `S32_LE`.

The card remained on Headphone, low gain, Slow Roll Off, and 2.0 channels.
`Master` was temporarily set to 0 so the analog output was silent,
`Enable OutFX` was off, and `PCM` was set to its 0 dB value of 255. PipeWire's
allowed-rate metadata temporarily contained `[ 44100 48000 96000 ]`. Its live
AE-5 sink reported `S32LE/44100` and `S32LE/96000` during the corresponding
streams.

| Rate | Direct 1 kHz | PipeWire 1 kHz | Level delta | Maximum response delta | Digital silence |
|---:|---:|---:|---:|---:|---:|
| 44.1 kHz | -21.32 dBFS | -21.32 dBFS | +0.00 dB | 0.00 dB | both `-inf` RMS |
| 96 kHz | -21.11 dBFS | -21.11 dBFS | +0.00 dB | 0.00 dB | both `-inf` RMS |

The response comparison covers the ten fixture frequencies from 31 Hz through
16 kHz. The direct and PipeWire silence WAV files were byte-identical at each
rate:

| Rate | Shared direct/PipeWire silence SHA-256 |
|---:|---|
| 44.1 kHz | `60367f62f9b57e5be734926a15991380b4fdd2f7d39d3553392aa53b7aef40bb` |
| 96 kHz | `b7e45dd6f47fdb0be4e8a6895179362e1eaaf507331e0fa3bb2faf172335616c` |

### Desktop mixer level diagnosis

The first PipeWire captures used the pre-test `PCM` value of 251, whose ALSA
dB metadata is `-0.80 dB`. They preserved the direct-ALSA response within
0.01 dB but measured `-0.81 dB` at 44.1 kHz and `-0.80 dB` at 96 kHz. This
matched the control exactly: the PipeWire sink's `-23.80 dB` gain was the sum
of `Master` at `-23.00 dB` and `PCM` at `-0.80 dB`.

Repeating only the PipeWire captures with `PCM` at 255 removed the complete
offset at both rates. The discrepancy was therefore normal desktop mixer
gain, not rate conversion or a CA0132 frequency-response defect. AE-5 Control
does not force PCM to 0 dB because doing so would change the user's volume.

The complete 48-control mixer snapshot had SHA-256
`7a61ac34dbca132e929806a1198a61f9334c5241bcb83e9da205152008ffea6e`
before and after every sequence. The original 47-control recovery profile had
SHA-256
`7039ee6c0d71eddb82c5d99c61eacd42a655563364037e9e1158fab26eb1d1c6`.
Allowed-rate metadata returned to `[ 48000 ]`, native-rate switching remained
disabled, and the kernel journal contained no matching CA0132, HDA, ALSA,
codec, or DSP warning.

This proves the target's neutral digital path at 44.1 and 96 kHz. It does not
measure the analog DAC/output stage, distortion, DAC-filter response, or
Windows parity.

## Optional persistent configuration

AE-5 Control can create this user-owned PipeWire fragment:

```text
~/.config/pipewire/pipewire.conf.d/91-ae5-control-rates.conf
```

It sets only:

```text
default.clock.allowed-rates = [ 44100 48000 96000 ]
```

Enable, inspect, or remove the managed fragment with:

```sh
ae5ctl native-rates-enable
ae5ctl native-rates-status
ae5ctl native-rates-disable
```

The change takes effect after PipeWire restarts or the next login. The program
never restarts the audio session automatically. It creates a missing fragment,
recognizes its exact own content, and refuses to overwrite or remove different
content at the same path.

PipeWire documents that alternative rates are selected while devices are idle
and that this is not enabled by default because some kernels and Bluetooth
devices can have problems. The setting is global: when simultaneous streams
request different rates, some stream still has to be resampled. These are the
reasons the application presents it as an explicit experimental option.

The relevant primary documentation is:

- [PipeWire daemon configuration](https://pipewire.pages.freedesktop.org/pipewire/page_man_pipewire_conf_5.html)
- [WirePlumber ALSA rules and audio formats](https://pipewire.pages.freedesktop.org/wireplumber/daemon/configuration/alsa.html)
- [PipeWire audio properties](https://pipewire.pages.freedesktop.org/pipewire/devel/page_man_pipewire-props_7.html)

## Verification after enabling

Confirm that PipeWire merged the fragment:

```sh
pw-config -n pipewire.conf merge context.properties
```

Start a raw silent stream in one terminal:

```sh
pw-cat --playback --raw --target <AE5_NODE> \
  --rate 96000 --channels 2 --format s32 /dev/zero
```

While it runs, inspect the AE-5 node in another terminal:

```sh
pw-cli enum-params <AE5_NODE> Format
```

The active format should report `S32LE` and `96000`. Repeat with `44100`, then
stop the silent stream. If Bluetooth or another device regresses, disable the
managed fragment and restart PipeWire.

Frequency response, noise, distortion, effects, and output-level matching still
require the loopback procedure in
[`AUDIO_PARITY_MEASUREMENT.md`](AUDIO_PARITY_MEASUREMENT.md).
