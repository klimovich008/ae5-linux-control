# CA0132 kernel patches

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

Run the read-only apply and upstream style checks against a clean Linux source
tree:

```sh
bash scripts/check-ca0132-patch.sh /path/to/linux
```

To compile the affected object without installing it:

```sh
linux_source=/path/to/linux
patch_file=$PWD/kernel/ca0132-wedge-angle-default.patch
kernel_build=$(mktemp -d "${TMPDIR:-/tmp}/ae5-kernel-build.XXXXXX")

git -C "$linux_source" apply "$patch_file"
make -C "$linux_source" O="$kernel_build" defconfig
"$linux_source/scripts/config" --file "$kernel_build/.config" \
  --enable SND --enable SND_HDA --module SND_HDA_CODEC_CA0132
make -C "$linux_source" O="$kernel_build" olddefconfig
make -C "$linux_source" O="$kernel_build" -j"$(nproc)" W=1 \
  sound/hda/codecs/ca0132.o
```

On 2026-07-24, the patch applied cleanly to the official ALSA maintainer
`tiwai/sound.git` `for-next` branch at
`61471f29f3157f33a61194bf82b4a289cc03e1f1` and the Torvalds tree at
`48a5a7ab8d6ab7090564339e039c421f315de912`.
`scripts/checkpatch.pl --no-tree --strict` reported zero errors, warnings, or
checks. The affected x86-64 object compiled from both trees with `W=1`, GCC
16.1.1, GNU Make 4.4.1, and `CONFIG_SND_HDA_CODEC_CA0132=m`; both resulting
144,496-byte `ca0132.o` files had SHA-256
`dc0fa05f0dc9f27d12e28d593058a09d834e1b123cac82a0201397f9a877a3b8`.
This source/build validation did not install a kernel, load a module, or write
the AE-5.

Build and boot the patched kernel or module, then verify before writing the
control:

```sh
amixer -c <AE5_CARD> cget "iface=MIXER,name='Wedge Angle Capture Volume'"
```

The value must be `30`, remain within the advertised `20..180` range, and
setting 20, 30, and 180 must read back exactly. Voice Focus recording tests
must show no new kernel warning or DSP timeout.
