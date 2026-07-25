# Linux 6.18 LTS validation

## Scope and source

Collected on 2026-07-25 against Linux `6.18.40`, the newest longterm release
listed by [kernel.org](https://www.kernel.org/) that day, at stable commit
`221fc2f4d0eda59d02af2e751a9282fa013a8e97`. The source combined:

1. upstream CA0132 auto-detect commits `778031e1658d` and `6fd9f6e870ea`;
2. the Wedge Angle default fix;
3. the factory-EQ control-cache fix;
4. the AE-5 What U Hear mixer cleanup;
5. the DSP-image bounds parser and KUnit tests.

The exact application order and 6.18 context adapter are in
[`kernel/backports/6.18/README.md`](../kernel/backports/6.18/README.md).
The diagnostic SpeakerEQ address probe was not included.

## Static, build, and KUnit gates

The combined source passed `git diff --check`. Both the production
`sound/hda/codecs/ca0132.o` and parser-test object compiled with `W=1` and
warnings treated as errors.

The x86-64 KUnit run passed all four `ca0132-dsp-image` cases:

- valid image;
- truncated segment;
- HCI segment;
- metadata segment.

The complete bootable build reported `6.18.40-ae5-lts+`. The plus suffix
deliberately identifies a kernel built from a modified stable tree. Important
artifact SHA-256 values were:

| Artifact | SHA-256 |
|---|---|
| `arch/x86/boot/bzImage` | `7b1b83c0e227515c75fbf33afb0925d8ccd43298eed956d0e0de44209341156d` |
| `vmlinux` | `9e5f7d0a417393515be3c27306b2cbd5c2364acfc273de133cb67ed3c37ba3a1` |
| `System.map` | `cd01806250a4e68ac1feaa06619afea81f2f246cfa23fd1238d7c98733cba14d` |
| `snd-hda-codec-ca0132.ko` | `e4fa2d7d9a9f06501a6ed19063716c11e768a3ee4acb2fecd455db7f6257eae7` |

## Isolated boot gates

The kernel was installed as an additional Boot Loader Specification entry;
the stock Fedora kernel and earlier integrated test kernel remained available
for rollback. The first session-guest attempt exposed a Btrfs-specific BLS
path error: `grubby` generated `/vmlinuz-*` instead of `/boot/vmlinuz-*`.
Selecting the preserved kernel from the console recovered the guest. The
corrected entry was verified before retrying.

Both the session guest and the system guest then booted
`6.18.40-ae5-lts+` without a passed-through device. Each passed:

- Btrfs root and separate Btrfs `/boot`;
- VirtIO networking;
- nested KVM availability;
- CA0132 module load and matching vermagic;
- built-in ALSA sequencer and `/dev/snd/seq`;
- zero failed systemd units.

The minimal cloud image logged its existing TPM-user, BPF, and SELinux
warnings. None involved CA0132, HDA, ALSA, PCI reset, or the test kernel.

## Physical AE-5 cycle

The system guest was powered off before attaching the isolated
`0000:29:00.0` function with libvirt `managed='yes'`. Host PipeWire and
WirePlumber were stopped, no AE-5 PCM was open, the Fifine USB microphone was
present, and a private recovery set captured raw ALSA, all 47 app profile
controls, PipeWire defaults/routes, and WirePlumber state.

The guest booted the LTS kernel, received Creative `1102:0012` subsystem
`1102:0051` at `0000:07:00.0`, bound it to `snd_hda_intel`, and initialized
the DSP exactly once. Read-only checks found:

- 72 ALSA controls and 46 simple controls;
- Wedge Angle `30` in its `20..180` range;
- Flat EQ and all ten band values at raw `24`;
- `CA0132 What U Hear` capture PCM at device 2;
- no ineffective What U Hear volume or mute controls;
- zero failed guest units and zero relevant driver warnings.

The complete guest mixer SHA-256 was
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`,
matching all earlier integrated-kernel cycles.

The packaged `ae5-control-0.1.0-1.fc44.x86_64.rpm`, SHA-256
`63c0d378607625593964fab95dba856d5109222a633119075adbff38cac6da3b`,
installed successfully. The packaged CLI detected the exact card and saved
and validated a 46-control guest profile. In this headless SSH guest,
`sudo` was required because no desktop login session granted a device ACL,
and PipeWire route commands were unavailable without a user session.

The packaged CLI changed Wedge Angle to `20`, read it back, returned it to
`30`, and restored the complete mixer hash. A direct ALSA test then enabled
HP/Speaker auto-detect and selected Headphone manually. The manual selection
immediately disabled auto-detect and produced:

- Headphone: pin `0x11` output on; `0x0b`, `0x0f`, and `0x10` off;
- Speakers: pins `0x0b`, `0x0f`, and `0x10` on; `0x11` off.

No PCM was open. Returning to Speakers reproduced the exact starting mixer
hash with no CA0132/HDA warning. Removing the RPM left the same mixer hash,
Wedge `30`, Flat EQ, one DSP initialization, and zero failed guest units.

## Host recovery

Clean guest shutdown rebound the card from `vfio-pci` to host
`snd_hda_intel` in about two seconds. The temporary hostdev was immediately
removed, both guests remained off, no QEMU process remained, and VFIO
preflight passed again.

No fallback restore was needed:

- raw host ALSA state was byte-identical before and after;
- all 47 saved profile controls were semantically identical;
- full host mixer SHA-256 returned to
  `3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`;
- WirePlumber default-node and route files were byte-identical;
- `stream-properties` differed only in JSON key order and had identical
  canonical content;
- AE-5 returned as the default sink on the packaged headphone route;
- Fifine remained the default source.

The recent reset and rebind window contained no CA0132, HDA, VFIO, timeout,
or reset-failure warning. The root-owned `0600` system qcow2 could not be
rerun through `qemu-img check` without interactive host authorization; the
domain was cleanly shut down and libvirt reported no storage error.

## Read-only SpeakerEQ follow-up

The diagnostic probe excluded from the production validation stack was later
built as two separate kernels on the same source and configuration. The first,
`6.18.40-ae5-lts-speq+`, queried immediately after firmware download. The
second, `6.18.40-ae5-lts-speq-late+`, queried after the complete AE-5 DSP setup.
Both passed a no-device boot before receiving the physical card.

Each physical boot initialized the DSP once but received no reply to
`MASTERCONTROL_QUERY_SPEAKER_EQ_ADDRESS`; dynamic debug recorded exactly one
`SpeakerEQ address query failed: -5` after `dspio_scp: send scp msg failed`.
The later placement rules out incomplete card-specific setup as the simple
cause. No undocumented request variant and no coefficient upload was attempted.

The late-probe kernel retained the same controls, defaults, What U Hear PCM,
zero failed units, and guest mixer hash as the production LTS stack. A muted
direct-ALSA playback/capture check found the 997 Hz fixture at a strongest bin
of `996.09375 Hz`, then restored the exact guest hash. Both follow-up shutdowns
again produced byte-identical host ALSA state, identical application controls,
the expected WirePlumber/default-device state, the known host mixer hash, and a
clean VFIO preflight.

## Remaining limits

This maintained-LTS cycle proves buildability, parser safety, bootability,
physical initialization, control defaults, manual-route behavior, one safe
app write, package install/removal, and host recovery. It does not replace the
remaining bare-metal cold-boot, suspend/resume, speaker/line-out/digital,
analog-input, long-duration stability, and Windows analog-parity gates.
