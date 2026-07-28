# Linux 6.18 LTS backport

This directory makes the tested AE-5 patch stack reproducible on the exact
Linux `6.18.40` stable source at
`221fc2f4d0eda59d02af2e751a9282fa013a8e97`.

The main DSP-image bounds patch targets the newer ALSA tree. Linux 6.18 has a
duplicate C-Media Makefile line and does not include `generic.h` from
`ca0132.c`, so two context hunks do not apply there. Use the 6.18-specific
[`ca0132-dsp-image-bounds.patch`](ca0132-dsp-image-bounds.patch) instead. Its
functional changes and new parser tests are identical to the main patch.

Apply the complete stack in this order:

```sh
git checkout 221fc2f4d0eda59d02af2e751a9282fa013a8e97
git fetch https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git \
  778031e1658d206a52bf9491c91ae5d4f4a2509d \
  6fd9f6e870ea285f05102e8e00e6a7f4495a9a02
git cherry-pick 778031e1658d206a52bf9491c91ae5d4f4a2509d
git cherry-pick 6fd9f6e870ea285f05102e8e00e6a7f4495a9a02
git apply /path/to/ae5-linux-control/kernel/ca0132-wedge-angle-default.patch
git apply /path/to/ae5-linux-control/kernel/ca0132-eq-preset-control-cache.patch
git apply /path/to/ae5-linux-control/kernel/ca0132-ae5-hide-ineffective-wuh-controls.patch
git apply /path/to/ae5-linux-control/kernel/backports/6.18/ca0132-dsp-image-bounds.patch
git diff --check
```

For the separately reviewable onboard-RGB extension, then apply:

```sh
git apply /path/to/ae5-linux-control/kernel/backports/6.18/ca0132-ae5-onboard-leds.patch
git diff --check
```

For the separately reviewable Direct Mode extension, apply:

```sh
git apply /path/to/ae5-linux-control/kernel/ca0132-ae5-direct-mode.patch
git diff --check
```

For the Smart Volume resume fix, apply the 6.18 context adapter last:

```sh
git apply /path/to/ae5-linux-control/kernel/backports/6.18/ca0132-restore-smart-volume-resume.patch
git diff --check
```

The adapter applies both directly after the production stack and after the
complete RGB/Direct Mode stack. Its functional C changes are identical to the
main [`ca0132-restore-smart-volume-resume.patch`](../../ca0132-restore-smart-volume-resume.patch);
only the context around `ca0132_init()` accounts for the RGB restore already
present in the complete 6.18 stack.

The complete host-package workflow for the production, RGB, Direct Mode, and
Smart Volume stack is in
[`docs/HOST_KERNEL_BUILD.md`](../../../docs/HOST_KERNEL_BUILD.md).

The first two commits are already upstream but were not present in 6.18.40:

- `778031e1658d`: set the auto-detect default from headphone-pin capability;
- `6fd9f6e870ea`: disable auto-detect when userspace explicitly selects an
  output.

The generated 6.18 DSP patch has SHA-256
`2e53dc7d759ddf7ed8d59a1016f5ff25f44f6dceedd3eb08b4d6f071616870fe`.
A separate 6.18 context adapter for the onboard-LED candidate has SHA-256
`05cbcb09a12c5b3a46491ff0d1192f2c36c6fe7766ae1343168be300be4900e4`;
its C changes are identical to the upstream patch, while its Kconfig hunk
accounts for 6.18 lacking the later `SND_HDA_GENERIC` selection.
A separate 6.18 context adapter for the Smart Volume resume candidate has
SHA-256
`29af4c56ada3b74070c7f47e88bf973dfbcfd409103a318bf76f7d11802cf1e3`.
Strict `checkpatch.pl` reports zero findings across its 155 lines.
A clean-worktree replay of the sequence above produced byte-identical
`Kconfig`, `Makefile`, `ca0132.c`, parser, and parser-test files to the source
used for the tested kernels.

The resulting `6.18.40-ae5-lts+` build passed:

- production CA0132 and parser-test object builds with `W=1 KCFLAGS=-Werror`;
- all four `ca0132-dsp-image` KUnit cases under x86-64 KVM;
- no-device boots in both session and system libvirt guests;
- one managed VFIO cycle on the physical `1102:0012/1102:0051` AE-5.

The LED-extended `6.18.40-ae5-lts-rgb+` stack additionally passed a full
kernel/module build, a card-less boot, and a managed physical cycle with five
multicolor LED devices, bounded color/brightness writes, unchanged mixer
state, and exact host recovery.

The Direct Mode-extended source was rebuilt as the side-by-side
`6.18.40-ae5-lts-rgb-direct-host+` RPM using the target Nobara host
configuration. Its package and module tree passed non-installing static
verification and an exact-image no-audio QEMU smoke boot. A newer
`6.18.40-ae5-lts-rgb-direct-svm-host+` candidate adds the Smart Volume adapter;
its current complete-stack source SHA-256 is
`01ab8448b258a24046ad59f279270eff4706d9315f616a39debf79b6432a7f71`.
Neither candidate has been installed or booted on bare metal.

The full build, hardware, package, and recovery evidence is in
[`docs/LTS_KERNEL_VALIDATION.md`](../../../docs/LTS_KERNEL_VALIDATION.md).
