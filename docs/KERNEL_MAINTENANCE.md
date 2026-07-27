# Maintainable AE-5 kernel workflow

> **Current queue (2026-07-27):** the authoritative `kernel/series` excludes
> Direct Mode and includes both the fail-closed AE-5 OutFX guard and the
> qualified stable-playback fix. Its verified `7.1.4-ae5-stable` RPM is
> installed side by side and selected for the next boot only. The installed
> `7.1.4-ae5-guarded` release predates the final fix and is historical; do not
> select it for physical testing. The stock Nobara kernel remains the running
> and saved/default entry.

The AE-5 changes are maintained as an ordered patch queue, not as one frozen
kernel binary. The same queue can be checked against every new kernel source
before anything is built or installed.

Compatibility checks, queue application, builds, and package verification do
not install a module, write `/boot`, change the boot loader, or load code into
the running kernel. Only the separately named installation helper performs an
installation, and it never reboots.

## Current exact baseline

The target host currently runs
`7.1.4-200.nobara.fc44.x86_64`. Its source package is
`kernel-7.1.4-200.nobara.fc44.src.rpm`; the package's CA0132 source is
byte-identical to Linux stable `v7.1.4`, and none of Nobara's downstream
patches changes CA0132. The matching `kernel-devel`, `.config`, and
`Module.symvers` are installed. `CONFIG_SND_HDA_CODEC_CA0132=m`, and Secure
Boot is disabled.

The already installed historical guarded build used:

- source RPM SHA-256
  `3c832ad0c6ceacf76c94648d5d2964a338fa9e734c6ca8c09e17ed05dd015fd7`;
- series-file SHA-256
  `298333722ceb859dcab345296ea6421f7ff9881ba1cfb88bb29c39e46b8d0b5f`
  and aggregate seven-patch SHA-256
  `cb89ce2f96ae010bc0e9daf6e48963f1892200a9b7f400311667606136d3cf18`;
- OutFX-guard patch SHA-256
  `cd2a242facf1ee0aab7e9ff0632e282e644ba4fbf390ee19af17e85743b67fa1`;
- pristine and patched CA0132 source SHA-256
  `7b61bcb02c4079b9ca6c82cde3147e95706cdbe958324ae383e7875d9a33a4f0`
  and
  `c5d4134d7e3a053b3046f215abce0257193e35c990e26081a3251c414df7074d`;
- base-config SHA-256
  `2da93a68ccd892892f96334b0a48a807963437d7ffa5e3edb7f1710eee360eb6`
  and migrated build-config SHA-256
  `e84f6e5c2e144564b69ce4cf76174d62765851295800f76ef74e00e6aafaf161`.

The external-module warning-as-error gate passed with exact stock
`vermagic`; its test module SHA-256 was
`605f2f37c846ce3af7dbd52295e933858ec28c4a092fc6cf73ebf4b5440b2184`.
The complete build produced release `7.1.4-ae5-guarded` and RPM:

```text
/home/maks/.cache/ae5-control/nobara-kernel-guarded-20260727-v1/ae5-host-build/rpmbuild/RPMS/x86_64/kernel-7.1.4_ae5_guarded-1.x86_64.rpm
```

Its SHA-256 is
`2bed800fcae874856ad934fd53dfa85270fba9475d8fd9b4e65ead6f461a0e76`.
Non-installing extraction verified 6,469 signed zstd modules, required
configuration and AE-5 markers, exact CA0132 `vermagic`, dependency indexes,
package scripts, the OutFX guard marker, and the absence of Direct Mode. The
boot-image SHA-256 is
`66ea31488fee9977c05328f87fc49d6aed3d94c4246dbf0cec8185402cab2bb6`;
the installed compressed CA0132 module SHA-256 is
`d383eac5f44f5d8ef0131b020500fb8e055502c4c40d9b8ef81f141d33193f31`.

A Q35/TCG no-root smoke guest reached Linux
`7.1.4-ae5-guarded` and the expected missing-root panic. The exact RPM then
passed a full-root Fedora 44 cardless guest test with no Creative PCI device
or emulated audio: the guest rebooted into the guarded release, reached
`systemd` running with zero failed units, and loaded the matching signed
`snd-hda-codec-ca0132` module. The guest shut down cleanly and `qemu-img
check` reported no errors.

The exact RPM is installed side by side on the physical host. Installation
did not reboot or load custom code. At installation time, the boot-loader
state read back as:

```text
saved_entry=fca8dc3f5d9347008f0dfcd322dbdcd8-7.1.4-200.nobara.fc44.x86_64
next_entry=fca8dc3f5d9347008f0dfcd322dbdcd8-7.1.4-ae5-guarded
default_kernel=/boot/vmlinuz-7.1.4-200.nobara.fc44.x86_64
running_kernel=7.1.4-200.nobara.fc44.x86_64
```

The obsolete one-shot selection was cleared after the stable-playback fix was
qualified because this artifact lacks
`ca0132-ae5-stable-playback-stream.patch`. That historical `next_entry` was
removed before the distinct `7.1.4-ae5-stable` build below passed its full
gate. The saved/default entry and `grubby --default-kernel` still resolve to
stock `7.1.4-200.nobara.fc44.x86_64`; the current one-shot entry names only
`7.1.4-ae5-stable`.

## Current stable-playback queue

The authoritative queue now has eight patches. Against ALSA `for-next`
`61471f29f3157f33a61194bf82b4a289cc03e1f1`, its series SHA-256 is
`c0093c53597db2128dfbc24c8375fab34cc3a41608c70e1e6291ec1c2e84151f`
and aggregate patchset SHA-256 is
`17decd4c9bc79d20565ca2c94fe00f2a4bcce7853219236c93d3e2be27bfe1a4`.
The stable-playback patch SHA-256 is
`26a4599bdab8a75cce5bddb06e4cb3ca2de081706148040b240201df44ad8dc7`.

The queue passed isolated apply/reverse compatibility, `git diff --check`,
strict checkpatch with no findings for the new patch, and an upstream
`ca0132.o` build with `W=1 KCFLAGS=-Werror`. The exact Nobara 7.1.4 functional
candidate passed the same warning gate and physical-card VFIO qualification;
see
[`PCM_REOPEN_EVIDENCE.md`](PCM_REOPEN_EVIDENCE.md).

### Exact stable package and staged host boot

The complete build from the current queue produced:

```text
release: 7.1.4-ae5-stable
package: kernel-7.1.4_ae5_stable-1.x86_64
RPM SHA-256: a295451e29ee936095068b47da7c34d565a21fdc0079bc3555b0ad9bd18fbda9
base config SHA-256: 2da93a68ccd892892f96334b0a48a807963437d7ffa5e3edb7f1710eee360eb6
build config SHA-256: 9a04016620ae6a3d5b15965ce628bf9c4d3179748fd142a54e9ce5c247297bed
```

The package verifier extracted 6,469 modules and accepted the required
configuration, compressed signed CA0132 module, exact vermagic, dependency
indexes, and all current AE-5 source markers. The exact RPM then passed a
physical-card Fedora passthrough boot: zero taint, signed matching module,
clean DSP initialization, a clean first-open capture, 12/12 warm reopens, a
clean 20-second-idle reopen, exact `EOPNOTSUPP` OutFX rejection with off
readback, and 8/8 clean subsequent reopens.

The guarded installer installed this package side by side without rebooting.
The boot loader readback is:

```text
running_kernel=7.1.4-200.nobara.fc44.x86_64
default_kernel=/boot/vmlinuz-7.1.4-200.nobara.fc44.x86_64
saved_entry=fca8dc3f5d9347008f0dfcd322dbdcd8-7.1.4-200.nobara.fc44.x86_64
next_entry=fca8dc3f5d9347008f0dfcd322dbdcd8-7.1.4-ae5-stable
```

After the next reboot, run
`scripts/check-ae5-kernel-runtime.sh 7.1.4-ae5-stable` before changing any
audio control. The one-shot selection leaves stock as the saved/default entry.

The exact source RPM can be obtained without root:

```sh
release=$(uname -r)
dnf download --source "kernel-core-$release"
```

Nobara's complete prepared source can be produced without installing it:

```sh
work=/path/to/disposable/kernel-work
mkdir -p "$work"/rpmbuild/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
rpmbuild -rp --nodeps \
  --define "_topdir $work/rpmbuild" \
  kernel-7.1.4-200.nobara.fc44.src.rpm
```

Use the resulting
`rpmbuild/BUILD/kernel-7.1.4-build/linux-cachyos-7.1.4-1` directory as the
source tree. On a future update, use the source RPM matching that new running
or target kernel instead.

## Check and apply the queue

[`kernel/series`](../kernel/series) is the single ordered list. A compatibility
check copies only the files touched by the queue into an isolated temporary
tree, applies every patch strictly, reverses every patch, and compares the
round trip with the source:

```sh
scripts/apply-kernel-series.sh --check /path/to/linux-source
```

The source tree is not modified. Any context drift, missing source, whitespace
defect, or failed reverse application stops the command. It never uses
three-way merge or patch fuzz.

GitHub's `Kernel patch compatibility` workflow runs the same strict check when
the queue or checker changes, on manual request, and every Monday against the
current ALSA `for-next` head. It downloads only the four existing source files
needed by the queue at the exact remote commit, rather than cloning the full
kernel repository. A scheduled failure is an upstream-drift signal: rebase and
retest the queue before using that newer source. This gate checks patch
compatibility only; the exact-kernel module and full-build gates below remain
mandatory.

Apply only after the check passes:

```sh
scripts/apply-kernel-series.sh --apply /path/to/linux-source
```

For Git sources, apply mode refuses a dirty worktree. For a source-RPM prep
tree, use a fresh disposable directory because RPM prep trees have no Git
baseline for detecting unrelated local edits.

## Fast per-kernel build gate

Before spending time on a complete kernel, compile only the replacement
CA0132 module against the target kernel's exact `kernel-devel` tree:

```sh
scripts/build-ca0132-module.sh \
  /path/to/patched-linux-source \
  "$(uname -r)"
```

The command uses `W=1 KCFLAGS=-Werror`, requires matching `vermagic`, checks
the complete AE-5 feature markers, and does not install or load the output.
This is suitable as the first update gate and as the compilation stage for a
future akmods package.

The module gate may leave generated Kbuild state in the patched source tree.
Before a complete out-of-tree kernel build, clean only that disposable source
tree:

```sh
make -C /path/to/patched-linux-source mrproper
```

`mrproper` removes generated files but preserves the applied source patches.
The full builder detects generated state and refuses to proceed instead of
cleaning the source automatically; it then verifies the applied feature
markers before compiling.

CA0132 uses private in-tree HDA interfaces. An akmods package can rebuild it
for nearby kernel updates, but it cannot promise compatibility across API
changes. A failed patch check or module build is an intentional rebase gate;
the old module or kernel must remain in use until the queue is updated and
retested.

## Full side-by-side kernel RPM

The safest hardware-test artifact remains a complete kernel with its matching
module set:

```sh
scripts/build-ae5-kernel-rpm.sh \
  /path/to/patched-linux-source \
  /path/to/empty-build-tree \
  "/boot/config-$(uname -r)" \
  -ae5
```

The builder migrates the current configuration with `olddefconfig`, first
compiles CA0132 with warnings treated as errors, enables build-time module
signing, and then runs the kernel's `binrpm-pkg` target. It writes only to the
explicit build tree and does not install the RPM.

Verify a generated main kernel RPM before any installation:

```sh
scripts/check-host-kernel-rpm.sh \
  /path/to/kernel-RPM \
  EXPECTED-KERNEL-RELEASE
```

The staging helper runs that verifier by default and makes no changes:

```sh
scripts/install-ae5-kernel-test.sh \
  /path/to/kernel-RPM \
  EXPECTED-KERNEL-RELEASE
```

Only its explicit root mode installs:

```sh
sudo scripts/install-ae5-kernel-test.sh --install \
  /path/to/kernel-RPM \
  EXPECTED-KERNEL-RELEASE
```

Before installation it requires a disabled or unavailable Secure Boot path,
`GRUB_DEFAULT=saved`, an existing stock saved BLS entry and module tree, no
pending `next_entry`, at least 512 MiB free in `/boot`, an absent candidate
package, and a successful RPM install-only test. It then installs only the
verified main RPM, restores the exact original `saved_entry`, identifies the
new BLS entry by its expected release, schedules that entry with
`grub2-reboot`, and reads back both GRUB variables. It never reboots. A failed
post-install check restores the stock saved entry and clears `next_entry`; it
does not guess whether a partially installed RPM is safe to erase.

Do not install the generated `kernel-headers` RPM. Keep at least two stock
Nobara kernels and the stock saved boot default. A custom kernel should first
be selected for one boot only. Distribution kernel-install scripts may select
a newly installed kernel as the saved default; inspect the boot-loader
environment immediately after installation, restore the stock saved entry,
and only then schedule the custom entry as a one-shot boot.

## Update and test sequence

For every future kernel:

1. Obtain and prepare its exact distribution source and config.
2. Run the isolated queue compatibility check.
3. Apply the queue to a fresh source tree.
4. Pass the exact-kernel external-module warning and `vermagic` gate.
5. Run `mrproper` on that disposable patched tree.
6. Build and non-installingly verify a side-by-side kernel RPM.
7. Smoke-boot without the AE-5, then boot a cardless full guest.
8. Retain the stock boot default and perform one physical test boot.
9. Before changing a control, run the fail-closed read-only runtime gate:

   ```sh
   bash scripts/check-ae5-kernel-runtime.sh EXPECTED-KERNEL-RELEASE
   ```

   It requires the exact untainted release, AE-5 PCI identity and
   `snd_hda_intel` binding, matching signed CA0132 module, Direct Mode absent,
   all five onboard LED interfaces, OutFX off, closed PCMs, Low gain when
   available, and the existing routing/20% safety preflight.
10. Keep playback at or below 20% for the first physical matrix; validate
   boot stability, signed-module state, LEDs, logs, rejected OutFX enable,
   harmless redundant OutFX-off requests, and a managed persistent S16 stream.
   With every AE-5 analog output physically unplugged, the reproducible
   internal-capture gate is:

   ```sh
   AE5_ANALOG_OUTPUTS_UNPLUGGED=1 \
       bash scripts/check-ae5-playback-stability.sh EXPECTED-KERNEL-RELEASE
   ```

   It independently hard-mutes Master and Front, selects Low gain, uses a
   −30 dBFS exact-card fixture, and restores only desktop services that were
   active before the run. Do not set the acknowledgement merely to bypass the
   topology check.

   Do not test Direct Mode or hardware OutFX enable as a listening mode.
   Keep hardware EQ/effects and output transitions blocked until the rebuilt
   kernel passes its cold-start and analog-output acceptance gate. Treat
   suspend/resume as a separate bounded test because only runtime PM was
   qualified in VFIO.

A normal reboot validates the guarded physical boot. A true cold-start test
requires a later complete shutdown and physical power removal. Keep AE-5
outputs unplugged, or headphones off the user's head, until the runtime gate
passes. Once managed playback opens the PCM, do not force it closed merely to
manufacture a reopen test.

Patch failure, compile failure, module signature mismatch, unexpected
`vermagic`, a kernel warning, or incomplete audio-state recovery blocks
installation and physical testing.
