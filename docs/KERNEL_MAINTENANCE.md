# Maintainable AE-5 kernel workflow

The AE-5 changes are maintained as an ordered patch queue, not as one frozen
kernel binary. The same queue can be checked against every new kernel source
before anything is built or installed.

No command in this workflow installs a module, writes `/boot`, changes the
boot loader, or loads code into the running kernel.

## Current exact baseline

The target host currently runs
`7.1.4-200.nobara.fc44.x86_64`. Its source package is
`kernel-7.1.4-200.nobara.fc44.src.rpm`; the package's CA0132 source is
byte-identical to Linux stable `v7.1.4`, and none of Nobara's downstream
patches changes CA0132. The matching `kernel-devel`, `.config`, and
`Module.symvers` are installed. `CONFIG_SND_HDA_CODEC_CA0132=m`, and Secure
Boot is disabled.

The 2026-07-26 non-installing validation used:

- source RPM SHA-256
  `3c832ad0c6ceacf76c94648d5d2964a338fa9e734c6ca8c09e17ed05dd015fd7`;
- series-file SHA-256
  `0860e0c593d0482b68dcf4fb9a46fe55104688c59f2dd3abc763b0b1389ece3b`
  and aggregate seven-patch SHA-256
  `e63443c9f561e99ff768c5d686fd7d086cd23ea46ee50a562ef2ed426c3fffcf`;
- base-config SHA-256
  `2da93a68ccd892892f96334b0a48a807963437d7ffa5e3edb7f1710eee360eb6`
  and migrated build-config SHA-256
  `bdc869b4ff8c28c1421ccd0e6ae901c5180637cd3d2f23f06bf48ed9bcabc2bf`.

The external-module gate passed with warnings as errors, exact
`7.1.4-200.nobara.fc44.x86_64` `vermagic`, and patched CA0132 source SHA-256
`76bdd35018012a3ccfad5f25b84bc3c8eeab589df6cd1196761c37e938725beb`.
The complete build produced release `7.1.4-ae5-current`; its main RPM has
SHA-256
`8c9f50229ffc764a3574ca0e789991406f1f932aba40035fd116c2a4e542d434`.
Non-installing extraction verified 6,469 signed zstd modules, all required
device/configuration and AE-5 markers, exact CA0132 `vermagic`, dependency
indexing, package scripts, and no conflicts or obsoletes. The boot-image
SHA-256 is
`462b8a0d85558c9a4c4c7146a548d6ff16204a1a966b1af46793c6b864585599`;
a Q35/TCG guest with no audio or network reached the expected no-root
filesystem panic while reporting `7.1.4-ae5-current`.

The same RPM then passed a full-root test in a recoverable Fedora 44 guest
with no Creative PCI device or emulated audio. An install-only RPM test passed
before installation. Fedora's kernel-install script selected the new kernel as
the saved default, so the test explicitly restored
`6.19.10-300.fc44.x86_64` as `saved_entry` and selected the custom BLS entry
only through `next_entry`. The one-shot boot:

- reported release `7.1.4-ae5-current`;
- loaded `snd-hda-codec-ca0132` from the matching module tree with exact
  `vermagic` and the build-time module signature;
- had kernel taint `0`, zero failed systemd units, and no CA0132, HDA, Creative,
  or audio messages in the kernel journal;
- consumed `next_entry` while preserving the stock Fedora `saved_entry`.

A second boot without an override returned automatically to Fedora
`6.19.10-300.fc44.x86_64`, again with taint `0`, zero failed units, and an
empty `next_entry`. The guest shut down cleanly and `qemu-img check` found no
disk errors. Nothing was installed or loaded on the host.

The fail-closed installation helper later repeated that complete cardless
cycle from a clean package state. Its simulation first proved that a package
script may change the saved entry, that the original stock entry is restored
exactly, that only the candidate becomes `next_entry`, and that an existing
one-shot override blocks installation. In the recoverable guest, the helper
then installed the exact RPM, preserved Fedora
`6.19.10-300.fc44.x86_64` as `saved_entry`, selected the candidate once,
booted the signed CA0132 module with taint `0`, zero failed units, and clean
relevant logs, and returned automatically to stock on the next boot. The
guest shut down cleanly and its disk again passed `qemu-img check`.

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
9. Keep playback at or below 20% for the first physical matrix; validate
   headphone/speaker routing, Direct Mode, DSP controls, suspend/resume,
   Smart Volume restoration, logs, shutdown, and exact state recovery.

Patch failure, compile failure, module signature mismatch, unexpected
`vermagic`, a kernel warning, or incomplete audio-state recovery blocks
installation and physical testing.
