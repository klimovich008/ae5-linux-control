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

## Rebuild with exact input routes and profile compatibility

The recording investigation added card-scoped Microphone, Front Microphone,
and Line In paths. It also made the ineffective What U Hear controls read-only
and kept profiles compatible with kernels that expose or hide them. The final
rebuilt binary RPM has SHA-256
`a5a780cf5df668e387d3b33afb2347d59f37dd7780a59ae6d652116449a78bab`;
the source RPM has SHA-256
`2d0964e75d2022998a1173c75dc6d6443a71f58a6ac5484d71a1eccad71ae87c`.
RPM payload digests passed.

The release build passed all 49 Rust tests, the ACP output and input
invariants, diagnostics self-test, desktop and strict offline AppStream
validation, and the private build-path check. The exact binary payload was
streamed from the RPM and all six ACP/WirePlumber files matched the repository
byte-for-byte:

| File | SHA-256 |
|---|---|
| Front Microphone path | `8cf31284e79acc7d2f53c58b51e1621dbf65af4b937636f87befede2965468b9` |
| Line In path | `6bf23c70b1b828ba036d220a6eccbbea741687a9c4b40270d768014c8c02c2d6` |
| Microphone path | `a20eb71df4b9ffe8540a5e0522e7679574e59b3402f8b95cbecad2ba748d466c` |
| fixed headphone path | `49b50e8fbc0a87fe2e963a83574da9b2f697aa3ddf274261723603dab88e15a6` |
| AE-5 profile set | `8ec638cfce429c96e442eb9490816bbaeb0d0eaa14c7f9895ecfde2b0b76c17d` |
| WirePlumber rule | `b3ae2ac7a9b43f5a43b66067b39dcf0cf088386a3f7b880bf43ac62e8a9dfd2f` |

The prior headphone-fix RPMs remain under `dist/previous-fdfa462/`; the first
input-route build was preserved under
`dist/previous-recording-acp-pre-profile-compat/`. The rebuilt package has not
replaced the host RPM; the user-scoped profile supplied the live route test
described in
[`RECORDING_MIXER_INVESTIGATION.md`](RECORDING_MIXER_INVESTIGATION.md).

## Rebuild with synchronized desktop routes

The shared Rust setter now sends `Output Select` and `Input Source` choices
through the matching WirePlumber routes after confirming the packaged AE-5
profile set. The rebuilt binary RPM has SHA-256
`63c0d378607625593964fab95dba856d5109222a633119075adbff38cac6da3b`;
the source RPM has SHA-256
`c3e361683d54cc88aada3d35d8882d7b652834f06c7c5af680f5dcd455f8bbc9`.
Both RPM payload digests passed.

The release build passed all 50 Rust tests, strict route-order and shared-Front
validation, the diagnostics self-test, desktop and AppStream validation, and
the private-build-path check. All six packaged ACP/WirePlumber files matched
the repository byte-for-byte.

The exact `ae5ctl` binary streamed from this RPM selected Line In,
Microphone, Speakers, and Headphone through the live target card. The complete
ALSA mixer and retained WirePlumber route-state hashes matched their starting
values afterward. The previous recording-control RPMs were preserved under
`dist/previous-3df5d27/`; the host package database was not changed.

## Automated RPM lifecycle gate

Collected on 2026-07-25 in a fresh Fedora 44 container. The new
`scripts/check-rpm-lifecycle.sh` gate is run against the binary RPM produced
by `scripts/build-rpm.sh` in pull-request CI and on `main`.

The first local end-to-end run of the gate built
`ae5-control-0.1.0-1.fc44.x86_64.rpm`, SHA-256
`41a8768321af1a6b0000db639e0567902aea87ac926da7e227c924ef12b4d4ca`.
The RPM release check passed all 51 Rust tests, the ACP route invariants,
diagnostics self-test, desktop and strict offline AppStream validation, and
the private-build-path check.

The lifecycle gate then:

1. required a disposable container and a clean package database;
2. installed the local RPM with weak dependencies disabled;
3. passed `rpm -V ae5-control`;
4. found the GUI, CLI, and diagnostics commands and exercised packaged
   command help plus the diagnostics self-test;
5. confirmed hardware status could not succeed without `/dev/snd`;
6. removed the package and confirmed all 17 package-owned files were gone;
7. confirmed byte-for-byte preservation of a user-profile sentinel and
   `/var/lib/alsa/asound.state`.

This turns the clean Fedora build/install/remove result into a repeatable
release gate. It does not emulate the physical AE-5, the host PipeWire
session, or a desktop application menu.

## Maintained-LTS hardware package cycle

The same binary RPM, SHA-256
`63c0d378607625593964fab95dba856d5109222a633119075adbff38cac6da3b`,
was installed in the Fedora 44 system guest running
`6.18.40-ae5-lts+` with the physical AE-5 attached through managed VFIO.

The packaged CLI detected `1102:0012/1102:0051`, reported 46 simple controls,
saved and validated a 46-control profile, changed Wedge Angle from `30` to
`20`, read it back, and restored `30`. The complete guest mixer returned to
SHA-256
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`.

The headless SSH guest had no desktop-login device ACL or user PipeWire
session, so the CLI ran through guest `sudo` and desktop route writes were not
claimed. Removing the RPM preserved the same mixer hash, Flat EQ, Wedge `30`,
one DSP initialization, and zero failed guest units. Host shutdown/recovery
then passed exactly as recorded in
[`LTS_KERNEL_VALIDATION.md`](LTS_KERNEL_VALIDATION.md).

## Remaining release gate

This proves clean Fedora dependency resolution and package ownership/removal,
physical-card operation of the exact payload, and a headless maintained-LTS
package cycle. It does not claim a system-installed application on this host:
`sudo` required an interactive password, so the host RPM database and `/usr`
were deliberately not changed.

Before calling Phase 5 complete, install the RPM through an authenticated host
package transaction, launch it from the desktop application menu, exercise a
user-approved control as the desktop user, uninstall it, and confirm the user
profile library and ALSA state remain intact.
