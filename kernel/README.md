# CA0132 kernel patches

These patches are independent, reviewable Linux changes and diagnostic
experiments. The original four functional patches were loaded together on the
target AE-5 in a guarded KVM guest. The onboard-LED candidate was then added to
that same validated stack and exercised in a separate guarded cycle. The
Direct Mode candidate was prepared later from an independent behavior
specification, loaded on the physical card through VFIO, and exercised for
playback, bypass, output routing, failure handling, and normal-route
restoration.
The diagnostic SpeakerEQ probe was loaded separately in two earlier cycles and
produced the bounded negative result described below. None changes the running
host kernel merely by being present in this repository.

The Smart Volume resume candidate was validated later against the exact
running source and in the cardless kernel-build guest. It has not been loaded
on the physical card; its bare-metal suspend and acoustic acceptance gate is
documented in its section below.

The complete production, onboard-RGB, and Direct Mode stack has also been
packaged as a side-by-side host kernel and passed non-installing verification
plus a no-audio QEMU smoke boot. It has not been installed. The exact build,
artifact hashes, rollback path, and hard 20% first-boot gate are in
[`HOST_KERNEL_BUILD.md`](../docs/HOST_KERNEL_BUILD.md).

## Current upstream validation

On 2026-07-26, a direct `git ls-remote` check confirmed that the ALSA
maintainer tree's `for-next` head remained
`61471f29f3157f33a61194bf82b4a289cc03e1f1`. In separate clean worktrees at
that exact commit, the original four functional patches and the onboard-LED
candidate applied independently and in their tested series, and each resulting
tree passed `git diff --check`.

Strict `checkpatch.pl` reported no errors, warnings, or checks for the Wedge,
factory-EQ, and What U Hear patches. The DSP-image patch reported only the
expected question about adding new files; `get_maintainer.pl` maps both new
files to the existing ALSA maintainers and `linux-sound`/`linux-kernel` lists,
so no narrower MAINTAINERS entry is needed.

The combined production `ca0132.o` and parser-test object rebuilt with `W=1`
and warnings treated as errors. The `ca0132-dsp-image` KUnit suite then passed
all four cases under x86-64 KVM in 58.419 seconds. The LED-only upstream tree
also built `ca0132.o` with the same warning gate, and strict `checkpatch.pl`
reported zero findings for its 217 changed lines. No patch content needed
rebasing or correction.

The Direct Mode raw diff was regenerated on 2026-07-26 after a clean-tree
check found that its first context hunk unnecessarily depended on constants
added by the separate LED patch. The current patch applies directly to the
exact `for-next` base, after all five production/RGB patches, to clean Linux
6.18.40, and after the maintained 6.18.40 production/RGB stack. Strict
`checkpatch.pl` reported zero findings across 642 checked lines. Standalone
Direct Mode and complete production/RGB/Direct `ca0132.o` builds passed with
`W=1 KCFLAGS=-Werror`; the complete stack's DSP-image parser-test object did
too. Compared with the physically tested full-stack source, the only source
delta is relocation of the private
`AE5_DIRECT_MAX_ROUTER_ENTRIES` preprocessor definition; no executable C
statement changed. Physical playback, transition, routing, and busy gates
pass; three warm boots also pass; bare-metal power-management, host cold-boot,
and connected line-out gates remain open.

External submission has not been performed. Each submitting contributor must
personally add the Developer Certificate of Origin `Signed-off-by` line before
sending a patch to the maintainer recipients reported by
`scripts/get_maintainer.pl`.

## AE-5 Direct Mode

[`ca0132-ae5-direct-mode.patch`](ca0132-ae5-direct-mode.patch) adds an
AE-5-only `AE-5: Direct Mode Playback Switch`. A physical passthrough cycle
proved 48/96 kHz stereo output, DSP bypass, coherent Headphone/Speakers
selection, and restoration of the normal DSP route without the previously
required ALSA toggle. It remains a candidate pending host cold-boot,
bare-metal suspend/resume, and connected line-out acceptance; three warm guest
boots already pass.

The behavior-level interoperability result is:

- Direct ON stops internal stream `0x18`;
- Direct OFF restores stream `0x05` from `0x43` to `0x00`, stream `0x18` from
  `0x09` to `0xd0`, the `0xd0` 96 kHz connection, six stream channels, stream
  enable, and ASI parameter `7`.

The stream routes, `0xd0` rate, channel count, and enable state independently
match the driver's existing `ae5_post_dsp_stream_setup()` path. That startup
path writes ASI value `4`; the reconstructed Windows Direct-to-normal
transition writes `7`, and physical restoration now validates that
transition-specific value. The patch uses the existing ChipIO helpers under
`chipio_mutex`, snapshots and restores the exact playback-router entries that
Direct PCM overwrites, reads stream state back after transitions, and changes
the cached mode only after successful readback.

To avoid reproducing transition-time distortion or loss of sound, the control
returns `-EBUSY` whenever analog playback is open. A Direct Mode PCM open is
limited to stereo 48 or 96 kHz, and the driver reports no DSP playback delay
while bypassed. The first subsequent normal prepare reapplies the cached
processing edge and rebinds the HDA converter. Codec reinitialization restores
the requested direct state. The patch does not expand sample-width support.

The Rust application detects the control dynamically. It temporarily suspends
only the AE-5 PipeWire sink, waits for analog PCM to close, writes and verifies
the control, and resumes the sink. The GTK page disables DSP controls that
cannot affect Direct Mode, including PCM and hardware output levels, while
retaining output, gain, and DAC-filter controls. A real-card regression fixed
route validation so an already-enabled X-Bass no longer blocks safe output
selection while Direct Mode is active.

Suggested upstream commit message:

```text
ALSA: hda/ca0132: Add AE-5 Direct Mode control

The AE-5 can bypass its DSP playback path through a separate stream 0x14
route while ChipIO stream 0x18 is stopped. Direct PCM setup changes rate,
clock, and playback-router state. Returning to normal playback requires
restoring the overwritten router entries, streams 0x05 and 0x18, the 96 kHz
ASI connection, cached processing state, and the HDA converter endpoint.

Expose the route as an AE-5-only playback switch. Reject transitions while
the analog PCM is open, constrain direct playback to stereo 48/96 kHz,
verify stream state, and restore the selected mode after codec
reinitialization. Rebind the endpoint on the first normal prepare.
```

The submitting contributor must add their own Developer Certificate of Origin
`Signed-off-by` line.

### Validation

The exact patch is based on `sound.git` `for-next`
`61471f29f3157f33a61194bf82b4a289cc03e1f1`. It passes `git diff --check`,
strict `checkpatch.pl` with no findings across 642 checked lines, direct
application to the pristine base, composition after the production/RGB stack,
and standalone plus complete-stack warnings-as-errors object builds. The
current patch SHA-256 is
`49e571c51b035d4feb453ccabb9c42e8b28b699ca1b00ebac9dc34e7d6cbf23a`.
The physical test used the predecessor raw diff whose only source-level
difference placed the same private array-size definition beside the LED
constants.
The physical AE-5 passed exact 48/96 kHz stereo negotiation, S16/S32 playback,
open-PCM rejection, DSP-bypass comparison, normal restoration, ten repeated
cycles, and Headphone/Speakers routing with at least 35.5 dB acoustic
separation. The complete mixer state returned exactly and the kernel log
remained clean. Three warm guest boots then loaded the exact signed module and
each completed Direct and normal PCM with an unchanged safe mixer hash. All
playback fixtures were at approximately 5% digital amplitude; future tests are
capped at 20% and must pass the repository's non-mutating
`playback-preflight` against the exact fixture first. A post-Direct What U Hear
capture and final VFIO shutdown then returned the exact guest and host mixer
hashes, host default nodes, profile, routes, services, PCI driver, and inactive
VM configuration.

The full independent evidence, application sequencing, supported-format
boundary, passed gates, and remaining power/boot/line-out matrix are in
[`DIRECT_MODE_INVESTIGATION.md`](../docs/DIRECT_MODE_INVESTIGATION.md).

## AE-5 onboard multicolor LEDs

[`ca0132-ae5-onboard-leds.patch`](ca0132-ae5-onboard-leds.patch) exposes the
five onboard AE-5 LEDs through Linux's standard multicolor LED class. It does
not use `/dev/mem`, map PCI resources into userspace, or implement the separate
external-strip protocol.

The public GPL OpenRGB history documents five internal APA102-compatible LEDs
clocked through the AE-5's existing GPIO data and clock pins. The CA0132 driver
already owns and maps the required PCI region and already provides
`ca0113_mmio_gpio_set()` for those pins. The patch therefore keeps the complete
transaction in the driver:

- five devices named `hdaudioC*D*:rgb:ae5-1` through `ae5-5`;
- standard `brightness`, `multi_index`, and `multi_intensity` attributes;
- complete five-LED frames, with the existing `chipio_mutex` serializing MMIO;
- HDA power references around userspace writes;
- a full color cache restored when the codec is reinitialized;
- LED unregistration before the PCI region is unmapped.

The first write initializes every cached LED to the selected color. This
prevents a partial update from transmitting uninitialized colors; later writes
can address each LED independently.

### Validation

The exact patch is based on ALSA `for-next`
`61471f29f3157f33a61194bf82b4a289cc03e1f1`. It passes `git diff --check`,
strict `checkpatch.pl` with zero findings, and a `W=1 KCFLAGS=-Werror`
`sound/hda/codecs/ca0132.o` build.

It was also backported onto the physically validated Linux `6.18.40` stack.
The complete `6.18.40-ae5-lts-rgb+` image and modules built successfully, and
a card-less one-shot VM boot loaded both `snd-hda-codec-ca0132` and
`led-class-multicolor` with no failed unit.

A managed physical passthrough boot then:

- received the exact `1102:0012/1102:0051` device and initialized the DSP once;
- registered exactly five RGB devices with `red green blue` channel order;
- accepted solid red, green, and blue frames, five independent RGB values, and
  an individual brightness off/on cycle;
- retained the exact 72-control guest mixer SHA-256 throughout;
- produced no CA0132, HDA, timeout, lockup, or kernel warning.

Visible color confirmation is still required before calling the protocol
fully accepted. A second physical cycle installed the application RPM's exact
udev rule, which made only the five `brightness` and five `multi_intensity`
attributes writable. A non-`audio` user then applied, persisted, restored, and
failure-tested unified and independent colors through the Rust backend without
`sudo`; RPM removal returned all ten values to mode `0644`. The complete mixer
hash remained unchanged. The external WS2812 strip remains unsupported because
the verified public implementation reaches it only through a private Windows
driver command.

A third physical cycle on the same production RGB kernel exercised only the
existing audio path. External microphone captures tracked two advertised
five-step Master changes within `0.66 dB`, Master mute reached the quiet
baseline, a confirmed Front mute suppressed the fixture by more than 34 dB,
and all three guarded headphone-gain choices produced distinct external
levels. The guest mixer restored exactly, the DSP initialized once, and the
kernel remained warning-free. An 18 kHz microphone probe was below a useful
signal-to-noise ratio, so DAC-filter validation still requires an attenuated
electrical capture rather than a driver change based on acoustic noise.

## AE-5 What U Hear mixer controls

[`ca0132-ae5-hide-ineffective-wuh-controls.patch`](ca0132-ae5-hide-ineffective-wuh-controls.patch)
stops advertising volume and mute controls that do not affect the AE-5
`CA0132 What U Hear` PCM.

The controls are ordinary HDA input-amplifier elements on node `0x0a`. The
physical card accepts and reports level and mute writes, including raw values
`0x5a`, `0x00`, and `0x80`. A counterbalanced direct-ALSA fixture nevertheless
measured exactly `0.022104` RMS at level 90, level 0, muted, and level 90
again. The DSP loopback bypasses this amplifier.

The patch adds an AE-5-only mixer table without those two elements. It retains
the What U Hear PCM and every analog capture control, and leaves the Sound
Blaster Z, ZxR, Recon3D, Recon3Di, AE-7, and generic CA0132 tables unchanged.
The measurements and ACP input-route work are in
[`RECORDING_MIXER_INVESTIGATION.md`](../docs/RECORDING_MIXER_INVESTIGATION.md).

Suggested upstream commit message:

```text
ALSA: hda/ca0132: Hide ineffective AE-5 What U Hear controls

The AE-5 inherits standard HDA volume and mute controls for the What U
Hear converter from desktop_mixer. Node 0x0a accepts and reports the
amplifier writes, but its DSP loopback stream bypasses that amplifier.
Level 90, level 0, and mute therefore produce the same captured signal.

Use an AE-5-specific mixer table without those two ineffective controls.
Keep the What U Hear PCM and all analog capture controls unchanged.

Fixes: 88268ce8a64e ("ALSA: hda/ca0132 - Set AE-5 bools and select mixer.")
Cc: stable@vger.kernel.org
```

The submitting contributor must add their own Developer Certificate of Origin
`Signed-off-by` line.

### Validation

The patch applies cleanly to the exact running Linux `v7.1.4` source and to
the mutually identical CA0132 files in Linux `master` at `48a5a7ab8d6a`, ALSA
`master` at `f5657cb8480c`, and ALSA `for-next` at `61471f29f315`.

Both the running source and current upstream source compiled as external
modules against the matching Nobara kernel-devel tree with `W=1` and warnings
treated as errors. Strict `checkpatch.pl` reports no errors, warnings, or
checks. The resulting objects contain separate `ae5_mixer` and
`desktop_mixer` tables. No module was loaded.

An authorized patched-kernel boot must verify:

- the What U Hear PCM still captures the 997 Hz fixture;
- only its ineffective volume and mute controls are absent;
- analog input selection, capture volume, mute, and boost still work;
- normal playback, headphone/speaker routing, suspend/resume, and shutdown
  remain clean;
- no CA0132, HDA, ALSA, codec, or DSP warning appears.

The integrated physical validation below now covers the What U Hear fixture,
normal headphone playback, Front-muted negative control, three warm guest
reboots, 50 speaker/headphone route transitions, clean shutdown, and exact
guest/host restoration. Analog input controls, physical speaker/line-out and
digital playback, and suspend/resume remain. The maintained-kernel repetition
is recorded below.

## Smart Volume restoration after DSP reload

[`ca0132-restore-smart-volume-resume.patch`](ca0132-restore-smart-volume-resume.patch)
fixes a stale-cache defect after a suspend or other codec reinitialization that
loses the downloaded DSP state.

`ae5_setup_defaults()` writes the Smart Volume level and mode defaults back to
the DSP during every reload: request 5 receives level 74 and request 6 receives
Normal. The cached `fx_ctl_val[]` level and `smart_volume_setting` mode survive
that path, however, and the existing resume tail replays only the output-effect
switches. Both ALSA getters read those surviving caches. The mixer and
application can therefore report the pre-suspend level and mode while the DSP
is actually processing with 74/Normal.

The candidate replays the cached Smart Volume level and mode before restoring
the output-effect switches. It also makes the shared output-effect level write
return the DSP error, updates the level or mode cache only after a successful
DSP write, and reports a successful changed ALSA control as changed. A failed
Smart Volume restore leaves `dsp_reload` set so a later initialization can
retry.

This patch does not reset Smart Volume's adaptive history. There is no
source-backed DSP request for that operation. Cycling global `Enable OutFX`
also reselects the output route, so the driver and application must not use
that audible transition as an automatic reset.

### Static and build validation

The exact patch has SHA-256
`144fa9c2d4766f502fd67df82eac93576723f2655b173420cf8af545ca3905d9`.
It applies and reverses cleanly against the exact Linux stable `v7.1.4`
CA0132 source, SHA-256
`7b61bcb02c4079b9ca6c82cde3147e95706cdbe958324ae383e7875d9a33a4f0`,
and produces the reviewed source SHA-256
`c0e4f63e79b74ac0833a185adfbbf44d0feb23b965c1418f8607aed918b861e1`.
It also applies cleanly to the recorded current-upstream CA0132 snapshot,
SHA-256
`95a23cdef3504d67762b35d3e0fcedf31651233f08477c4dcf56bd436c2552cb`.

Strict `checkpatch.pl` reports zero errors, warnings, or checks across 156
lines. Applied after the maintained Linux 6.18.40 AE-5 stack in the cardless
build guest, `sound/hda/codecs/ca0132.o` compiles with
`W=1 KCFLAGS=-Werror`. The patched integrated source has SHA-256
`a9a6373512886ed98a76ad2d26002002f13edfc223808fbb6f0de7bfd1333ac0`;
the resulting object has SHA-256
`e27add747d76de41de6c582eeef6114f7b884dcf74f92d3947aa691328291a38`.
No module was loaded and the physical card remained on the host driver.

### Remaining physical acceptance

The fix is not accepted until a guarded bare-metal test:

- saves the exact mixer and PipeWire baseline with no open PCM;
- selects non-default Smart Volume level and mode values;
- proves the suspend caused a real DSP reload rather than a shallow no-op;
- compares direct-ALSA What U Hear captures before and after resume, with the
  playback path held at or below the repository's 20% safety ceiling;
- distinguishes restored non-default DSP behavior from the 74/Normal default,
  instead of trusting cache-backed ALSA readback alone;
- repeats the matrix in counterbalanced order because Smart Volume is stateful;
- restores the exact mixer, routes, defaults, and zero-PCM state and checks the
  kernel log for CA0132, HDA, DSP, timeout, or warning messages.

Adaptive Normal/Night repeatability and matched Windows measurements remain a
separate parity investigation even if this resume test passes.

## Wedge Angle default

`ca0132-wedge-angle-default.patch` fixes an invalid ALSA control value present
in Linux `v7.1.4` and upstream `master` at `48a5a7ab8d6a`.
It was rechecked on 2026-07-26 against the official Linux `master` at
`3dab139d4795` and `sound.git` `for-next` at `61471f29f315`; both still contain
the invalid cached value.

The driver exposes Wedge Angle as a logical integer from 20 to 180 degrees.
`voice_focus_ctl_put()` subtracts 20 only when converting that public value to
an index in `voice_focus_vals_lookup[]`, while `tuning_ctl_get()` returns the
cached public value unchanged. Initialization nevertheless caches the lookup
index `10`, so a fresh device reports a value below its own minimum:

```text
numid=46,iface=MIXER,name='Wedge Angle Capture Volume'
  ; type=INTEGER,values=1,min=20,max=180,step=1
  : values=10
```

The DSP default `0x41F00000` is 30.0, so the cached public value must be `30`.
The patch changes only that initialization value.

Suggested upstream commit message:

```text
ALSA: hda/ca0132: Fix Wedge Angle control default

The Wedge Angle control advertises a range of 20 through 180 degrees,
and its put callback subtracts 20 only to obtain the DSP lookup-table
index. The get callback returns cur_ctl_vals[] directly.

Initialize cur_ctl_vals[] to the logical default of 30 degrees instead
of the lookup index 10. Otherwise userspace reads a value below the
declared minimum until the control is written once.

Fixes: 44f0c9782cc6 ("ALSA: hda/ca0132: Add tuning controls")
Cc: stable@vger.kernel.org
```

The submitting contributor must add their own Developer Certificate of Origin
`Signed-off-by` line. The repository does not invent one on their behalf.

### Validation

Apply the patch to a current Linux source tree:

```sh
git apply --check /path/to/ae5-linux-control/kernel/ca0132-wedge-angle-default.patch
git apply /path/to/ae5-linux-control/kernel/ca0132-wedge-angle-default.patch
```

Build and boot the patched kernel or module, then verify before writing the
control:

```sh
amixer -c <AE5_CARD> cget "iface=MIXER,name='Wedge Angle Capture Volume'"
```

The value must be `30`, remain within the advertised `20..180` range, and
setting 20, 30, and 180 must read back exactly. Voice Focus recording tests
must show no new kernel warning or DSP timeout.

On 2026-07-24, the patch was applied to `sound.git` `for-next` at
`61471f29f315` and built inside the Fedora 44 KVM guest. The resulting
`7.2.0-rc2-ae5-wedge+` kernel booted from Btrfs with EFI, VirtIO networking,
and SSH intact. Its matching `snd-hda-codec-ca0132` module loaded successfully,
and no systemd unit failed. A later integrated physical boot, recorded below,
proved the initial control value is `30`; a third physical cycle proved exact
readback at `20`, `30`, and `180` while Voice Focus was enabled. Voice Focus
recording behavior remains untested.

## Factory EQ preset control cache

`ca0132-eq-preset-control-cache.patch` fixes stale individual EQ controls
after selecting a factory preset. The preset callback writes DSP requests 10
through 20 directly and updates only `eq_preset_val`; each `EQ Band0` through
`EQ Band9` get callback instead returns `cur_ctl_vals[]`. On the physical AE-5,
selecting Acoustic changed the measured frequency response by up to 2.49 dB
while all ten band controls continued to report `24` (0 dB).

Some factory values are fractional, such as 1.1, 3.1, and -1.2 dB, while the
public controls expose whole-dB steps. The patch keeps sending the original
exact float bit patterns to the DSP, caches each preset's nearest
representable whole-dB value, and notifies ALSA clients only when a displayed
band value changes. A same-value band write remains a no-op, so saving and
reapplying the rounded public values does not replace the exact factory DSP
curve.

Suggested upstream commit message:

```text
ALSA: hda/ca0132: Synchronize EQ controls after preset changes

The factory EQ preset callback writes requests 10 through 20 directly to
the DSP but updates only eq_preset_val. The individual EQ controls return
cur_ctl_vals[], so they continue to expose the previous band values after
a preset change. Saving that state and restoring it can then overwrite the
factory curve with stale values.

Cache the nearest values representable by the whole-dB band controls after
all preset requests succeed, and notify userspace for each displayed value
that changed. Keep the preset's original fractional values in the DSP.
```

The submitting contributor must add their own Developer Certificate of Origin
`Signed-off-by` line.

### Validation

The patch applies cleanly to the exact running `v7.1.4` source and to the
identical CA0132 file currently present in Linux `master`, `sound.git`
`master`, and `sound.git` `for-next`. Both the running source and current
upstream source compiled `sound/hda/codecs/ca0132.o` with `W=1`. The compile
used a separate temporary source/output tree; no module was loaded and no
running kernel file was changed. `checkpatch.pl --strict` reports no errors,
warnings, or checks.

After an authorized patched-kernel boot, select every factory preset and
verify that:

- every reported band equals the nearest whole dB to its preset table value;
- ALSA value events are emitted for bands whose displayed value changed;
- writing each reported value back leaves the measured factory response
  unchanged;
- saving and applying a profile preserves that response from any prior custom
  EQ state;
- Flat restores all ten bands to `24`;
- no CA0132, HDA, ALSA, or DSP warning appears.

AE-5 Control also protects unpatched kernels: factory-preset profiles omit
stale band values, legacy profiles ignore them, and custom band editing
requires selecting Flat first.

## Bounded DSP fast-load images

[`ca0132-dsp-image-bounds.patch`](ca0132-dsp-image-bounds.patch) hardens the
CA0132 DSP fast-load parser independently of any proposed headphone-tuning
support. It is based on the ALSA maintainer tree `for-next` commit
[`61471f29f315`](https://git.kernel.org/pub/scm/linux/kernel/git/tiwai/sound.git/commit/?h=for-next&id=61471f29f3157f33a61194bf82b4a289cc03e1f1).

The existing parser advances through variable-size segments using each
firmware-provided word count, but its caller does not provide the firmware
length. A truncated image can therefore move traversal beyond the
`request_firmware()` buffer. Image validation also occurs only after DSP
transfer setup has begun.

The patch passes `fw_entry->size` into `dspload_image()` and validates the
complete immutable image before `dsp_reset()` or transfer setup. It rejects:

- missing, truncated, oversized, or overflowing segments;
- invalid segment or terminator magic;
- images with no data segment or no in-bounds terminator;
- nonzero bytes after the terminator, while retaining the zero padding used by
  the distributed Recon3D firmware;
- odd or consecutive HCI programming lists;
- relocation overflow and targets outside the CA0132 X, Y, or microcode RAM
  ranges.

The transfer path retains its existing checks, and the HCI writer now rejects
an odd word count defensively as well.

Suggested upstream commit message:

```text
ALSA: hda/ca0132: Validate DSP image bounds before loading

The CA0132 fast-load parser advances through variable-length segments using
the word count stored in each segment, without knowing the size of the
request_firmware() buffer. A truncated or malformed image can therefore move
segment traversal beyond the firmware data.

Pass the firmware size to dspload_image() and validate the complete image
before resetting or programming the DSP. Bound every header and payload,
require an in-bounds terminator, validate HCI list pairs and DSP address
ranges, and reject relocation arithmetic overflow. Permit only zero padding
after the terminator for compatibility with the distributed Recon3D image.

Add KUnit coverage for valid multi-segment images and malformed metadata.
```

The submitting contributor must add their own Developer Certificate of Origin
`Signed-off-by` line.

### Validation

The recorded source and test environment was:

- `sound.git` `for-next` at `61471f29f315`;
- x86-64 GCC kernel build with the production CA0132 codec and DSP enabled;
- KUnit under QEMU 10.2.2;
- no module load, reboot, firmware upload, or live hardware write.

The patch passed:

- `git apply --check` and `git diff --check` on a clean worktree at the pinned
  base;
- production `ca0132.o` and parser-test compilation with `W=1` and no warning;
- all four `ca0132-dsp-image` KUnit cases;
- a metadata-only compatibility scan of the installed `ctefx.bin`,
  `ctefx-desktop.bin`, `ctefx-r3di.bin`, and `ctspeq.bin`.

After the host QEMU packages were installed on 2026-07-24, the pinned tree and
patch were rebuilt and booted again with KVM. All four tests passed; KUnit
reported 62.434 seconds total, including 56.485 seconds of compilation and
0.487 seconds running the test kernel.

The `ctspeq.bin` result proves only that its outer fast-load structure is
accepted. It is not evidence that the Chromebook SpeakerEQ data is suitable
for the AE-5, and the patch never requests or loads it.

`checkpatch.pl --strict` reports no errors or code-style checks. Its sole
warning asks whether new files require a MAINTAINERS update; the existing ALSA
sound patterns already resolve the new files to the same maintainers and
mailing lists as `ca0132.c`.

Apply and rerun the build-only validation with:

```sh
git apply --check /path/to/ae5-linux-control/kernel/ca0132-dsp-image-bounds.patch
git apply /path/to/ae5-linux-control/kernel/ca0132-dsp-image-bounds.patch

tools/testing/kunit/kunit.py run \
  --arch=x86_64 \
  --kconfig_add CONFIG_PCI=y \
  --kconfig_add CONFIG_SOUND=y \
  --kconfig_add CONFIG_SND=y \
  --kconfig_add CONFIG_SND_HDA_INTEL=y \
  --kconfig_add CONFIG_SND_HDA_CODEC_CA0132=y \
  --kconfig_add CONFIG_SND_HDA_CODEC_CA0132_DSP=y \
  --kconfig_add CONFIG_SND_HDA_CODEC_CA0132_DSP_IMAGE_KUNIT_TEST=y \
  ca0132-dsp-image
```

QEMU must be installed for a non-UML architecture. This validates malformed
parser inputs and compilation. The later integrated physical boot recorded
below proved that the distributed AE-5 firmware is accepted and reaches
`ca0132 DSP downloaded and running`; playback and deliberately malformed
firmware on the physical card remain out of scope.

## Integrated no-device kernel validation

On 2026-07-24, the Wedge Angle, factory EQ cache, AE-5 What U Hear, and DSP
image bounds patches were applied together to `sound.git` `for-next` commit
`61471f29f315`. The diagnostic SpeakerEQ address probe was not included. The
combined source passed `git diff --check` and built as
`7.2.0-rc2-ae5-integrated+`, including the production CA0132 codec module.
The x86 instruction decoder passed 8,073,002 instructions, and its random
instruction test passed 1,000,000 cases with no error.

The first no-device boot exposed a guest configuration dependency:
Fedora's `snd-pcm` modprobe policy loads `snd-seq`, but the configuration
derived by `localmodconfig` did not include the sequencer. The candidate was
rebuilt with `CONFIG_SND_SEQUENCER=m` and `CONFIG_SND_SEQ_DEVICE=m`. The final
kernel then booted from Btrfs under KVM, and both
`snd-hda-codec-ca0132` and `snd-seq` loaded with zero failed systemd units.

The exact powered-off guest state was flattened into a standalone image and
that image itself passed a second boot/module smoke test and `qemu-img check`.
It had no emulated audio or passed-through PCI device. This validates the
combined build, kernel boot, and module dependency path. Physical
initialization, reset recovery, headphone playback, and What U Hear capture
were subsequently validated below; the remaining route and power-management
gates are stated there.

## Integrated physical AE-5 validation

On 2026-07-24 and 2026-07-25, libvirt passed the isolated physical
`1102:0012/1102:0051` function to the powered-off system guest for five
managed test cycles. The guest booted `7.2.0-rc2-ae5-integrated+`, bound the
card at `0000:07:00.0` to `snd_hda_intel`, and reported
`ca0132 DSP downloaded and running`.

The first cycle was read-only:

- the card exposed 72 ALSA controls and 46 simple controls;
- Wedge Angle reported `30` inside its advertised `20..180` range before any
  write;
- the `CA0132 What U Hear` capture PCM remained present as device 2;
- the ineffective What U Hear volume/mute simple control was absent;
- Flat and all ten EQ band controls initialized to raw value `24`;
- no CA0132, HDA, codec, DSP, firmware, or timeout warning appeared, and no
  systemd unit failed.

The second cycle selected every factory EQ preset and compared all ten
reported band values with the cache table added by the patch:

| Preset | Band0 through Band9 raw values |
|---|---|
| Flat | 24 24 24 24 24 24 24 24 24 24 |
| Acoustic | 24 25 26 24 24 24 24 26 26 26 |
| Classical | 24 30 30 27 24 24 24 24 27 27 |
| Country | 23 24 25 25 25 24 24 26 27 28 |
| Dance | 23 26 27 28 23 23 24 24 28 28 |
| Jazz | 24 24 25 28 28 28 24 25 27 27 |
| New Age | 24 26 26 24 24 24 25 26 26 26 |
| Pop | 22 24 26 26 24 23 23 24 27 30 |
| Rock | 23 23 25 26 23 23 24 24 28 28 |
| Vocal | 22 23 23 24 27 28 27 24 24 25 |

Every vector matched. An ALSA monitor observed value events for Acoustic's
five changed bands—Band1, Band2, Band7, Band8, and Band9—plus the preset
control, and the same events when returning to Flat. Flat restored the
complete guest mixer SHA-256
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`
exactly after both the full matrix and notification check. No matching kernel
warning appeared.

The third cycle exercised Wedge Angle while Voice Focus was enabled. Its
initial raw and simple-mixer values were both `30`; writes of `20`, `30`, and
`180` read back exactly through both ALSA APIs. Returning to `30` restored the
same complete guest mixer hash. No PCM stream was open, no systemd unit failed,
and no CA0132, DSP, or timeout warning appeared.

The fourth cycle exercised real playback and capture with a two-second,
48 kHz, 24-bit stereo 997 Hz fixture at -18 dBFS. Headphone output used Low
gain, output effects off, and a bounded Master ramp. At Master 65, a host
Fifine capture measured mean 987–1007 Hz power 21.75 dB above baseline and
19.59 dB above the same stream with Front muted. A second positive capture
repeated within 1.04 dB. Front remained off during the negative stream and was
restored immediately afterward.

With Front muted to keep the headphones silent, the retained
`CA0132 What U Hear` PCM captured the same fixture at 48 kHz, signed 32-bit
stereo. The capture measured -21.26 dBFS RMS with its strongest analyzed bin
at 996.09375 Hz, while the ineffective mixer controls remained absent. The
complete guest mixer returned exactly to
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`,
no unit failed, and no matching driver warning appeared.

The fifth cycle rebooted the running guest three times while it owned the
physical card. Every new boot ID retained the integrated kernel, initialized
the DSP exactly once, exposed 72 controls and 46 simple controls, restored
Wedge Angle `30` and Flat EQ, kept the What U Hear PCM while omitting only its
ineffective controls, and reproduced the complete guest mixer hash. No
relevant warning or failed unit appeared.

After the reboots, 50 alternating speaker/headphone selections all produced
the expected codec route. Headphone enabled output pin `0x11` and disabled
`0x0b`, `0x0f`, and `0x10`; Speakers did the inverse. No PCM was open during
the test. The final Speakers selection reproduced the complete mixer hash,
with no CA0132, HDA, DSP, firmware, reset-failure, or timeout warning.

Each guest shutdown returned the card automatically to the host
`snd_hda_intel` driver in about two seconds. Before each handoff the host audio
services had no open stream and were stopped. After each handoff, the complete
saved Creative ALSA state matched without a fallback restore, the card-scoped
WirePlumber headphone port returned, and VFIO preflight passed again. The
hostdev was removed from the powered-off domain after each cycle. After the
fifth cycle, the powered-off qcow2 passed `qemu-img check` with SHA-256
`d7ee6ed48b3ba5800e5c93576fdbbec76bbe0eb81d2708c59dd600058262a664`.

This evidence does not yet cover Voice Focus recording, analog input,
speaker/line-out or digital playback, suspend/resume, or repeated cold-start
acceptance.

## Maintained Linux 6.18 LTS validation

The same functional stack, plus upstream auto-detect commits `778031e1658d`
and `6fd9f6e870ea`, was backported to Linux `6.18.40` stable commit
`221fc2f4d0eda59d02af2e751a9282fa013a8e97`. The exact application order and
the two-context DSP adapter are in
[`backports/6.18/README.md`](backports/6.18/README.md).

The resulting `6.18.40-ae5-lts+` kernel passed strict production/parser object
builds, all four parser KUnit cases, and no-device boots in both libvirt
guests. One managed physical cycle then reproduced the expected 72/46 control
counts, Wedge `30`, Flat EQ vector, retained What U Hear PCM, hidden
ineffective controls, and complete mixer hash.

With auto-detect enabled, a manual Headphone selection disabled auto-detect
and activated only pin `0x11`; Speakers activated `0x0b`, `0x0f`, and `0x10`.
The packaged CLI also performed and restored a Wedge `20` write. Guest
shutdown returned the card to host `snd_hda_intel` in about two seconds with
byte-identical raw ALSA state and no relevant warning. Full evidence is in
[`LTS_KERNEL_VALIDATION.md`](../docs/LTS_KERNEL_VALIDATION.md).

## Read-only SpeakerEQ address probe

[`ca0132-speaker-eq-address-probe.patch`](ca0132-speaker-eq-address-probe.patch)
implements the next bounded headphone-tuning experiment. It is diagnostic
instrumentation, not a functional driver fix and not a SpeakerEQ loader.

After the card-specific DSP setup has completed, the patch sends the existing
`MASTERCONTROL_QUERY_SPEAKER_EQ_ADDRESS` request as one SCP `GET`. It runs only
for the AE-5/AE-5 Plus and AE-7 quirks, provides a four-byte reply buffer, and
accepts only an exact four-byte response. Success, DSP error, and unexpected
response length are reported through `codec_dbg()`.

The probe does not request another firmware file, upload coefficients, expose
an ALSA control, enable `SPEAKER_TUNING_USE_SPEAKER_EQ`, or change the
reported address. The result establishes only whether this DSP request is
implemented and stable on the target card. It cannot identify the meaning,
size, or safe contents of the returned memory region.

### Static and build validation

The patch was developed against Linux stable `v7.1.4`, whose `ca0132.c`
SHA-256 is
`7b61bcb02c4079b9ca6c82cde3147e95706cdbe958324ae383e7875d9a33a4f0`.
Both the unmodified and patched source compiled as external modules against
the exact running Nobara
`7.1.4-200.nobara.fc44.x86_64` kernel-devel tree with `W=1` and warnings
treated as errors. The patch also applies cleanly to Linux `master` at
`48a5a7ab8d6a` and ALSA `for-next` at `61471f29f315`; their identical CA0132
source compiled with the patch in the same isolated harness. Strict
`checkpatch.pl` reports no errors, warnings, or checks.

Apply it to a matching source tree with:

```sh
git apply --check \
  /path/to/ae5-linux-control/kernel/ca0132-speaker-eq-address-probe.patch
git apply \
  /path/to/ae5-linux-control/kernel/ca0132-speaker-eq-address-probe.patch
```

The hardware session used a separate boot entry and enabled the CA0132
dynamic-debug call sites before module initialization with:

```text
snd_hda_codec_ca0132.dyndbg=+p
```

### Physical AE-5 result: no reply

Two Linux `6.18.40` probe kernels were built from the same full LTS patch stack
that had already passed KUnit, no-device boot, and physical-card validation:

- `6.18.40-ae5-lts-speq+` sent the query immediately after firmware download;
- `6.18.40-ae5-lts-speq-late+` used the repository patch's current placement,
  after `ae5_setup_defaults()` had started the DSP audio streams and initialized
  the AE-5 output modules.

Each kernel first passed a no-device boot with zero failed units, matching
module `vermagic`, active dynamic debug, and no kernel warning. With the exact
`1102:0012/1102:0051` function passed through, each boot initialized the DSP
once and emitted exactly one probe result:

```text
dspio_scp: send scp msg failed
SpeakerEQ address query failed: -5
```

The late placement therefore ruled out the simple hypothesis that the query
ran before AE-5 setup was complete. It did not prove whether request `60` is
absent from `ctefx-desktop.bin` or whether undocumented request fields differ
from the public-source assumption. No source-backed alternate request shape is
available, so the experiment stopped without guessing.

The failed GET did not otherwise disturb the card. The late-probe boot retained
72 ALSA controls, 46 simple controls, Wedge `30`, Flat with all ten bands at raw
`24`, the What U Hear PCM without ineffective controls, one DSP initialization,
zero failed units, and mixer SHA-256
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`.
With Front muted, What U Hear captured a two-second 997 Hz direct-ALSA fixture
at 48 kHz signed 32-bit stereo; its strongest bin was `996.09375 Hz` and its
RMS amplitude was `0.048654`. Restoring the saved guest state reproduced the
same mixer hash.

Both clean shutdowns returned the card to host `snd_hda_intel`. Raw ALSA state
was byte-identical, all 47 application controls were identical, WirePlumber
defaults and routes were byte-identical, stream properties were canonically
identical, the AE-5/Fifine defaults returned, and the host mixer SHA-256 was
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`.
Both guests were off, the inactive domain had no `hostdev`, and VFIO preflight
passed afterward.

The success acceptance gate was not met: there is no address to compare across
boots. The test therefore did not proceed to three repeated query boots,
another 50-switch route run, or unsupported guest suspend. The existing
non-probe kernels already retain separate 50-switch and routing evidence. The
next safe step is objective Windows/Linux measurement or new public protocol
documentation, not another live query variant or a SpeakerEQ upload.
