# CA0132 source and interoperability inventory

This inventory pins every external implementation reference before kernel work
begins. Public source and observable hardware behavior are preferred over
proprietary binary analysis. No Creative executable, driver binary, firmware,
decompiler output, or private Windows data belongs in this repository.

The inventory was last verified on 2026-07-26.

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

At the earlier verification time, `sound.git` `master` was
`f5657cb8480cd4b38589bf50cd8eae07e183b53e` and contained the same CA0132
file. On 2026-07-26, `master` had advanced to
`a9bde483214af0b667e282131fd4aebe50695f02`, while its `ca0132.c` remained
byte-identical with SHA-256
`95a23cdef3504d67762b35d3e0fcedf31651233f08477c4dcf56bd436c2552cb`.
The factory-EQ cache candidate therefore still applies to the exact running
`v7.1.4` source and all three recorded upstream snapshots without rebasing.

A direct remote-head check on 2026-07-26 found `for-next` still at
`61471f29f3157f33a61194bf82b4a289cc03e1f1`. All four functional patches and
the onboard-LED candidate apply independently and in their production series
in a clean worktree. The Direct Mode patch was regenerated from that pristine
base to remove an accidental LED-context dependency; it now applies both
standalone and after the production/RGB stack on `for-next` and Linux 6.18.40.
Standalone and combined objects rebuilt with warnings as errors. The patches
therefore have no outstanding source rebase delta as of that check; external
submission still requires the contributor's own DCO sign-off.

## What U Hear control history

The AE-5's `What U Hear Capture Volume` and `What U Hear Capture Switch` are
standard HDA input-amplifier controls for node `0x0a`. They entered the
alternate desktop mixer through
[`017310fbe767`](https://github.com/torvalds/linux/commit/017310fbe7670f522cdde4e68d4e1859f16d2757),
then the AE-5 began using that mixer in
[`88268ce8a64e`](https://github.com/torvalds/linux/commit/88268ce8a64ec0658a9131d491cc5575372ef0ad).
The same source separately selects the DSP loopback source through module
`0x31`; no verified public implementation was found for a What U Hear DSP
gain parameter.

Direct physical-card testing proved that node `0x0a` accepts and reports
level and mute writes while its captured signal remains identical. The
counterbalanced method and values are in
[`RECORDING_MIXER_INVESTIGATION.md`](RECORDING_MIXER_INVESTIGATION.md). The
driver already has precedent for omitting CA0132 mixer elements that advertise
unsupported operations:
[`c41999a23929`](https://github.com/torvalds/linux/commit/c41999a23929f30808bae6009d8065052d4d73fd).

The minimal AE-5-only candidate is
[`kernel/ca0132-ae5-hide-ineffective-wuh-controls.patch`](../kernel/ca0132-ae5-hide-ineffective-wuh-controls.patch).
It applies to the exact running source and all three upstream snapshots pinned
above, passes strict `checkpatch.pl`, and compiled with `W=1` and warnings as
errors against both running and current upstream source. It retains the What U
Hear PCM. No proprietary binary, firmware disassembly, or decompiler output
was used.

## Windows Direct Mode interoperability reference

After the user explicitly requested an installed-driver comparison, three
locally owned `CtxHda.sys` versions were analyzed offline. The files remain
outside this repository:

| Version | Role | SHA-256 |
|---|---|---|
| `6.0.105.0065` | Active Windows driver | `4be35390a2de694041cd20317ed5a148d4852e46f201945a346a8b2a2c79dccf` |
| `6.0.105.0064` | Previous DriverStore package | `3e250aa313f15d960d9717ca93a37783ccada108d02c5f8cb6de9a453367b79c` |
| `6.0.105.0055` | Older comparison package | `9273eb1c873224cc99de7fd8398924c4e8e86fa0a9f81639a0970dd2c730f201` |

Official Ghidra `12.1.2` was used from an archive with SHA-256
`b62e81a0390618466c019c60d8c2f796ced2509c4c1aea4a37644a77272cf99d`.
Its projects, scripts, and reports are private local analysis material and are
not redistributed.

Only an independent behavior specification was carried into the project:

- the AE-5 backend is selected for subsystem `1102:0051`;
- Direct Mode stops ChipIO stream `0x18`;
- normal mode restores streams `0x05` and `0x18`, connection point `0xd0` at
  96 kHz, six stream channels, stream enable, and ASI value `7`;
- the newer transition quiesces the endpoint before changing route state;
- direct formats are stereo and DSP volume is bypassed.

The stream routes, `0xd0` rate, channel count, and enable state independently
match the GPL Linux AE-5 startup path. Linux startup currently writes ASI value
`4`, whereas the Windows Direct-to-normal transition writes `7`; the candidate
uses that transition-specific value. Physical-card tests now validate exact
normal-route restoration around it; repeated-boot, power-management, and
connected line-out gates remain. No decompiled function, copied control flow,
binary, or report is committed. The resulting candidate and physical
acceptance boundary are documented in
[`DIRECT_MODE_INVESTIGATION.md`](DIRECT_MODE_INVESTIGATION.md).

## Windows Super X-Fi capability reference

The user's exact Sound Blaster Command 3.5.10.0 installation was inspected
offline to correct an overly broad “feature absent from the build” assumption.
The application is a multi-device bundle and does include generic Super X-Fi
UI, cloud, profile, and device-feature components. Their presence does not
mean that the AE-5 supports Super X-Fi.

The narrow managed capability path in
`Creative.Platform.Devices.dll` creates a Super X-Fi feature only when a live
device repository exposes `SuperXFiFeatureId`; otherwise the feature enricher
returns no feature. The exact installed AE-5 product-profile tree and active
`ctxhda.inf` package contain no Super X-Fi feature or parameter entry, while
the Linux CA0132 interface exposes none. Static files cannot reveal what the
Windows device repository returned at runtime, so final confirmation requires
opening Command with this AE-5 connected and recording whether its device UI
advertises the page.

The inspected device assembly has SHA-256
`e76ad407d5a2b7eeeb1049fa92d4b378ef03fdfddb8c7c963d8e07d8537eecdb`;
the UI framework has SHA-256
`06e7a61c95392fe76ec59d4a1ef1c5a8c465b07dd8c7d7b5256c2ce7ab109e3e`;
and the exact active INF has SHA-256
`36c88f2d7b39f9aa0a59ad2212c2ebe62e4b1443802b8d729e64934d49513b39`.
No Creative binary, decompiler output, cloud credential, or copied
implementation is committed. Super X-Fi remains outside Version 1 unless the
live exact-product result and a legal Linux mechanism both justify
reclassification.

## Windows equalizer bass and treble reference

The same installed Sound Blaster Command 3.5.10.0 tree was inspected offline
to determine whether the AE-5 has separate bass and treble equalizer
parameters. It does not. The AE-5 product EQ resource selects the framework's
`BaseEQPageTemplate`. That template renders the ten-band graphic equalizer
alongside Bass and Treble sliders, but the shared view model backs those two
sliders with existing band models: Bass is band index 1 and Treble is band
index 8 when the feature contains ten bands.

The exact shipped AE-5 Flat preset independently fixes that ordering at 31,
62, 125, 250, 500, 1000, 2000, 4000, 8000, and 16000 Hz. Command's Bass
slider therefore edits the stored 62 Hz value and its Treble slider edits the
stored 8000 Hz value. The preset schema contains only those ten bands and
preamp; it has no separate bass or treble scalar. Generic bass/treble feature
identifiers elsewhere in the multi-device bundle are not used by this AE-5 EQ
page path.

The native importer already maps those positions to `EQ Band1` and
`EQ Band8`, respectively. The Linux UI intentionally keeps one control per
underlying value instead of displaying duplicate aliases. This preserves the
Windows setting exactly while avoiding two widgets that edit the same ALSA
control.

| Evidence | SHA-256 |
|---|---|
| AE-5 root product assembly | `878881db6db73c1450931fe87557a45377e7a63ffb5298512c4203be1192657a` |
| Inherited product UI assembly (`Lang/Creative.SBConnect2.AE9.dll`) | `aacce22cbad477dd631bcdaa59f4798fbdf66c33bd559f45ab1ecbfcc82500c3` |
| AE-5 EQ resource entry | `6c7ac5f7fe78aa9e51bf578a3b4ab7c1ced50a040122435050f5f01b299c2d2e` |
| Shared UI framework | `06e7a61c95392fe76ec59d4a1ef1c5a8c465b07dd8c7d7b5256c2ce7ab109e3e` |
| English framework resources | `149ce5265cc80f3d64adf680bb81c72edcddeb7b218fb1fc487538fd1e80b4aa` |
| Base EQ resource entry | `acac94daae79d74cb5865475000402a40dbc7a9efbe70334ec92e273c298063c` |
| AE-5 Flat preset | `8ca9c9a9f30185cf623e86974496c606b4fd7be28a1e282505159a940e3f5b1c` |

No Windows code was executed. No Creative binary, BAML payload, decompiler
output, private preset value, or copied implementation is committed; only the
independent interoperability result and content hashes are retained.

## Windows equalizer gain write path

The exact installed Command 3.5.10.0 components and its existing runtime log
were inspected offline to determine whether Command compensates the outer EQ
bands before writing them. It does not in any managed layer:

1. `EqBandViewModel` fixes the ten positions at 31, 62, 125, 250, 500, 1000,
   2000, 4000, 8000, and 16000 Hz. Its action passes the displayed float and
   band index directly to `GraphicEqBandLevelEffectParameterId`.
2. `GraphicEQ.Commit` likewise passes each stored band float and index directly
   to that parameter.
3. `BasicIndexedEffectParameter` performs only configured-type and range
   checks, then gives the same value to the selected device repository.
4. The SoundCore key table maps the parameter to GraphicEQ parameter `2`.
   Generating indexed keys adds indices 0 through 9, and the embedded
   SoundCore enum names parameters 2 through 11 as Band0 through Band9 gain.
5. `SoundCoreRepository.SetValue<float>` marks the value as a SoundCore float,
   copies it without arithmetic, and calls `ISoundCore.SetParamValue`.

This is the path used by the exact card rather than only a generic
multi-device fallback. The existing Command log records 185 initializations
where `Speakers (2- Sound BlasterX AE-5)` bound as a SoundCore endpoint and
the AE5 product package was then selected.

Linux performs the equivalent value conversion explicitly. The CA0132 tuning
control maps ALSA level `24 + gain_db` through an IEEE-754 lookup and sends it
to output DSP module `0x96`; level 24 sends `0x00000000` (0.0), level 36 sends
`0x41400000` (+12.0), and level 12 sends `0xc1400000` (-12.0). Bands 0 through
9 use consecutive module requests 11 through 20.

The independent interoperability result is therefore that the Linux importer
preserves Command's ten band values and ordering without a per-band
compensation map. It does not prove that the proprietary native SoundCore
layer translates every parameter to the same DSP request, or that Windows
produces the same measured response at the 31 Hz and 16 kHz filter edges. The
prepared Windows capture or VFIO guest must still establish physical response
parity before the Version 1 acoustic gate is closed.

| Evidence | SHA-256 |
|---|---|
| Shared UI framework | `06e7a61c95392fe76ec59d4a1ef1c5a8c465b07dd8c7d7b5256c2ce7ab109e3e` |
| Device and SoundCore framework | `e76ad407d5a2b7eeeb1049fa92d4b378ef03fdfddb8c7c963d8e07d8537eecdb` |
| Profile framework | `a190130b146eb46e55a05ddfae0ead722fc45786cdba990ddc9ce1994ec319a1` |
| Existing Command runtime log | `bafe00375931359354816ff14f2d80f519c815a716b0bd5da250bce34dffb2a6` |

No Windows program was started and no hardware value was changed. The runtime
log and proprietary assemblies remain outside the repository; the repository
contains only the independently described interfaces, behavior, and hashes.

## Windows profile serialization defaults

A second scoped metadata and managed-method inspection distinguished five
serialized defaults from settings that can affect an AE-5. The root
`AE5ViewModel` inherits the implementation in the product's
`Lang/Creative.SBConnect2.AE9.dll` and sets its product name to `AE-5`.
Creative's profile library uses `SpeakerMethod` to choose between two Windows
speaker-routing APIs; it is not an acoustic profile parameter. Value zero is
the default path used by the inspected AE-5 files.

The same library reads or writes `Surround.Mode`, `DialogPlus.Mode`, and
`SVM.PlusMode` only when the device name is exactly `Katana`. The normal
`SVM.Mode` remains a separate cross-product setting and is mapped by the Linux
importer. Independently, all 34 shipped AE-5 profiles omit the three
Katana-only fields, all 34 profiles omit `SpeakerMethod`, and all 44 shipped
AE-5 EQ presets omit `SpeakerMethod`. The zero values serialized into the
active user copies therefore require no AE-5 control. The importer recognizes
only zero as a no-op and continues to report nonzero values for review.

| Evidence | SHA-256 |
|---|---|
| AE-5 root product assembly | `878881db6db73c1450931fe87557a45377e7a63ffb5298512c4203be1192657a` |
| Inherited product UI assembly | `aacce22cbad477dd631bcdaa59f4798fbdf66c33bd559f45ab1ecbfcc82500c3` |
| Creative profile library | `a190130b146eb46e55a05ddfae0ead722fc45786cdba990ddc9ce1994ec319a1` |

No Windows code was executed. No Creative binary, managed instruction dump,
profile content, or copied implementation is committed.

## Windows speaker and headphone preset metadata

The exact shared Command speaker-configuration view model exposes `Builtin`,
`Desktop`, `Bookshelf`, `Tower`, and `Custom` speaker categories. For
`Desktop`, the only processing action is to derive an X-Bass crossover from
the current speaker layout and write
`XBassCrossOverFrequencyEffectParameterId`. The persisted profile separately
stores its active `Bass.XOver` value, which the Linux importer already
handles. `SelectedSpeakerType=Desktop` therefore requires no second Linux
control; other categories remain warnings until their complete paths are
classified.

The neighboring named-headphone list is not a collection of EQ curves. All 33
AE-5 `SpeakerEqConfigs/*.cfg` files are 23–48-byte CRLF text records containing
only a display model and sort order. Command uses each filename as a preset
identifier. Its SoundCore backend maps that identifier to a driver-enumerated
SpeakerEQ image index, while its APO backend writes the selected config path
to the headphone SpeakerEQ endpoint property. No coefficient data is present
in the product text metadata.

The native importer now reads only the selected config's bounded `model` line
to make the unsupported warning intelligible. It never maps the file to the
ten-band EQ or sends it to hardware.

| Evidence | SHA-256 |
|---|---|
| Creative device-feature library | `e76ad407d5a2b7eeeb1049fa92d4b378ef03fdfddb8c7c963d8e07d8537eecdb` |
| Shared UI framework | `06e7a61c95392fe76ec59d4a1ef1c5a8c465b07dd8c7d7b5256c2ce7ab109e3e` |
| Selected headphone metadata config | `6d51c470242b53ea025f1a72de772f5a75d3c1c8e9f8021660b8709202023ed1` |
| Default no-tuning metadata config | `030693fc1dc35d32014db59bde6615028e2cdc42aab47ef9f1255c435f1b5b22` |
| Deterministic 33-config manifest | `a8741684a5078a556c107434ae82288d0f48c8e51d75600b62d78e55aff631e1` |

No Windows code was executed. No Creative binary, config content, coefficient,
managed instruction dump, or copied implementation is committed.

## Windows Bass Management selection

The exact Command 3.5.10.0 managed path explains why an enabled `Bass` object
can coexist with the active AE-5 5.1 profile. The profile binds that object to
the shared `BassMgmtXBassEnableAttributeId` feature. Its device implementation
uses the X-Bass enable key for headphones and speaker layouts without a
subwoofer, but uses the Bass Management enable key when the speaker channel
mask contains the Windows subwoofer bit (`0x8`). In that same case it redirects
the shared crossover property to the Bass Management crossover parameter.

The strength value is not a Bass Management parameter. Command deliberately
does not retrieve it for an external speaker layout with a subwoofer, while
the device feature continues to associate it with the ordinary X-Bass
strength parameter. It is therefore inactive when the Bass toggle selects Bass
Management.

This behavior has direct public-source Linux equivalents. CA0132 exposes
`Bass Redirection` and `Bass Redirection Crossover`, uses the same 10–1000 Hz
crossover table as X-Bass, enables redirection only for layouts with an LFE
channel, and suppresses X-Bass on those layouts. The active Windows importer
now maps 2.1, 4.1, and 5.1 Bass state to those controls, explicitly disables
X-Bass before the route change, and does not write the inactive strength.

| Evidence | SHA-256 |
|---|---|
| Creative profile library | `a190130b146eb46e55a05ddfae0ead722fc45786cdba990ddc9ce1994ec319a1` |
| Creative device-feature library | `e76ad407d5a2b7eeeb1049fa92d4b378ef03fdfddb8c7c963d8e07d8537eecdb` |
| Inherited AE-5 product UI assembly | `aacce22cbad477dd631bcdaa59f4798fbdf66c33bd559f45ab1ecbfcc82500c3` |
| ILSpy command-line package 10.1.1.8388 | `17a8baf571c889516bf8c268e3089156ba4cdfc2192a814206a3233581c9ae77` |

No Windows code was executed. No Creative binary, decompiler output, profile
content, or copied implementation is committed.

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
A later physical read-only probe received no response to the source-defined
request `60` either immediately after desktop firmware download or after the
complete AE-5 DSP setup. That negative result adds no lawful coefficient source
and does not change the firmware licence boundary.

## Additional public references

| Reference | Pinned revision | Licence and allowed role |
|---|---|---|
| [`Conmanx360/ca0132-tools`](https://github.com/Conmanx360/ca0132-tools) | [`6c1563c6ec07a18e9aa0a51a0a697c7a61de242d`](https://github.com/Conmanx360/ca0132-tools/commit/6c1563c6ec07a18e9aa0a51a0a697c7a61de242d) | No explicit licence was found. Do not copy, vendor, modify, or redistribute its code without permission. Its README warns that unsafe commands can lock the DSP/8051 and that its disassembler must not be used on DSP firmware. |
| [`Conmanx360/QemuHDADump`](https://github.com/Conmanx360/QemuHDADump) | [`82aa13e45c63ad2a0d1c411923b27f6ccbb48686`](https://github.com/Conmanx360/QemuHDADump/commit/82aa13e45c63ad2a0d1c411923b27f6ccbb48686) | No explicit licence was found. Treat it as a description of a possible HDA-verb observation technique; do not integrate or redistribute its code without permission. A VM trace does not replace physical AE-5 testing. |
| [OpenRGB AE-5 merge request `!2997`](https://gitlab.com/CalcProgrammer1/OpenRGB/-/merge_requests/2997) | Squash commit [`587a706f2873e7632ff835f9d8fda98d70e4d957`](https://gitlab.com/CalcProgrammer1/OpenRGB/-/commit/587a706f2873e7632ff835f9d8fda98d70e4d957); Linux prototype removal [`c75f0f6e502e07fae7c693a05f098077e1298a1d`](https://gitlab.com/CalcProgrammer1/OpenRGB/-/commit/c75f0f6e502e07fae7c693a05f098077e1298a1d) | `GPL-2.0-only`. The merged implementation uses a private Windows driver command. The removed prototype documents the five onboard APA102-compatible LEDs and their CA0113 GPIO data/clock pins. |

OpenRGB's removed Linux prototype mapped PCI region 2 through `/dev/mem` and
bit-banged offset `0x320`. That access model is intentionally not reused.
Offset `0x320` and GPIO pins 2 and 3 are already owned by the in-tree CA0132
driver through `ca0113_mmio_gpio_set()`, so the separate
[`ca0132-ae5-onboard-leds.patch`](../kernel/ca0132-ae5-onboard-leds.patch)
implements the public five-LED frame inside the kernel and exposes only
validated multicolor LED-class values.

The final OpenRGB implementation remains useful evidence for Windows device
matching and the existence of separate onboard and external-light commands,
but it does not disclose a Linux-safe external WS2812 strip protocol. The
external strip therefore remains deferred. No Creative binary, firmware, or
decompiler output was used for the onboard candidate.

## Proprietary Windows package boundary

Creative publishes Sound Blaster Command installers, but the
[official download agreement](https://support.creative.com/downloads/download.aspx?nDownloadId=100330)
restricts decompilation, disassembly, memory dumps, and reverse engineering
except where the agreement or applicable law expressly permits it. It directs
users seeking interoperability information to request it from Creative. No
installer was downloaded for this work. The later Direct Mode comparison used
only driver files already present in the user's Windows installation, kept all
analysis private, and retained only the independent device behavior listed
above. This project does not claim that the analysis changes Creative's licence
terms or supply legal advice.

The permitted evidence order for this project is:

1. verified GPL Linux source and history;
2. Creative's public documentation and user-exported JSON;
3. normal hardware controls, logs, audio measurements, and one-setting-at-a-time
   observation on hardware the user owns;
4. public GPL implementations such as OpenRGB within their actual scope;
5. a narrowly scoped Windows trace or static analysis only on user-provided or
   locally owned files, with no redistribution and only an independent
   interoperability specification retained.

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

This gate applies to the reported routing and audio-parity problems. The
separate What U Hear matrix did fail reproducibly: five counterbalanced
captures were signal-identical across level 90, level 0, and mute. Its
quirk-scoped candidate hides the two false mixer controls without claiming to
change routing, analog quality, or the loopback signal.

A separate read-only control audit found another self-contained CA0132 defect:
Wedge Angle
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
