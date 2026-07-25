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
enabled Windows Bass. Linux CA0132 cannot enable X-Bass with an LFE channel,
so the importer retained that setting as unsupported and added an explicit
X-Bass-off transition before the route change instead of generating an
unapplicable profile.

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
