# CA0132 source and interoperability inventory

This inventory pins every external implementation reference before kernel work
begins. Public source and observable hardware behavior are preferred over
proprietary binary analysis. No Creative executable, driver binary, firmware,
decompiler output, or private Windows data belongs in this repository.

The inventory was last verified on 2026-07-24.

## Exact source for the running Nobara driver

The target system runs:

- kernel `7.1.4-200.nobara.fc44.x86_64`;
- module
  `/lib/modules/7.1.4-200.nobara.fc44.x86_64/kernel/sound/hda/codecs/snd-hda-codec-ca0132.ko.zst`;
- module `srcversion` `C2F3A21840C28ABD806F5D1`;
- source package `kernel-7.1.4-200.nobara.fc44.src.rpm`.

The installed `kernel-devel` package contains headers and build infrastructure,
not `ca0132.c`. Nobara publishes the matching source RPM in its
[package repository](https://use.nobaraproject.org/rolling/linux-nobara-mainline/x86_64/Packages/k/kernel-7.1.4-200.nobara.fc44.src.rpm).
It was downloaded to a temporary directory, verified, inspected, and removed:

| Evidence | Value |
|---|---|
| Source RPM SHA-256 | `3c832ad0c6ceacf76c94648d5d2964a338fa9e734c6ca8c09e17ed05dd015fd7` |
| RPM signature key | `770ca349ff06522c0fa294b672bb3d5a06ed89f5` |
| RPM signature and payload digests | valid |
| Base archive | `cachyos-7.1.4-1.tar.gz` |
| CachyOS tag commit | [`5b7a496b2c1a4886925ae25d762e0d34df214dcb`](https://github.com/CachyOS/linux/commit/5b7a496b2c1a4886925ae25d762e0d34df214dcb) |
| Extracted `ca0132.c` SHA-256 | `7b61bcb02c4079b9ca6c82cde3147e95706cdbe958324ae383e7875d9a33a4f0` |
| Downstream patch audit | none of the 16 packaged patches references CA0132 |

The extracted file is byte-identical to
[`sound/hda/codecs/ca0132.c`](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/tree/sound/hda/codecs/ca0132.c?h=v7.1.4)
from Linux stable `v7.1.4`, whose dereferenced tag commit is
[`7a5cef0db4795d9d453a12e0f61b5b7634fc4d40`](https://git.kernel.org/pub/scm/linux/kernel/git/stable/linux.git/commit/?id=7a5cef0db4795d9d453a12e0f61b5b7634fc4d40).
The file is licensed `GPL-2.0-or-later`.

This establishes the exact source used for the CA0132 part of the installed
kernel package. A compiled-object reproducibility check is not required for the
current source-level diagnosis, but can be added if a distribution build
discrepancy is suspected.

## Upstream routing fixes

Both relevant fixes are present in the verified source:

| Commit | Behavior |
|---|---|
| [`778031e1658d`](https://github.com/torvalds/linux/commit/778031e1658d206a52bf9491c91ae5d4f4a2509d) | Defaults HP/Speaker auto-detect from the headphone pin's presence-detect capability. |
| [`6fd9f6e870ea`](https://github.com/torvalds/linux/commit/6fd9f6e870ea285f05102e8e00e6a7f4495a9a02) | Disables auto-detect and reapplies the route when a manual output is selected, including a same-value selection. |

The source calls `ca0132_alt_select_out()` during initialization after DSP
download and board setup. That function mutes the DSP, programs the selected
AE-5 output path, restores the output effects, clears the speaker-EQ-use flag,
and unmutes the DSP. The instrumented failing boot showed that this kernel path
had selected the correct pins. The later no-stream route matrix identified the
actual fault in PipeWire's generic ACP headphone path: it forced the shared
CA0132 `Front Playback Switch` off. The card-specific profile fix and evidence
are in [`DRIVER_ROUTING_INVESTIGATION.md`](DRIVER_ROUTING_INVESTIGATION.md).

The upstream `master` snapshot
[`48a5a7ab8d6a`](https://github.com/torvalds/linux/commit/48a5a7ab8d6ab7090564339e039c421f315de912)
had `ca0132.c` SHA-256
`95a23cdef3504d67762b35d3e0fcedf31651233f08477c4dcf56bd436c2552cb`
when this inventory was recorded. Kernel patches must be rebased onto the
current sound maintainer tree before submission; this snapshot is not a
permanent patch base.

The bounded DSP-image candidate was prepared against Takashi Iwai's
[`sound.git` `for-next` commit
`61471f29f315`](https://git.kernel.org/pub/scm/linux/kernel/git/tiwai/sound.git/commit/?h=for-next&id=61471f29f3157f33a61194bf82b4a289cc03e1f1).
At that commit, `ca0132.c` has the same
`95a23cdef3504d67762b35d3e0fcedf31651233f08477c4dcf56bd436c2552cb`
SHA-256 as the recorded `master` snapshot. The exact generated diff is stored
as
[`kernel/ca0132-dsp-image-bounds.patch`](../kernel/ca0132-dsp-image-bounds.patch);
its implementation and validation record is in
[`kernel/README.md`](../kernel/README.md).

At the same verification time, `sound.git` `master` was
`f5657cb8480cd4b38589bf50cd8eae07e183b53e` and contained the same CA0132
file. The factory-EQ cache candidate therefore applies to the exact running
`v7.1.4` source and all three recorded upstream snapshots without rebasing.

## Firmware already distributed for Linux

Fedora package `alsa-firmware-1.2.4-17.fc44` supplies the target system's
Creative firmware:

| File | SHA-256 |
|---|---|
| `ctefx-desktop.bin` | `c9ab092e5717080bcd90971d44aa7d8d30778058ea691dda76320cd315dcc18e` |
| `ctefx-r3di.bin` | `324b44968afd5a232651f5e040e41c12ebd120cd768c9559a4a7b7a5c823f7dc` |
| `ctefx.bin` | `dcf71cabbdf7f0febbb091fe3ee88d772b64cc6fb515e1ecc780d514f1440c9f` |
| `ctspeq.bin` | `4d7e872064a2e0b6c10de1b1a4ca16b0d2e2557cab3a15d53cc9c56c2369c4be` |

The Creative firmware licence permits unmodified binary redistribution for
use with an open-source operating system, but prohibits reverse engineering,
decompilation, and disassembly. The firmware may be loaded and its externally
observable behavior measured; it must not be disassembled.

The
[ALSA firmware commit that added `ctspeq.bin`](https://github.com/alsa-project/alsa-firmware/commit/cbb9d36a7cdb36697e0db2f8455465bdaa3008c2)
identifies it as a SpeakerEQ coefficient preset tuned for Chromebook Pixel
hardware. A later Linux driver comment associates it with a similar
speaker/headphone EQ upload path observed on Windows, but explicitly presents
that association as a belief. It is not evidence that the Chromebook preset is
an AE-5 headphone profile. The verified conclusion and safe experiment order
are recorded in
[`HEADPHONE_TUNING_INVESTIGATION.md`](HEADPHONE_TUNING_INVESTIGATION.md).

## Additional public references

| Reference | Pinned revision | Licence and allowed role |
|---|---|---|
| [`Conmanx360/ca0132-tools`](https://github.com/Conmanx360/ca0132-tools) | [`6c1563c6ec07a18e9aa0a51a0a697c7a61de242d`](https://github.com/Conmanx360/ca0132-tools/commit/6c1563c6ec07a18e9aa0a51a0a697c7a61de242d) | No explicit licence was found. Do not copy, vendor, modify, or redistribute its code without permission. Its README warns that unsafe commands can lock the DSP/8051 and that its disassembler must not be used on DSP firmware. |
| [`Conmanx360/QemuHDADump`](https://github.com/Conmanx360/QemuHDADump) | [`82aa13e45c63ad2a0d1c411923b27f6ccbb48686`](https://github.com/Conmanx360/QemuHDADump/commit/82aa13e45c63ad2a0d1c411923b27f6ccbb48686) | No explicit licence was found. Treat it as a description of a possible HDA-verb observation technique; do not integrate or redistribute its code without permission. A VM trace does not replace physical AE-5 testing. |
| [OpenRGB AE-5 merge request `!2997`](https://gitlab.com/CalcProgrammer1/OpenRGB/-/merge_requests/2997) | Squash commit [`587a706f2873e7632ff835f9d8fda98d70e4d957`](https://gitlab.com/CalcProgrammer1/OpenRGB/-/commit/587a706f2873e7632ff835f9d8fda98d70e4d957) | `GPL-2.0-only`. Documents Windows-only AE-5/AE-5 Plus RGB discovery and commands. It is useful for a later RGB workstream, not audio routing or quality. |

OpenRGB's merged implementation is explicitly Windows-only. Linux RGB still
needs a narrow kernel-managed interface; unrestricted `/dev/mem` or userspace
MMIO is out of scope.

## Proprietary Windows package boundary

Creative publishes Sound Blaster Command installers, but the
[official download agreement](https://support.creative.com/downloads/download.aspx?nDownloadId=100330)
restricts decompilation, disassembly, memory dumps, and reverse engineering
except where the agreement or applicable law expressly permits it. It directs
users seeking interoperability information to request it from Creative. No
Creative installer or driver was downloaded or decompiled during this audit.

The permitted evidence order for this project is:

1. verified GPL Linux source and history;
2. Creative's public documentation and user-exported JSON;
3. normal hardware controls, logs, audio measurements, and one-setting-at-a-time
   observation on hardware the user owns;
4. public GPL implementations such as OpenRGB within their actual scope;
5. a narrowly scoped Windows trace or static analysis only after the user
   confirms a lawful basis and the applicable licence has been reviewed.

Any cleared Windows analysis must produce an independent behavior
specification—inputs, outputs, HDA verbs, parameter identifiers, ordering, and
timing—not copied control flow, proprietary code, firmware contents, or
decompiler output.

## Evidence required before a driver patch

1. Repeat the now-reproduced cold-boot failure with the fixed ACP profile and
   record the acoustic result before any mixer action.
2. Compare Windows, direct ALSA, and PipeWire captures with the parity harness.
3. Separate digital DSP differences from analog DAC/amplifier differences.
4. Instrument the earliest divergent public-source state transition.
5. Build the smallest patch against the current upstream sound tree and test it
   as an alternate kernel/module while retaining the known-good kernel.

Until one of these tests fails reproducibly, there is no justified CA0132 code
change.

This gate applies to the reported routing and audio-parity problems. A separate
read-only control audit found a self-contained CA0132 defect: Wedge Angle
declares a `20..180` range but initializes its cached public value to `10`.
Both the running `v7.1.4` driver and upstream `master` at `48a5a7ab8d6a`
contain the same line. The original line was introduced by `44f0c9782cc6`
(`ALSA: hda/ca0132: Add tuning controls`). The minimal fix and its evidence are
recorded in [`kernel/README.md`](../kernel/README.md); it does not alter routing
or make any audio-parity claim.

A later physical EQ audit found a second self-contained control defect. The
factory-preset callback writes requests 10 through 20 from
`ca0132_alt_eq_presets[]`, then updates only `eq_preset_val`.
`tuning_ctl_get()` returns the independent `cur_ctl_vals[]` cache for all ten
band controls, and the preset callback never synchronizes that cache. Acoustic
therefore changed the measured DSP response while every band still reported
0 dB. The callback is identical in the exact running source and all recorded
upstream snapshots. The minimal cache/notification fix, userspace compatibility
guard, and validation record are in
[`kernel/README.md`](../kernel/README.md).
