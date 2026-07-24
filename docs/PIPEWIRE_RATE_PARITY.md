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
