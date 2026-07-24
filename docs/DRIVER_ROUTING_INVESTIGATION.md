# CA0132 cold-boot routing investigation

This investigation concerns the report that AE-5 headphone output sometimes
starts working only after toggling the ALSA output selection. No driver change
should be proposed until that symptom is reproduced on the current kernel and
the failing layer is identified.

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

At the time of investigation:

- ALSA `Output Select` was `Headphone`.
- `HP/Speaker Auto Detect` was off.
- The read-only kernel `Headphone Jack` control was on.
- PipeWire selected
  `analog-output-headphones;output-headphones`.
- WirePlumber's ACP device had automatic profile and port selection disabled,
  so it restores an explicit route.
- `alsa-state.service` entered the active state roughly 1.4 seconds before the
  first PipeWire/WirePlumber startup recorded in the boot journal.
- On the first observed Linux boot there was no saved
  `/var/lib/alsa/asound.state`; one now exists after development and contains
  the manually selected headphone state.

These facts do not prove a current bug. The originally observed behavior may
already be fixed in Linux 7.1, may depend on state restored at boot, or may be
a mismatch between the driver's logical control value and the DSP's actual
route.

## Safe cold-boot probe

`collect-routing-state.sh` discovers the exact audited card by
`1102:0012/1102:0051`; it does not assume card 1. It reads the route controls,
jack state, relevant codec pins, service timing, and the matching PipeWire
card/sink. It never writes a mixer control.

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
| ALSA and PipeWire values are correct, but one same-value toggle starts audio | CA0132 DSP route initialization/replay; instrument `ca0132_alt_select_out()` |
| Jack presence is wrong before PipeWire | HDA unsolicited/presence detection or board pin configuration |
| Only the PipeWire port is wrong | WirePlumber profile/port persistence, not the kernel driver |

If a kernel defect remains, the next patch will be made against upstream
`ca0132.c`, kept minimal, and tested first as a matching out-of-tree test
module. Docker and WSL cannot validate PCI/DSP initialization. A Linux host
with VFIO PCI passthrough can give a Linux or Windows guest direct ownership of
the AE-5, but only real hardware, cold boots, and host/guest isolation make
that result meaningful.
