# CA0132 kernel patches

These patches are independent, reviewable Linux changes and diagnostic
experiments. None has been loaded on the target AE-5, and none changes the
running kernel merely by being present in this repository.

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

## Wedge Angle default

`ca0132-wedge-angle-default.patch` fixes an invalid ALSA control value present
in Linux `v7.1.4` and upstream `master` at `48a5a7ab8d6a`.

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
and no systemd unit failed. The guest had no emulated audio device or PCI host
device, so this proves the candidate builds and boots but does not yet prove
the physical control value or Voice Focus behavior.

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

QEMU must be installed for a non-UML architecture. This validates parser logic
and compilation only. A later alternate-kernel and cold-boot session is still
required to prove successful firmware loading and playback on the physical
AE-5.

## Read-only SpeakerEQ address probe

[`ca0132-speaker-eq-address-probe.patch`](ca0132-speaker-eq-address-probe.patch)
implements the next bounded headphone-tuning experiment. It is diagnostic
instrumentation, not a functional driver fix and not a SpeakerEQ loader.

After a successful DSP firmware download, the patch sends the existing
`MASTERCONTROL_QUERY_SPEAKER_EQ_ADDRESS` request as one SCP `GET`. It runs only
for the AE-5/AE-5 Plus and AE-7 quirks, provides a four-byte reply buffer, and
accepts only an exact four-byte response. Success, DSP error, and unexpected
response length are reported through `codec_dbg()`.

The probe does not request another firmware file, upload coefficients, expose
an ALSA control, enable `SPEAKER_TUNING_USE_SPEAKER_EQ`, or change the
reported address. The result establishes only whether this DSP request is
implemented and stable on the target card. It cannot identify the meaning,
size, or safe contents of the returned memory region.

### Build-only validation

The patch was developed against Linux stable `v7.1.4`, whose `ca0132.c`
SHA-256 is
`7b61bcb02c4079b9ca6c82cde3147e95706cdbe958324ae383e7875d9a33a4f0`.
Both the unmodified and patched source compiled as external modules against
the exact running Nobara
`7.1.4-200.nobara.fc44.x86_64` kernel-devel tree with `W=1` and warnings
treated as errors. The patch also applies cleanly to Linux `master` at
`48a5a7ab8d6a` and ALSA `for-next` at `61471f29f315`; their identical CA0132
source compiled with the patch in the same isolated harness. Strict
`checkpatch.pl` reports no errors, warnings, or checks. No module was installed
or loaded.

Apply it to a matching source tree with:

```sh
git apply --check \
  /path/to/ae5-linux-control/kernel/ca0132-speaker-eq-address-probe.patch
git apply \
  /path/to/ae5-linux-control/kernel/ca0132-speaker-eq-address-probe.patch
```

The later authorized hardware session must use a known-good boot entry and
enable the CA0132 dynamic-debug call sites before module initialization, for
example with the kernel command-line option:

```text
snd_hda_codec_ca0132.dyndbg=+p
```

Acceptance requires one exact non-error reply, the same address across three
cold boots, normal playback, 50 speaker/headphone route changes, a
suspend/resume cycle, clean module removal or shutdown, and no CA0132, HDA,
ALSA, codec, or DSP warning. A control run without this patch must have the
same neutral frequency response. Loading the patched module or kernel remains
an explicit future test; this build-only milestone does not change the live
audio stack.
