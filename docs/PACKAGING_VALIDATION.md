# Nobara/Fedora package validation

Collected on 2026-07-24 from source commit
`71c1e8a3cf774d31aac07bb50d9fa47dd53961b1`. The tested binary RPM was
`ae5-control-0.1.0-1.fc44.x86_64.rpm`, SHA-256
`8c49dc0ab5cb9f30447d725f834e399595ce5f9367ac3b2f8502133ef4e367e5`.

## Reproducible package build

`scripts/build-rpm.sh` created the source, binary, and debuginfo RPMs from a
temporary archive with the locked Rust dependencies vendored into that
archive. RPM built Cargo in frozen offline mode and passed:

- all 46 Rust tests in the release profile;
- the diagnostics script self-test;
- strict offline AppStream validation;
- desktop-file validation;
- the private-build-path check for both installed Rust binaries;
- RPM payload and digest verification.

The binary package contained the GTK application, CLI, diagnostics command,
desktop entry, AppStream metadata, icon, licences, and all current project
documentation. It declared only normal runtime library, icon-theme, and
WirePlumber dependencies; it contained no daemon, setuid binary, device rule,
or kernel module.

## Clean Fedora install and removal

A rootless `fedora:44` container at image digest
`sha256:ad119a9813828e36e1bc0a8337c14b12dd5dc41673f64f81a39cb9f4a33ca1a8`
provided a clean RPM database and distribution dependency resolver. The test:

1. installed the local RPM with weak dependencies disabled;
2. passed `rpm -V ae5-control`;
3. found all three executables, the desktop entry, and AppStream metadata;
4. invoked the packaged CLI and confirmed the current guarded-reset command;
5. removed `ae5-control`;
6. confirmed the package was absent and every project-owned executable,
   desktop, and metadata path had been removed.

The disposable container was then deleted. A container cannot prove PCI,
ALSA, PipeWire-session, or desktop-launcher behavior on the physical card.

## Exact payload on the target AE-5

The RPM payload was extracted into a new temporary directory and executed
unmodified on the Nobara 44 target. This avoids substituting a Cargo build for
the package binary while leaving the host package database untouched.

The packaged CLI:

- detected card 0 as the audited `1102:0012` / `1102:0051` AE-5 with 48
  simple controls;
- reported Headphone, low gain, Slow Roll Off, and 2.0 channels;
- found the AE-5 as the default PipeWire playback target;
- validated all 29 Linux-driver reset controls without writing them;
- passed the packaged diagnostics self-test.

`ldd` found no missing dependency for either Rust executable. The packaged
GTK application remained running for a four-second launch probe and exited
when sent `SIGTERM`; it left no process behind. GTK emitted one host-wide
settings warning for the unknown `gtk-modules` key, which does not originate
from the package and did not prevent launch.

The complete 48-control snapshot had SHA-256
`7a61ac34dbca132e929806a1198a61f9334c5241bcb83e9da205152008ffea6e`
before and after the host checks. No matching CA0132, HDA, ALSA, codec, or DSP
kernel warning appeared during the test. The extracted payload was moved to
the recoverable desktop Trash.

## Rebuild with the headphone ACP fix

The first-use headphone investigation added three package-owned files:

- `/usr/share/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf`
- `/usr/share/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf`
- `/usr/share/wireplumber/wireplumber.conf.d/90-ae5-control.conf`

The rebuilt binary RPM has SHA-256
`9cc2aa2b63ce7266d268fff98519e435544a7e87d46fcde30f7f2802c415ac4f`.
Its release build passed all 46 Rust tests, the ACP shared-Front invariant,
the diagnostics self-test, desktop and AppStream validation, and the private
build-path check. RPM payload digests passed.

The exact binary payload was extracted to a temporary directory and all three
ACP/WirePlumber files matched the repository byte-for-byte. Their SHA-256
values are:

| File | SHA-256 |
|---|---|
| fixed headphone path | `49b50e8fbc0a87fe2e963a83574da9b2f697aa3ddf274261723603dab88e15a6` |
| AE-5 profile set | `8331ade79f7eb6bba5c3ee538e0111d543bf8a767830754e1844abe6bcdebebd` |
| WirePlumber rule | `b3ae2ac7a9b43f5a43b66067b39dcf0cf088386a3f7b880bf43ac62e8a9dfd2f` |

The previous validated RPMs were preserved under `dist/previous-71c1e8a/`
instead of being overwritten. This rebuild has not replaced the package in the
host RPM database; the user-scoped live profile test is documented separately
in [`DRIVER_ROUTING_INVESTIGATION.md`](DRIVER_ROUTING_INVESTIGATION.md).

## Remaining release gate

This proves clean Fedora dependency resolution and package ownership/removal,
plus read-only operation of the exact payload on the target hardware. It does
not claim a system-installed application on this host: `sudo` required an
interactive password, so the host RPM database and `/usr` were deliberately
not changed.

Before calling Phase 5 complete, install the RPM through an authenticated host
package transaction, launch it from the desktop application menu, exercise a
user-approved control, uninstall it, and confirm the user profile library and
ALSA state remain intact. Repeat on one maintained LTS kernel.
