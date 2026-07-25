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
  configuration files, identifiers, or user profile contents are committed.
- Conversion creates native profiles but does not apply them to hardware.

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

The active speaker selection now converts to a native profile containing 21 ALSA
controls. Its Windows channel mask mapped exactly to the AE-5 `5.1` speaker
choice, and the output route mapped to `Speakers`. The source profile also
enabled Windows Bass. The later exact Bass Management trace described below
established that this toggle maps to speaker bass redirection, not X-Bass, for
that layout.

The active headphone selection now converts to a native profile containing 20
ALSA controls, including the `Headphone` output route. The selected Creative
headphone tuning was retained in the migration report as unsupported because
the current CA0132 ALSA interface has no mapped control for that processing.
This is a known sound-parity gap rather than a silently discarded setting.

Both generated profiles passed `ae5ctl profile-check` against the physical
AE-5 without changing any mixer value. Float equalizer values were rounded to
the nearest representable ALSA step and identified as approximate in the
report. The converter now also selects `FX: Equalizer Preset` Flat before
applying those custom bands, preventing a previously selected factory curve
from surviving underneath them. An aggregate SHA-256 of every file below the
mounted Creative and Creative_Technology_Ltd trees was
`409cbe439f23ca22a378280499cdcad3c1f67999a841235cc7e0899bb8913f9f`
both before and after the 2026-07-25 conversion rerun.

## Offline native-library round trip

The installed `ae5ctl` later repeated both active imports under disposable
`HOME` and `XDG_CONFIG_HOME` directories. The speaker conversion produced 21
controls and reported 24 exact, 2 approximate, and 9 unsupported mappings.
The headphone conversion produced 20 controls and reported 17 exact, 8
approximate, and 8 unsupported mappings. Both profiles contained numeric
values for all ten `EQ Band0` through `EQ Band9` controls and selected
`FX: Equalizer Preset` Flat before those custom bands.

Static inspection of the exact installed Command UI later established that
its separate Bass and Treble equalizer sliders are aliases for band index 1
(62 Hz) and band index 8 (8000 Hz), not additional persisted settings. Those
values are therefore already preserved by `EQ Band1` and `EQ Band8`; no
migration field is missing. The evidence boundary and binary hashes are
recorded in [`SOURCE_INVENTORY.md`](SOURCE_INVENTORY.md).

The isolated profile library then:

1. discovered both imported profiles;
2. renamed the headphone profile with surrounding whitespace removed;
3. exported both library entries to new standalone files; and
4. compared each export byte-for-byte with its corresponding library file.

Every step passed. No profile was applied or checked against ALSA, and no
audio stream was opened. The temporary profiles and reports were deleted
after their derived counts were recorded. A before/after aggregate over the
exact active input scope was identical, and the broader Creative and
Creative_Technology_Ltd aggregate remains
`409cbe439f23ca22a378280499cdcad3c1f67999a841235cc7e0899bb8913f9f`.
This completes the real custom-EQ native-library round-trip gate without
publishing the user's profile names, identifiers, or tuning values.

## Inactive unsupported-feature settings

A later schema audit separated two inactive settings from genuinely
unsupported active behavior. Both exact source targets contain a complete
Scout object with every enable flag set to false and
`Bass.SubWooferGain` set to false. Reproducing either state requires no Linux
control, so the importer now records them as exact no-ops. A configured Scout
object or `SubWooferGain` set to true remains explicitly unsupported; focused
tests cover both refusal paths.

Fresh mounted-user discovery with the rebuilt converter produced the same 21
speaker and 20 headphone controls. The speaker report improved from 24 exact,
2 approximate, and 9 unsupported items to 26 exact, 2 approximate, and 7
unsupported items. The headphone report improved from 17 exact, 8
approximate, and 8 unsupported items to 19 exact, 8 approximate, and 6
unsupported items.

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

A fresh mounted-user conversion produced the same 21 speaker and 20 headphone
controls. The speaker report improved from 26 exact, 2 approximate, and 7
unsupported items to 31 exact, 2 approximate, and 2 unsupported items. The
headphone report improved from 19 exact, 8 approximate, and 6 unsupported
items to 24 exact, 8 approximate, and 1 unsupported item. At that stage, the
remaining warnings were active behavior: the selected speaker tuning plus the
LFE/X-Bass conflict for speakers, and the selected Creative headphone tuning
for headphones.

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

A fresh mounted-user conversion retained the same 21 speaker and 20 headphone
controls. The speaker report improved from 31 exact, 2 approximate, and 2
unsupported items to 32 exact, 2 approximate, and 1 unsupported item; the
then-remaining warning was the active LFE/X-Bass conflict. The headphone counts
remain 24 exact, 8 approximate, and 1 unsupported because the selected
driver/APO tuning still has no verified Linux equivalent, but that warning now
includes its display model. The conversion used disposable output and
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

A fresh mounted-user conversion produced a 23-control speaker profile with 34
exact, 1 approximate, and 0 unsupported mappings. The resulting profile passed
read-only validation against the physical AE-5. The unchanged headphone
conversion retained 20 controls with 24 exact, 8 approximate, and 1
unsupported mapping for the selected driver/APO headphone tuning.

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
