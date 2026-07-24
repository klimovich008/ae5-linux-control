# CA0132 cold-boot routing investigation

This investigation concerns the report that AE-5 headphone output sometimes
starts working only after selecting Line Out in the desktop and then selecting
Headphone in ALSA. The symptom is reproduced, and the failing layer is the
generic PipeWire ALSA Card Profile (ACP) headphone mixer path rather than the
CA0132 output-selection code.

## Current finding

The target system runs Nobara kernel `7.1.4-200.nobara.fc44.x86_64`. Linux 7.1
contains both recent upstream CA0132 routing fixes:

- [`778031e1658d`](https://github.com/torvalds/linux/commit/778031e1658d206a52bf9491c91ae5d4f4a2509d)
  initializes HP/Speaker auto-detect from the headphone pin's presence-detect
  capability.
- [`6fd9f6e870ea`](https://github.com/torvalds/linux/commit/6fd9f6e870ea285f05102e8e00e6a7f4495a9a02)
  makes a manual output selection disable auto-detect and apply the selected
  route even when the enum value was already selected.

The current upstream implementation is in
[`sound/hda/codecs/ca0132.c`](https://github.com/torvalds/linux/blob/master/sound/hda/codecs/ca0132.c).
The matching Nobara source RPM was verified, and its CA0132 source is
byte-identical to Linux stable `v7.1.4`; none of the packaged downstream
patches modifies CA0132. Exact hashes, revisions, licences, and the proprietary
analysis boundary are recorded in
[`SOURCE_INVENTORY.md`](SOURCE_INVENTORY.md).

On the failing boot:

- ALSA `Output Select` was `Headphone`.
- `HP/Speaker Auto Detect` was off.
- The read-only kernel `Headphone Jack` control was on.
- PipeWire selected its generic
  `analog-output-headphones;output-headphones` route.
- WirePlumber's ACP device had automatic profile and port selection disabled,
  so it restores an explicit route.
- `alsa-state.service` entered the active state roughly 1.4 seconds before the
  first PipeWire/WirePlumber startup recorded in the boot journal.
- On the first observed Linux boot there was no saved
  `/var/lib/alsa/asound.state`; one now exists after development and contains
  the manually selected headphone state.

The route was logically consistent but physically silent. The user recovered
audio by selecting PipeWire Line Out and then ALSA Headphone. A guarded
no-stream replay proved why: the generic ACP headphone path writes
`Front Playback Switch=off`, but the AE-5 headphone path shares that Front DAC.
Line Out writes `Front=on`, and the final ALSA Headphone selection routes that
enabled DAC to the headphone jack.

### First instrumented reboot

Boot `5c9efcee-2a1a-4cf3-ac07-bf5154ab6ef7` on 2026-07-24 provided the first
instrumented cold-boot sample:

- The kernel downloaded and started the CA0132 DSP without a relevant warning
  or error.
- The pre-PipeWire service found ALSA card 0, but its three `amixer` reads
  failed with `Invalid card number '0'`. The sysfs card was visible before the
  ALSA control device was ready for that user process, so this sample does not
  establish the pre-PipeWire logical route.
- After PipeWire started, ALSA reported `Headphone`, auto-detect off, and the
  headphone jack on. PipeWire selected the matching headphone port.
- Codec pins matched the driver's intended AE-5 headphone route: line-out
  `0x0b`, surround `0x0f`, and front headphone/center-LFE `0x10` were off,
  while rear headphone `0x11` was enabled for output.

The user subsequently confirmed that this internally consistent state was
silent on the same boot. The before and after snapshots share boot ID
`5c9efcee-2a1a-4cf3-ac07-bf5154ab6ef7`; codec pins and ALSA output selection
were unchanged after recovery. The post-recovery PipeWire route instead
identified Line Out, and `Front` was on.

This reboot also exposed two probe defects: it queried ALSA too early and
omitted the AE-5 rear-headphone pin `0x11`.

## Confirmed ACP root cause and fix

With no playback stream open, the exact transition matrix was:

| Step | PipeWire route | Output Select | Front switch | Master |
|---|---|---|---|---|
| Known working state | Line Out / Speaker | Headphone | on | 87 |
| Select generic Headphones | Headphones / Headphones | Headphone | **off** | 76 |
| Select Line Out | Line Out / Speaker | Speakers | on | 87 |
| Select ALSA Headphone | Line Out / Speaker | Headphone | on | 87 |

The upstream generic
`analog-output-headphones.conf` deliberately defines `[Element Front]` with
`switch=off`, because it assumes Front belongs only to speakers or line out.
That assumption is false for CA0132 desktop cards.

The package now supplies:

- `sound-blaster-ae5-output-headphones.conf`, which includes the generic path
  but changes only Front to `switch=mute` and `volume=zero`;
- `sound-blaster-ae5.conf`, which replaces the generic headphone path in the
  analog stereo mappings;
- a WirePlumber rule selecting that profile set only for PCI Creative
  `1102:0012` cards.

The live profile was parsed by PipeWire's `spa-acp-tool`, exposed the fixed
headphone route, and passed Speakers → Headphones switching with
`Output Select=Headphone` and `Front=on`. It also survived a WirePlumber
restart with the fixed route, 43% sink volume, and the expected ALSA controls.
An acoustic microphone check and repeated cold-boot/suspend testing remain.

WirePlumber documents `monitor.alsa.rules` as the supported mechanism for
updating ALSA-device properties, while ACP is responsible for profiles, ports,
and mixer settings:
[ALSA configuration](https://pipewire.pages.freedesktop.org/wireplumber/daemon/configuration/alsa.html).

## Safe cold-boot probe

`collect-routing-state.sh` discovers the exact audited card by
`1102:0012/1102:0051`; it does not assume card 1. It reads the route controls,
jack state, all four AE-5 output pins, service timing, and the matching
PipeWire card/sink. It waits up to five seconds for the ALSA control interface
to become readable and never writes a mixer control.

Install two temporary user services:

```sh
bash scripts/install-routing-probe.sh
```

The first snapshot is ordered before the PipeWire process. It deliberately
skips `pactl`, because connecting to PipeWire's passive socket would itself
start the process. A timer takes the second snapshot eight seconds after the
user manager starts. Both append to the private file:

```text
~/.local/state/ae5-control/routing-boot.log
```

After the next reboot, test sound before opening AE-5 Control or manually
toggling an output. If sound is broken, capture one more snapshot:

```sh
bash scripts/collect-routing-state.sh before-toggle --append
```

Then perform exactly one output toggle, confirm whether audio starts, and run:

```sh
bash scripts/collect-routing-state.sh after-toggle --append
```

Remove the temporary boot probes after the experiment:

```sh
bash scripts/install-routing-probe.sh --uninstall
```

Uninstalling leaves the private log intact.

## How the evidence will be interpreted

| Observation | Most likely next target |
|---|---|
| The current cold boot works | Treat the two upstream 2026 fixes as the resolution; test repeated boots before closing the bug |
| Pre-PipeWire and post-PipeWire ALSA values differ | WirePlumber/ACP policy or state restoration |
| ALSA says `Speakers` before PipeWire, then `Headphone` after it | Expected explicit userspace routing; investigate only if the physical route remains wrong |
| ALSA and pins are correct, but generic Headphones sets Front off | Confirmed here: fix the card-specific ACP path |
| ALSA, pins, and Front are all correct, but a same-value toggle starts audio | CA0132 DSP route initialization/replay; instrument `ca0132_alt_select_out()` |
| Jack presence is wrong before PipeWire | HDA unsolicited/presence detection or board pin configuration |
| Only the PipeWire port is wrong | WirePlumber profile/port persistence, not the kernel driver |

No CA0132 patch is justified for this reproduced failure. Docker and WSL cannot
validate PCI/DSP initialization. A Linux host with VFIO PCI passthrough can
give a Linux or Windows guest direct ownership of the AE-5; this target is
especially suitable because `0000:29:00.0` is the only device in IOMMU group
28. The host must release the card while the guest owns it, and physical
cold-boot checks remain authoritative. The audited setup, safety boundaries,
kernel A/B matrix, and recovery gates are in
[`VFIO_TEST_PLAN.md`](VFIO_TEST_PLAN.md).
