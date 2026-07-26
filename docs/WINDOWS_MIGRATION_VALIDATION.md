# Windows settings migration validation

Validated against a mounted Sound Blaster Command 3.5.10.0 installation and
its active user-owned AE-5 profile and equalizer files.

## Safety boundary

- The importer reads only named `serializeAs="String"` values from
  `user.config`.
- It never parses .NET `BinaryFormatter` values such as `LastAEStates` or
  saved audio-format state.
- Selected profile and equalizer IDs accept only ASCII letters, digits,
  hyphens, and underscores before they are joined to the AE-5 product path.
- Mounted-user discovery scans only the expected AppData directories, selects
  the newest numeric Command version, limits each directory to 512 entries,
  ignores symlinked candidates, and rejects multiple newest configs or AE-5
  product directories.
- A validated `SelectedHpEq` identifier may read one regular UTF-8 file from
  the exact `ProgramData/Creative/SBCommand/Product/AE5/SpeakerEqConfigs`
  fallback when the per-user cache omits display metadata. The common 1 MiB
  source limit still applies.
- On a complete Windows volume, driver discovery compares the installed
  `CtxHda.sys` with at most 16 matching packages in a 16,384-entry-bounded
  DriverStore scan. It reads `DriverVer` only from the uniquely matching INF
  after confirming the exact AE-5 hardware ID. It does not execute, decompile,
  or disassemble the driver.
- Source Windows files are read-only inputs. No Creative binaries, firmware,
  artwork, raw configuration files, or user profile contents are committed.
  A separate generated catalog contains only the factory profile names,
  identifiers, and representable native AE-5 control values.
- Conversion creates native profiles but does not apply them to hardware.

## Embedded factory profile catalog

The exact AE-5 product tree contains 33 selectable factory Sound Effects
profiles plus one non-selectable base template. Every selectable profile has a
speaker section and a headphone section. The validated importer converted all
33 pairs to the native profile schema, including the one Call of Duty profile
whose profile and EQ filenames differ but whose source identifiers match.

The GTK Sound Effects page exposes the converted defaults as immutable cards.
Application uses the live output route to select the correct section and does
not change that route. For 2.1, 4.1, or 5.1 speakers, the existing
layout-aware migration rule maps the shared Windows Bass setting to CA0132
Bass Redirection and turns X-Bass off before normal profile validation,
readback, and rollback.

The embedded catalog is generated interoperability data, not a copy of
Command's presentation layer. Creative images, descriptions, file paths, raw
JSON, executables, and user-created settings remain outside the repository.
The source snapshot and generated catalog hashes are recorded in
[`data/README.md`](../data/README.md). Focused tests require 33 unique names
and identifiers, validate both variants of every entry, pin known
speaker/headphone differences, and cover LFE adaptation without a route
change.

## Exact Command export equivalence

The managed export paths in the user's exact Sound Blaster Command 3.5.10.0
installation were inspected offline to answer one narrow interoperability
question. `ExportProfile` copies the selected profile's stored `FilePath`
directly to the destination chosen in the save dialog, with replacement
enabled. `ExportPreset` does the same for the selected equalizer preset. The
destination files are not reserialized or transformed. Command's subsequent
JSON parsing and metadata changes apply only to the application's internal
profile copy.

Consequently, the active stored profile and EQ files consumed by the Linux
importer are byte-for-byte the same payloads that this Command version writes
through its export UI. The corresponding import paths also parse those JSON
files through Command's normal profile and EQ deserializers before performing
its product and duplicate checks.

The inspected `Creative.SBConnect.UI.Framework.dll` has SHA-256
`06e7a61c95392fe76ec59d4a1ef1c5a8c465b07dd8c7d7b5256c2ce7ab109e3e`.
The active profile input was 2,106 bytes with SHA-256
`bbf23d3348e25f61e98be3d5ffe43a10fea4daf73c3d649433f5e489c8b0588f`;
the active EQ input was 1,214 bytes with SHA-256
`fd63464944e3f55816d65ce1759858b628ffc127475478681a69e7e54d43bfeb`.
These hashes identify the evidence without publishing profile names,
identifiers, contents, or Creative binaries. No proprietary binary,
disassembly, or source-derived code is stored in this repository.

## Real-installation result

Selecting the mounted Windows user folder found Command 3.5.10.0 instead of
the older 3.4.98.0 config and found the single installed AE-5 product
directory without requiring either internal path from the user. The migration
report now identifies 3.5.10.0 as the selected active configuration.

The installed `Windows/System32/drivers/CtxHda.sys` matched only
`ctxhda.inf_amd64_f05837e20abd2faf` in DriverStore. Its INF contains the exact
AE-5 ID `PCI\VEN_1102&DEV_0012&SUBSYS_00511102` and declares driver
`6.0.105.0065` dated 2022-11-24. The active `oem33.inf` is byte-identical to
that package's INF, and archived Windows setup logs repeatedly select the same
package. Command and driver versions now appear first in both the CLI report
and desktop preview.

Live Command validation later established that the profile section type is
`0` for headphones and `1` for speakers. With that mapping, the active speaker
selection converts to 22 ALSA controls and reports 31 exact mappings, one
approximate mapping, and no unsupported setting. Its Windows channel mask maps
to the AE-5 `5.1` choice and its output route maps to `Speakers`. Its effect
states, levels, and ten flat EQ bands match the values Command displayed for
the speaker route.

The active headphone selection converts to 21 ALSA controls and reports 23
exact mappings, eight values rounded to representable ALSA steps, and one
unsupported setting. It includes the `Headphone` output route, the visible SBX
states and levels, and all ten custom EQ bands. The only unsupported item is
the selected Creative headphone model tuning because the current CA0132 ALSA
interface has no mapped equivalent. This is a known sound-parity gap rather
than a silently discarded setting.

Earlier generated profiles passed `ae5ctl profile-check` against the physical
AE-5 without changing a mixer value. The corrected profiles are structurally
valid and are stored in the private native profile library, but their final
read-only hardware check waits until the card returns from the Windows guest.
The converter selects `FX: Equalizer Preset` Flat before applying custom bands,
preventing a previously selected factory curve from surviving underneath them.
The source aggregate remained unchanged during conversion.

## Isolated Windows import round trip

The source settings were also imported into an isolated Windows 11 Enterprise
Evaluation VM to validate the native Command side of the migration boundary.
The source NTFS partition remained read-only on the Linux host and was never
attached to the VM. A private read-only transfer ISO contained exactly 126
files totaling 183,478 bytes. Every relative path, length, and SHA-256 matched
the source before the disc was ejected.

Sound Blaster Command was installed from Creative-signed media and updated
from Creative's live release endpoint to `3.5.10.0`, the exact configuration
version being imported. The installed executable has SHA-256
`32c71d5ad40f5d3cc1bb35f756038e3de5c08e3291550f26ac9fa1cb1cabff58`
and is byte-identical to the executable in the inspected source installation.
The verified setting tree was copied into Command's actual versioned user,
AE-5 product, and shared metadata locations without replacing any
non-identical destination. The first no-device launch left all 126 files
byte-for-byte identical, but that was not sufficient live-device validation.

Managed PCI passthrough then assigned the physical AE-5 to the system Windows
guest. Both Creative PnP nodes were healthy with the Creative-signed
`6.0.105.65` driver, and the expected Creative services were running. Command
initially detected the card in its log but displayed an unsupported-device
view because the read-only transfer medium had propagated its file attribute
to 93 destination user files, including `user.config`. Clearing only the
ReadOnly and Hidden attributes on those destination copies fixed startup.
Their contents and aggregate SHA-256 did not change; the 33 shared metadata
files were already writable and byte-identical to the installed copies.

After a fresh launch, Command recognized the AE-5 and exposed the complete
device UI. Of the 126 imported files, 125 remained byte-identical. Command
legitimately updated only `user.config`: it added the selected speaker audio
format cache and changed the saved surround-feature runtime state. The active
profile and preset identifiers, both route-specific selections, and every
plain setting consumed by the importer remained unchanged.

The speaker view displayed the section whose profile type is `1`; switching to
headphones displayed the section whose type is `0`. Narrow reflection over
Creative's managed profile assembly independently confirmed the enum
`Headphones = 0` and `Speakers = 1` in the profile, firmware-profile, and
audio-profile settings types. No Creative binary or reflection output is
committed. A focused Rust regression test fixes these values at the selection
boundary.

Windows playback safety was enforced independently at the endpoint layer:
both render endpoints were capped at exactly 20% and muted, all AE-5 outputs
were physically unplugged, and no audio was played. Command automatically
unmuted the endpoint during a Speakers-to-Headphones route change. The test
immediately restored 20% plus mute and verified it independently. Every future
Windows route, profile, or device transition must therefore reapply and
re-verify both the endpoint cap and mute before playback.

This proves that the saved configuration can be transferred into a clean
matching Command installation and selected after physical device discovery.
It does not yet prove equal Windows and Linux analog response; that remains
behind the guarded at-or-below-20% capture procedure. No Windows image,
Creative binary, user setting, private identifier, or VM credential is
committed.

## Offline native-library round trip

After the live output-type correction, the installed `ae5ctl` repeated both
active imports from the exact verified transfer tree. The speaker conversion
produced 22 controls and reported 31 exact, one approximate, and zero
unsupported mappings. The headphone conversion produced 21 controls and
reported 23 exact, eight approximate, and one unsupported mapping. Both
profiles contain numeric values for all ten `EQ Band0` through `EQ Band9`
controls and select `FX: Equalizer Preset` Flat before those custom bands.

Static inspection of the exact installed Command UI later established that
its separate Bass and Treble equalizer sliders are aliases for band index 1
(62 Hz) and band index 8 (8000 Hz), not additional persisted settings. Those
values are therefore already preserved by `EQ Band1` and `EQ Band8`; no
migration field is missing. The evidence boundary and binary hashes are
recorded in [`SOURCE_INVENTORY.md`](SOURCE_INVENTORY.md).

The private profile library then:

1. discovered both imported profiles;
2. displayed each profile and its expected target without hardware access; and
3. retained both output files with owner-only permissions.

No profile was applied or checked against ALSA because the card remained
assigned to Windows, and no audio stream was opened. The exact input tree was
read-only throughout. This completes the corrected custom-EQ native-library
import gate without publishing the user's profile names, identifiers, or
tuning values.

## Inactive unsupported-feature settings

A later schema audit separated two inactive settings from genuinely
unsupported active behavior. Both exact source targets contain a complete
Scout object with every enable flag set to false and
`Bass.SubWooferGain` set to false. Reproducing either state requires no Linux
control, so the importer now records them as exact no-ops. A configured Scout
object or `SubWooferGain` set to true remains explicitly unsupported; focused
tests cover both refusal paths.

Focused fixtures verified that disabled Scout settings and subwoofer gain are
classified as exact no-ops while enabled values remain visible warnings. The
final live-device counts are recorded above; earlier development counts used
an unverified output-type assumption and are not acceptance evidence.

The rerun used disposable output and configuration directories, applied no
profile, and opened no audio stream. Before and after SHA-256 values were
identical for the active Command configuration
(`75e18eee256ad8e330df7470a1c35e3eef688fb1d2beffe6fda3f13add567eb3`),
profile
(`bbf23d3348e25f61e98be3d5ffe43a10fea4daf73c3d649433f5e489c8b0588f`),
and EQ
(`fd63464944e3f55816d65ce1759858b628ffc127475478681a69e7e54d43bfeb`).

## AE-5-irrelevant serialized defaults

Scoped offline inspection of the exact product and profile assemblies then
established that `SpeakerMethod` selects a Windows routing API rather than an
acoustic setting. It also established that Command reads or writes
`Surround.Mode`, `DialogPlus.Mode`, and `SVM.PlusMode` only for a device named
`Katana`, not for the AE-5. The regular `SVM.Mode` is independent and remains
mapped to the Linux Smart Volume setting.

All 34 shipped AE-5 profiles omit the three product-specific modes and
`SpeakerMethod`; all 44 shipped EQ presets also omit `SpeakerMethod`. The
active user copies serialize zero for these five fields. The converter now
records those zero values as exact no-ops while retaining any nonzero value as
unsupported for conservative review. Focused tests cover both paths. Binary
hashes and the inspection boundary are recorded in
[`SOURCE_INVENTORY.md`](SOURCE_INVENTORY.md).

Focused fixtures verified these AE-5-irrelevant defaults and retained
non-zero values as conservative warnings. The final live-device conversion
leaves no unsupported speaker setting and only the selected Creative
headphone tuning unsupported.

The rerun used disposable `HOME`, configuration, report, and output paths. It
did not apply either profile or open an audio stream. An aggregate hash over
the active configuration, speaker/headphone profiles, and speaker/headphone EQ
files was identical before and after:
`a2c9f7d4a3491c045d07b42df87ea1c9ed6a2bc2484937717ce51925100595c0`.

## Speaker category and headphone model metadata

Scoped inspection of the exact Command speaker view model established that
`SelectedSpeakerType=Desktop` is a UI crossover template, not an independent
device effect. Its processing path writes the X-Bass crossover that Command
persists independently as `Bass.XOver`; the importer already maps that source
field. The Desktop selection is now reported as an exact no-op. Other speaker
categories remain conservative warnings.

The exact AE-5 `SpeakerEqConfigs` directory contains 33 tiny text records with
only `model` and `order` metadata. Command uses the filename/index to select a
SpeakerEQ preset from the Windows SoundCore or APO backend; the text file
contains no curve or coefficient that Linux can import. The converter now
validates the selected identifier and reads the bounded model line so the one
remaining headphone warning names the selected hardware model. It does not
map that metadata to the graphic EQ or touch the card. Full hashes and the
inspection boundary are recorded in
[`SOURCE_INVENTORY.md`](SOURCE_INVENTORY.md) and
[`HEADPHONE_TUNING_INVESTIGATION.md`](HEADPHONE_TUNING_INVESTIGATION.md).

Fresh conversion still identifies the selected driver/APO tuning by display
model while leaving it unsupported. The conversion used disposable output and
configuration paths and opened no audio stream. An aggregate over the five
active inputs plus the selected ProgramData metadata config was identical
before and after:
`a62b4ab1ce65c5bcdd80829e07bc028710e5ea9c6675ba966e6b3a4bf48d7eaf`.

## LFE Bass Management mapping

Scoped inspection of the exact Command profile and device-feature assemblies
established that the shared Bass feature changes its backend according to the
speaker channel mask. For headphones and speaker layouts without a subwoofer,
its toggle, strength, and crossover target X-Bass. When the Windows subwoofer
bit is present, the same toggle targets Bass Management and the crossover
targets the Bass Management frequency parameter. Command does not retrieve the
X-Bass strength for that route because it is inactive.

CA0132 already exposes the matching `Bass Redirection` and
`Bass Redirection Crossover` controls and uses the same 10 Hz crossover steps.
The active importer now maps Windows 2.1, 4.1, and 5.1 Bass state to those
controls, writes X-Bass off before changing the route, and skips the inactive
strength value. Headphone and non-LFE speaker imports continue to use X-Bass.
The exact assembly and analysis-tool hashes and the interoperability boundary
are recorded in [`SOURCE_INVENTORY.md`](SOURCE_INVENTORY.md).

The Bass Management implementation passed focused conversion tests and an
earlier read-only check against the physical AE-5. In the corrected live
import, the speaker's inactive Bass state is represented without an
unsupported warning. The headphone path continues to leave only the selected
driver/APO tuning unsupported.

The conversion used disposable Linux output and configuration directories,
did not apply either profile, and opened no audio stream. A deterministic
aggregate over the mounted Command configuration, product data, and the 33
headphone metadata records was identical before and after:
`920077c5a07bf682066116c29a1b6bf22b6b46a86684a7ebaccfbabc9a746174`.
The physical card's raw and simple mixer hashes also remained
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`
and
`65a1da375bd1e6d523a91ee819fa1d8e88f63a34afc10b8e1ef56c736cc38a25`,
respectively, and every PCM substream remained closed.
