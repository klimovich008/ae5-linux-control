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

The first two commits are already upstream but were not present in 6.18.40:

- `778031e1658d`: set the auto-detect default from headphone-pin capability;
- `6fd9f6e870ea`: disable auto-detect when userspace explicitly selects an
  output.

The generated 6.18 DSP patch has SHA-256
`2e53dc7d759ddf7ed8d59a1016f5ff25f44f6dceedd3eb08b4d6f071616870fe`.
A clean-worktree replay of the sequence above produced byte-identical
`Kconfig`, `Makefile`, `ca0132.c`, parser, and parser-test files to the source
used for the tested kernel.

The resulting `6.18.40-ae5-lts+` build passed:

- production CA0132 and parser-test object builds with `W=1 KCFLAGS=-Werror`;
- all four `ca0132-dsp-image` KUnit cases under x86-64 KVM;
- no-device boots in both session and system libvirt guests;
- one managed VFIO cycle on the physical `1102:0012/1102:0051` AE-5.

The full build, hardware, package, and recovery evidence is in
[`docs/LTS_KERNEL_VALIDATION.md`](../../../docs/LTS_KERNEL_VALIDATION.md).
