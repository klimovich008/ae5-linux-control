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

## Onboard RGB follow-up

The separate
[`ca0132-ae5-onboard-leds.patch`](../kernel/ca0132-ae5-onboard-leds.patch)
was added to the same production LTS stack; the diagnostic SpeakerEQ probe was
excluded. The resulting kernel reported `6.18.40-ae5-lts-rgb+`.

The production CA0132 object and DSP parser-test object passed `W=1` with
warnings as errors. The full `bzImage modules` build completed, the x86
instruction decoder checked 7,889,417 real instructions, and the randomized
instruction test completed 1,000,000 cases with no error. Important SHA-256
values were:

| Artifact | SHA-256 |
|---|---|
| `arch/x86/boot/bzImage` | `d4f11b32742a8a13ed40199a4857c87c585aabd8485822a0f36398c9a2b9673c` |
| `vmlinux` | `733e30dfd30155119babb22e2e6ffa4bfdfc7463c553aa49a7349ffecd66eff8` |
| `System.map` | `14fb38af2bced8982aa24462d498dd58a8c29d71bb8ef9daa04348994d1a4787` |
| `snd-hda-codec-ca0132.ko` | `f39b0e3a37d384d9d4f1e1e0dcef01e847c2e8e8b56732f820f97523617d1eda` |
| `led-class-multicolor.ko` | `574cbf4c44857daa48036390d2e01e57d6c3621dd11821954d7efc74207d69ea` |

The candidate was installed as an additional BLS entry with the late-probe
kernel retained as the saved fallback. A one-shot card-less boot loaded both
modules, reported no AE-5 LED as expected, and had zero failed units.

The managed physical cycle then received `1102:0012/1102:0051` at
`0000:07:00.0`, bound it to `snd_hda_intel`, and initialized the CA0132 DSP
exactly once. It exposed five devices named
`hdaudioC0D1:rgb:ae5-1` through `ae5-5`. Each reported:

- `multi_index` as `red green blue`;
- `brightness` and `max_brightness` as `255`;
- `multi_intensity` as three independently writable channel values.

Root-owned writes exercised solid red, green, and blue, a five-color
per-LED pattern, and one LED's brightness off/on path. Every value read back
through the LED class. The complete 72-control guest mixer hash remained
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`,
no PCM was open, the DSP initialized only once, no unit failed, and no
CA0132/HDA timeout, lockup, or warning appeared.

Visible color confirmation remains an acceptance gate. After clean shutdown,
the host recovered without a fallback restore: raw ALSA state and all 47 app
controls were byte-identical, the mixer hash returned to
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`,
all three WirePlumber state files were byte-identical, the AE-5 and Fifine
defaults returned, the hostdev was absent, and VFIO preflight passed.

### Normal-user application cycle

A second managed physical cycle booted the same RGB kernel and installed the
final binary RPM with SHA-256
`96410b79323cc5396f0e84164c1434ca3aca5c2490eafc0fef7aa69e3ca2293e`.
Before installation, the five `brightness` and five `multi_intensity`
attributes were root-owned mode `0644`. The package's exact udev match changed
only those attributes to `0666`; immutable identity attributes such as
`multi_index` and `max_brightness` remained root-owned mode `0444`.

The unprivileged SSH user was not in `audio` and had no desktop-session ACL.
It nevertheless used the installed lighting-only CLI path to apply and verify
one solid frame and the independent pattern `#FF0000`, `#00FF00`, `#0000FF`,
`#FFA000`, and `#B400FF`. The versioned, card-targeted configuration was
created as a user-owned file and `lighting-restore` reproduced it after direct
temporary white values.

Invalid LED and color arguments left the configuration and hardware hashes
unchanged. Making the configuration directory unwritable caused the hardware
transaction to roll back exactly. A forced loss of write permission on the
third LED exposed and then verified a userspace rollback-reporting fix: the
final backend skips LEDs already at their saved value, attempts every required
recovery, and reports only the underlying permission failure when recovery is
complete.

Throughout the package install, normal-user writes, injected failures,
restore, and removal, the complete guest mixer hash remained
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`.
No PCM was open, the DSP initialized once, no unit failed, and no relevant
driver warning appeared. Removal deleted the udev/autostart files, returned
all ten writable attributes to root-owned mode `0644`, and preserved the
user's lighting file byte-for-byte.

On host recovery, all 47 writable application controls matched the pre-cycle
profile exactly after desktop session policy restarted. The raw mixer diff
was limited to the read-only volatile `Playback Channel Map`: it was `FL,FR`
during the active pre-test Brave stream and zero with no post-test stream.
WirePlumber default-node and route files stayed byte-identical; the
stream-properties objects were semantically identical and their saved byte
ordering was restored. The AE-5/Fifine defaults, `snd_hda_intel` binding,
inactive domain without a hostdev, and ready VFIO preflight all returned.

### Analog headphone output cycle

An additional managed cycle booted the same `6.18.40-ae5-lts-rgb+` kernel,
without the diagnostic SpeakerEQ probe, and used direct ALSA playback plus the
host's Fifine microphone to measure the physical headphone output. The guest
selected Headphone, Low gain, 2.0 channels, disabled output processing, kept
Front at 0 dB, and used a two-second 997 Hz signed-32-bit fixture at
`-18 dBFS`.

Two captures at each Master value measured:

- raw 55: `-91.85 dBFS` mean, `0.24 dB` repeat spread;
- raw 60: `-86.64 dBFS` mean, `0.18 dB` repeat spread;
- raw 65: `-82.30 dBFS` mean, `0.58 dB` repeat spread.

The measured `+5.21 dB` and `+4.34 dB` changes follow the two advertised
`+5.00 dB` steps within `0.66 dB`. Master mute reached `-105.96 dBFS`, within
`0.88 dB` of the quiet baseline, and a confirmed Front-muted repeat suppressed
the tone by more than 34 dB. At Master 55, Medium gain measured `+1.28 dB` and
High measured `+7.04 dB` relative to Low; High was exercised only at that
attenuated level and the headphones were not worn.

A bounded 18 kHz attempt also documented what this acoustic setup cannot
prove. Slow and Fast were only 1.33 and 5.10 dB above the quiet 18 kHz
baseline, while Minimum Phase differed by 14.44 dB between repeats. The
roll-off filters therefore remain unverified pending an attenuated electrical
capture or analyzer with adequate high-frequency signal-to-noise ratio.

Restoring the guest ALSA snapshot reproduced mixer SHA-256
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`.
The DSP initialized once, all PCMs were closed, five LED devices remained
registered, no unit failed, and no relevant warning appeared. Clean shutdown
returned the card to host `snd_hda_intel`; all 46 current application-profile
controls and all three WirePlumber files matched exactly, the no-stream host
mixer hash returned to
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`,
the AE-5/Fifine defaults returned, the inactive domain contained no hostdev,
and VFIO preflight was ready. Raw microphone recordings were deleted after
deriving the documented values.

## Remaining limits

This maintained-LTS cycle proves buildability, parser safety, bootability,
physical initialization, control defaults, manual-route behavior, one safe
app write, onboard-LED class registration/writes, package install/removal, and
host recovery. It now also proves the scoped normal-user lighting backend,
persistence, rollback, permission cleanup, and external headphone
level/mute/gain behavior. A separate KDE/Wayland test exercised the unchanged
release GUI's native GTK color chooser, unified and individual writes,
cancellation, persistence restore, and cold readback against an isolated
five-device LED-class fixture while preserving the exact host mixer state. It
does not replace visibly confirming a GUI-selected color on the physical
card, the external-strip protocol, or the remaining bare-metal cold-boot,
suspend/resume, speaker/line-out/digital, analog-input, electrical filter,
long-duration stability, and Windows analog-parity gates.
