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

## Normal-user onboard-lighting package cycle

Collected on 2026-07-25 with the physical AE-5 attached to the Fedora 44
system guest running `6.18.40-ae5-lts-rgb+`. The final binary RPM had
SHA-256
`96410b79323cc5396f0e84164c1434ca3aca5c2490eafc0fef7aa69e3ca2293e`;
the source RPM had
`4ac00e183c41923336adce8b0c4c96461e9bb02f3eec4241aebd5a5ac91537ee`.
Its release check passed all 53 Rust tests, ACP/report/desktop/AppStream
validation, and strict udev-rule verification. A disposable Fedora 44
lifecycle test installed and removed all 19 package-owned files while
preserving profile and ALSA sentinels.

Before installation, all five exact onboard devices had root-owned mode
`0644` `brightness` and `multi_intensity` attributes. The package post-install
trigger matched the exact PCI and subsystem IDs, LED names, and
`red green blue` channel order, changing only those ten attributes to `0666`.
The SSH user was not in the guest's `audio` group and had no desktop-login
device ACL, yet the installed CLI ran without `sudo` and:

- changed all five LEDs from white to red;
- saved a user-owned mode `0644`
  `~/.config/ae5-control/lighting.json`;
- applied and read back the independent pattern red, green, blue, amber, and
  violet;
- rejected index `0` and channel value `256` without changing hardware or
  configuration;
- rolled hardware back exactly when its configuration directory was made
  unwritable;
- restored the saved pattern after direct temporary white values.

A forced permission loss on the third LED found an overly pessimistic
rollback error. The backend was corrected to skip LEDs already at their saved
value and continue attempting every required recovery. The rebuilt exact RPM
then returned the underlying permission error while preserving both the
hardware frame and configuration hashes. `udevadm test` showed the expected
single `chmod` command for the exact LED path.

Every stage retained the complete guest mixer SHA-256
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`.
No PCM was open, the DSP initialized once, no guest unit failed, and no
relevant kernel warning appeared. Before removal the hardware was returned to
its original white frame. Final removal deleted the rule and autostart entry,
returned all ten attributes to root-owned mode `0644`, and preserved the
lighting file byte-for-byte.

The headless guest did not provide a GTK desktop session, so this cycle proves
the package, normal-user shared backend, persistence, and failure handling,
but not a physical click through the GTK color dialog. Visible color
confirmation is also still pending.

## Desktop GTK lighting interaction

An additional test on 2026-07-25 ran the unchanged release GUI, SHA-256
`e43c30a608673f9a273ce4e896ac8691e72cbdab96824e841099e438319635ea`,
under the real KDE/Wayland desktop as UID 1000. A rootless, user-preserving
mount namespace bound five private, writable LED-class fixtures over
`/sys/class/leds` for that process only. The fixtures used the production
AE-5 names, `red green blue` channel order, and maximum brightness 255; no
test path or fake backend was added to the application.

The desktop accessibility interface selected the lazily loaded **Lighting**
page and invoked the semantic GTK actions rather than screen coordinates. The
native `GtkColorDialog` then passed these checks:

- choosing Red for the unified control wrote and read back `#E01B24` on all
  five fixtures and saved the same five values to `lighting.json`;
- choosing Blue for LED 3 wrote and read back `#3584E4` only on LED 3, retained
  Red on the other four, and refreshed the status and per-LED labels;
- selecting Green for LED 2 and pressing **Cancel** left every LED attribute
  and the saved JSON byte-for-byte unchanged;
- after all fixture colors and brightness values were changed out of band,
  `lighting-restore` reproduced the saved Red/Blue frame and a fresh GUI
  process displayed the five restored values.

Both GUI processes closed through their advertised accessibility action.
After the namespace ended, the host `/sys/class/leds` mount resolved exactly
as before, no application process or audio stream remained, the AE-5/Fifine
defaults were unchanged, and the complete host mixer SHA-256 remained
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`.
This closes the native desktop-dialog behavior gate without claiming that
fixture colors prove the physical LEDs' visible output.

## Rootless host installation and desktop launch

The development account has no sudo password, so a separate source installer
now provides a real, reversible application without pretending to install
system components. It builds the locked release, stores its private payload
under the user's XDG data directory, and links the CLI, GUI, report command,
desktop/AppStream/icon metadata, autostart entry, and per-user
WirePlumber/ACP files into their standard user locations. It leaves an
existing byte-identical path untouched, refuses any conflict before writing,
checks the required system ACP includes, and never restarts WirePlumber while
audio may be active.

`scripts/check-user-install.sh` passed an isolated lifecycle with temporary
HOME and XDG roots:

- release payload and integration hashes matched their source files;
- the desktop and AppStream files validated and the installed CLI/report
  commands ran;
- a second install was idempotent;
- unrelated destination content and a missing ACP include each stopped the
  installer before a partial payload appeared;
- an invalid payload marker stopped uninstall before any launcher was
  unlinked;
- the installed uninstaller removed its links and payload without relying on
  the source checkout while preserving profile and lighting sentinels.

The same path was then left installed for the real user on 2026-07-25. The GUI
and CLI SHA-256 values are respectively
`8f9e43543c5fdf1de8f9c09d587b8c193d099d43b2714ab8a6f8e81019d18912`
and
`553a1e4eca4a1f4c993fbd8b6e5cee7b3695a88a6205d4f52651fa854744fae8`.
The installer retained all existing, byte-identical routing links and did not
restart the audio session. Its retained
`ae5-control-user-install --uninstall` command was byte-identical to the
source installer. Launching the installed desktop entry produced the real
`AE-5 Control` GTK frame from the installed payload; accessibility confirmed
the exact `1102:0012/1102:0051` identity and all eight pages before closing
the window normally.

The upgraded payload adds a read-only native PipeWire route-health check.
`ae5ctl route-status` read the active `pw-dump` Routes and matched ALSA
Headphone and Microphone to the card-specific output and input routes, the
duplex profile, and `sound-blaster-ae5.conf`. The installed desktop entry
exposed the same **Desktop route health** card through AT-SPI, including the
combined `Matched`, ALSA output/input, PipeWire output/input, profile, and
profile-set text. The real GTK frame then closed through its advertised
`window.close` action.

Before and after installation and both launches, the complete host mixer
SHA-256 was
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`.
The AE-5/Fifine defaults, zero-stream state, every routing-file hash, and every
existing AE-5 profile/lighting hash were identical. This proves a useful
no-root desktop installation and its isolated removal behavior. It does not
claim an authenticated RPM transaction or install the RGB kernel/udev pieces.

## Transactional rootless upgrades

The user installer now stages every release file under the same data
filesystem, compares the staged files byte for byte with their build inputs,
and only then swaps the complete payload into the stable `user-install`
location. Existing application-menu, command, WirePlumber, and ACP links
continue to target that stable path. A rejected upgrade therefore cannot mix
old and new binaries.

The isolated lifecycle check replaced both installed binaries with explicit
old-version sentinels, reran the installer, and obtained exact current release
binaries while preserving the link manifest, native profile, and lighting
configuration. A forced staged-file verification failure, a failed live
directory swap, and a second attempted upgrade with a conflicting command path
all left every live payload file byte-identical. The swap failure restored the
complete previous payload. No staging or backup directory remained after the
successful or rejected upgrades.

The existing development-host installation was then upgraded in place. Its
GUI/CLI SHA-256 pair changed from
`8f9e43543c5fdf1de8f9c09d587b8c193d099d43b2714ab8a6f8e81019d18912` /
`553a1e4eca4a1f4c993fbd8b6e5cee7b3695a88a6205d4f52651fa854744fae8`
to
`b22a3fedd035b1dcca15822ee6521fc86c03d05a01e7eee91d66dcae0c547e99` /
`f15a973bc6e38e1952ccbbcd98dcb567ff71e533d866f51fa478b97fd1580f0c1`.
All 16 installed payload inputs matched the current source files byte for byte
and no staging or backup directory remained. The installed CLI found the
physical `1102:0012/1102:0051` card and matched its ALSA Headphone/Microphone
choices to the PipeWire routes with the Fedora 44 `wpctl` implementation.

The desktop entry resolved to the upgraded payload. In the real KDE/Wayland
session, AT-SPI confirmed the exact card identity and route-health card, loaded
all eight pages, and closed the window through its advertised `window.close`
action. The full mixer remained at SHA-256
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`;
the route-health output and existing user configuration set were unchanged.
No AE-5 PCM opened, all five PipeWire/WirePlumber units stayed active, and no
matching kernel warning appeared. This validation was read-only and played no
audio.

## Profile-library CLI host upgrade

The rootless installation was upgraded again after adding the shared
profile-library rename command. The installed release GUI and CLI SHA-256
values are respectively
`5f5610a1f52530e010701e69fc1e7e2afaa1d13daafc52d93b0acc5089772fc6`
and
`b2a56da0a1e933237a32ddccf1f0a154c514fa1308dd857544496d1160102874`,
each byte-identical to its release build input. Installed help exposes
`profile-rename LIBRARY_FILE NEW_NAME`.

The upgrade retained the complete raw mixer SHA-256
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`,
simple mixer SHA-256
`65a1da375bd1e6d523a91ee819fa1d8e88f63a34afc10b8e1ef56c736cc38a25`,
and AE-5 sink-definition SHA-256
`39c1581717da0b005d9ebc556f807e032e5ad5371dec4888eaa2360c69d81052`.
The AE-5 and FIFINE remained the default sink and source, and the installed
route-health command still matched ALSA Headphone/Microphone to both
card-specific PipeWire routes. PipeWire volume remained at its existing 43%,
which fails the project's 20% playback gate, so no audio test was attempted.

## Output route-health RPM rebuild

The route-health payload was also rebuilt as the Fedora 44 binary RPM with
SHA-256
`dc596375f3b479bbab1030284311a50526f515e17781dd24521203a5e12865e9`
and source RPM with SHA-256
`4d54516d984a9a8d8dc9a128efd7ad095df3f91089f64a131041db2b3219a715`.
The RPM release check passed all 57 Rust tests and the existing ACP,
diagnostics, desktop, AppStream, udev, and private-build-path gates. Its
metadata explicitly requires `pipewire-utils`, which supplies `pw-dump`.

A fresh rootless Fedora 44 container installed the exact binary RPM and
resolved `pipewire-utils-1.6.8`. Package verification and the installed
commands passed; removal deleted all 19 package-owned files while preserving
the profile and ALSA-state sentinels byte-for-byte. The previous RPM set and
its hashes remain preserved under `dist/previous-bea6902/`.

## Bidirectional route-health RPM rebuild

Extending the same parser and diagnostics to `Input Source` produced binary
RPM SHA-256
`8b039a1305fc5804910ecd807bfacc53ef655d2c532d216756d7ed96c3c4b647`
and source RPM SHA-256
`1ea43a7b5e389dd40f319799331137244ff639ef614c10e88d5fd86f2275b9f7`.
The hardened offline release build again passed all 57 Rust tests and every
package check. The package retained its explicit `pipewire-utils` dependency
and passed digest verification.

A fresh Fedora 44 container installed the exact package, including
`pipewire-utils-1.6.8`, verified its commands, and removed all 19 owned files.
The profile and ALSA-state sentinels remained byte-for-byte identical. The
prior output-only RPMs are preserved under `dist/previous-c026e4f/`.

## Speaker-layout and software-mixer RPM rebuild

The exact-layout synchronization and safe AE-5 software-mixer policy produced
binary RPM SHA-256
`57a03ecf28f56ef755d7620ce4df7adc6837205d70f616e6ab63edf6934518ed`
and source RPM SHA-256
`48da85c93762decf08be6ebb572b3041547797e2651dd5f5608253ba3e4a8a76`.
The hardened offline release build passed all 69 Rust/GTK tests plus the ACP,
diagnostics, desktop, AppStream, udev, and private-build-path checks.

The packaged headphone path uses `volume=ignore` and has SHA-256
`6f448005bbc506b0686d7bbe85b279d085390e2ecbf30639d6f010161d8e7e9e`.
The exact-card WirePlumber rule requires `api.alsa.soft-mixer=true` and has
SHA-256
`3ea8eac992eb15b0a279f377eaa3b8749317f14d2b6fbfa4b817ae149db90010`.
A fresh rootless Fedora 44 container installed the exact binary RPM, verified
the package and installed commands, and removed all 19 package-owned files.
The profile and ALSA-state sentinels remained byte-for-byte identical.

The physical profile matrix used the matching release build and user-installed
configuration rather than the temporary RPM payload. It is therefore hardware
evidence for the source milestone, while the disposable-container result is
the exact RPM packaging evidence.

## Embedded feature-compatibility release

The authoritative 54-row `feature-parity.tsv` is now embedded in the Rust
library and drives both `ae5ctl features` and a ninth native GUI page. The
current matrix contains 13 verified features, 13 documented Linux-native
substitutes, 18 functions pending acceptance, and 10 unavailable functions.
The parser and both presentation paths update from that single source.

The complete release build passed 73 Rust/GTK tests, strict Clippy and
formatting, the feature-matrix and ACP validators, AppStream validation, and
the transactional rootless lifecycle. The real user installation was then
upgraded without restarting WirePlumber. Its installed GUI and CLI are
byte-identical to the release inputs:

- GUI SHA-256:
  `3a82e524b73442a5e7718819cde53d91368684ed752f1670f0c4d196f1344a76`
- CLI SHA-256:
  `c3a98075f774e9abab83156b59ad8cd44bb3a5207a082e5a89bf882bb69936c8`

The installed CLI printed all ten unavailable features without requiring an
ALSA device write. Under the real KDE/Wayland session, AT-SPI selected the
installed **Compatibility** page, confirmed all nine sidebar pages and all 28
unavailable/pending feature buttons, expanded Super X-Fi, and read its Linux
mechanism, evidence, remaining gate, and source before closing the frame
through `window.close`.

The full mixer SHA-256 was
`e727bb4a9637af748f485838a09dcad782b14849a144854b586ae33e3d3a31a4`
before and after both source and installed GUI checks. ALSA remained at
Headphone, 2.0, Microphone, raw Master/Front 19/99, PCM 51/255, unused
surround channels zero and muted, and Low gain. PipeWire remained at exactly
20% with the matching duplex route. Its already-running PCM retained the same
PipeWire owner and trigger time; the test did not suspend it or create a
playback stream. No audio command was run.

The fresh Fedora 44 binary RPM SHA-256 is
`ea1c015936c5d03361a964ede57b1b03436469a65193ee4a661aabba80838736`;
the source RPM SHA-256 is
`9dbcf6ab31c7dd2cfeda8c113fa56608050991e17c386f35b664fd7a3f07c1c6`.
The package build repeated all 73 tests and metadata checks. A disposable
Fedora 44 container installed that exact binary RPM, verified that the
hardware-independent unsupported-feature report worked, and removed all 19
package-owned files while preserving profile and ALSA-state sentinels
byte-for-byte.

## Muted-headphone route diagnostics and explicit repair

The shared CLI and GTK route-health path now rejects normal-mode Headphone
output when the required Front DAC is muted or unreadable. The private
diagnostics report runs the same read-only check. Direct Mode skips that
normal-DSP-path requirement.

Reapplying the already-selected Headphone value preserves Front's prior mute
state, so recovery is deliberately separate from the ordinary output setter.
`ae5ctl route-repair` and the GTK Device page's conditional **Repair current
route** button use the same explicit repair plan. There is no automatic or
login-time unmute.

The release passed all 75 Rust/GTK tests, strict Clippy and formatting,
feature-matrix validation, diagnostics self-test, the complete transactional
rootless lifecycle, and ShellCheck with only the standard external
`/etc/os-release` source excluded.

On the physical AE-5, the 20%-ceiling/Low-gain playback preflight passed before
a no-stream negative test. Changing only Front from on to off made both
`route-status` and general status report the muted-DAC failure while the ALSA
Headphone choice, card-specific PipeWire route, and duplex profile still
matched. Restoration returned raw mixer SHA-256
`5f72b79126e713debcc4f975e86cc9ac1bfe1ed39cd4760e4f5f44a5766656bf`
and simple-control SHA-256
`b58ff5fa3cc6ae9271b45720ecd7f66edbdb13b455ba9ea72e1c47e165f49b9b`.
A generated report then recorded the restored route as healthy.

The healthy CLI repair path was a verified no-op. From the guarded
Front-muted fixture, the CLI repair restored Front and the exact raw mixer
hash above. A separate native GTK test found the repair button by the
application's AT-SPI process ID, invoked its accessible action, and observed
Front return before the cleanup guard ran. Both paths kept PipeWire at
`0.20`, opened no PCM, and played no audio.

The real rootless installation was upgraded transactionally. Its installed
payload is byte-identical to the release inputs:

- GUI SHA-256:
  `35ab0fdcdfa50d602d4f5b21550f3023c4d75dc4deb79ed672d247623f7b7f1d`
- CLI SHA-256:
  `87009883ce4be4e4209b29c5e7a1a03e5ad595612b74e9cc5d99d87d6de55412`

The upgrade preserved the raw mixer hash above, exact route-health output,
PipeWire volume `0.20`, and routing-file aggregate SHA-256
`809526b30f188f8f02e501cfbf9b397b471f121ff14369be7905d313bb9e0b9d`.
Every playback PCM remained closed throughout, and no audio was played.

The Fedora 44 RPM was also rebuilt from the milestone worktree. Its `%check`
stage repeated all 75 tests plus ACP, diagnostics, desktop, AppStream, and
udev validation. A clean disposable-container lifecycle found
`route-repair` in the installed CLI, removed all 19 package-owned files, and
preserved the profile and ALSA-state sentinels byte-for-byte.

## Startup-route recovery upgrade

The startup-recovery release extends both WirePlumber profile activation and
ALSA PCM-close polling from one to five seconds. On the physical card, an
immediate post-restart repair exercised the longer path for 3.282 seconds,
re-applied Headphone and Microphone, unmuted Front, and returned all 48 simple
controls to SHA-256
`b58ff5fa3cc6ae9271b45720ecd7f66edbdb13b455ba9ea72e1c47e165f49b9b`.
PipeWire remained exactly `0.20`; Master and Front remained 19/99, PCM 51/255,
and headphone gain Low. No test sound was played.

The rootless installation was upgraded transactionally without restarting the
active desktop audio session. Its app and CLI are byte-identical to the
release build:

- GUI SHA-256:
  `575dbe23bfd24243f554d0add6d927a1c6a53ad02921a175e544a67ef02f6110`
- CLI SHA-256:
  `1323df9afe500f7a152f8aa81cad1377af1e743e7e14127690ff914c10e6fae0`

The corrected boot probe was also installed for the next reboot, with
SHA-256
`59908e9d2aa529a6405973b303102f003019fe313614d3de2b8819934d79b996`.
It reports historical records without Front evidence as unavailable rather
than falsely calling them muted.

The Fedora 44 binary RPM SHA-256 is
`95caeade4aa1403acb7abc60fd611ba100715235f155c4655e6c238bc7b7dfa8`;
the source RPM SHA-256 is
`95cf4195fc6ad7a69cf952178fa8be4747d6df7d9a6e80cfb5f58082e4ba58fd`.
The build repeated all 75 release tests and package metadata checks. A fresh
Fedora 44 container resolved its full dependency set, installed and verified
the binary RPM, removed all 19 package-owned files, and preserved the profile
and ALSA-state sentinels byte-for-byte.

## Equalizer-evidence user upgrade

The release build after the exact Windows SoundCore equalizer trace passed all
76 Rust/GTK/CLI tests, strict Clippy and formatting, the 54-row feature-matrix
validator, and the complete transactional rootless lifecycle. The real user
installation was then upgraded without restarting WirePlumber.

The installed payload is byte-identical to the release inputs:

- GUI SHA-256:
  `cc798139e56a3bd018aa0d17be2de9924afe7ad84ac90b0968a5496318ea7cf6`
- CLI SHA-256:
  `8cf30a3e22d728a8f92faf8b20648b4e5657ab772c4f9b7a8f216a2b8e75b182`

The installed compatibility report contains the new evidence that Command's
UI, profile, indexed-parameter, key, and repository layers pass all ten EQ dB
floats without edge compensation. The upgrade preserved raw mixer SHA-256
`5f72b79126e713debcc4f975e86cc9ac1bfe1ed39cd4760e4f5f44a5766656bf`,
the complete route-health output, PipeWire volume `0.20`, and Low headphone
gain. Every playback PCM remained closed and no audio was played.

## Working PipeWire analog transport

The physical AE-5 normal playback path was isolated independently from the
GTK work. The packaged ACP profile now bypasses HDA-Intel's duplicate
`front:` softvol for stereo and each supported 2.1-through-5.1 mapping. The
exact-card WirePlumber rules select S16 RW playback with 6016-frame periods,
four periods, and ignored driver dB metadata. The live sink reported `hw:0`,
`S16LE`, mmap disabled, period size 6016, and period count four.

The ACP validator asserts every required property. Rust route health also
requires `api.alsa.ignore-dB=true`, so a stale or partial installation fails
closed instead of reporting a healthy route. The guarded physical capture
contained a clean 997 Hz tone where the prior generic PipeWire path measured
only its noise floor. The sink, mixer, route, and Low-gain state were restored
to the 20% project ceiling afterward.

A zero-amplitude installed-profile matrix then opened 2.0, 2.1, 4.0, 4.1,
and 5.1 as exact 2/3/4/5/6-channel S16 RW hardware streams. Every stream used
period size 6016 and buffer size 24064. The test suspended each sink after the
open, restored Headphone/2.0/Microphone and the profile's X-Bass state, and
reproduced complete mixer SHA-256
`26a75bb94621e15023ebb28bb3a3da92c63d210f0e657b74478187256d39142c`.

## Smart Volume mode user upgrade

The 2026-07-26 release build passed all 78 Rust/GTK/CLI tests, strict Clippy
and formatting, the audio-parity self-test, ACP and 54-row feature-matrix
validators, diagnostics self-test, and the complete transactional rootless
lifecycle. Native Wayland measurements passed the Version 1 budgets at 233 ms
startup, 86 ms refresh, 0.00% idle CPU, and 70,416 KiB peak idle RSS.

The real user installation was upgraded without restarting WirePlumber. Its
installed binaries are byte-identical to the release inputs:

- GUI SHA-256:
  `ea6d75c416f37a3343a48ae3b228db89059f5d48d5ac6cc989153dce7c389efa`
- CLI SHA-256:
  `09ccec2ea706451bbf87fc7ed16e0a05ee2b090aa9927aa64711013d7e22aa9f`

With a guarded temporary Night selection, the installed GTK accessibility
tree kept the Smart Volume playback switch and Night mode selector sensitive,
made only `FX: Smart Volume playback level` insensitive, and exposed the
fixed-DSP explanation. Applying the saved 46-control profile restored Normal
mode and complete mixer SHA-256
`743e602e8066bea0aed9145669584497289fdb459c4c8450913513dbb7e15bc1`.
Headphone/Microphone routes remained matched, PipeWire remained at `0.20`,
every PCM was closed, and no audio was played.

## Kernel-candidate evidence upgrade

After the complete Smart Volume kernel candidate passed package and cardless
boot validation, the same transactional rootless installer upgraded the
embedded Compatibility matrix without restarting WirePlumber. The installed
binaries are byte-identical to the new release inputs:

- GUI SHA-256:
  `dc3d3515c9ec32d18a56f4edb1497bec7ff68ca93acd84817378feced2a3b14d`
- CLI SHA-256:
  `0d15545362016e3591618e5e5eed713bab58594afde215bef31243f1e9aaf4a5`

The installed CLI reported the exact 6,326-module RPM, no-audio smoke boot,
cardless full-root boot, signed-module, zero-taint, clean-log, and automatic
fallback evidence while retaining the guarded bare-metal DSP-loss suspend as
the remaining gate. All existing WirePlumber and ACP paths were byte-identical
and retained in place. Before and after the upgrade, complete mixer SHA-256
was
`743e602e8066bea0aed9145669584497289fdb459c4c8450913513dbb7e15bc1`,
Headphone/Microphone routes matched, PipeWire output remained at 20%, Low gain
remained selected, and there were zero playback or capture streams. The custom
kernel was not installed on the host.

## Remaining release gate

This proves clean Fedora dependency resolution and package ownership/removal,
physical-card operation of exact payloads, and a normal-user maintained-LTS
lighting package cycle, plus the unchanged release GUI's native color-dialog
path and a rootless application-menu installation on the development host. It
does not claim a host RPM transaction or visible physical color output.

Before calling Phase 5 complete, install the RPM through an authenticated
development-host package transaction, launch it from the desktop application
menu, exercise a user-approved physical control and visibly confirm one
on-card color, uninstall it, and confirm the already-proven per-user profile
library and lighting configuration plus ALSA state remain intact.
