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
- Source Windows files are read-only inputs. No Creative binaries, firmware,
  configuration files, identifiers, or user profile contents are committed.
- Conversion creates native profiles but does not apply them to hardware.

## Real-installation result

Selecting the mounted Windows user folder found Command 3.5.10.0 instead of
the older 3.4.98.0 config and found the single installed AE-5 product
directory without requiring either internal path from the user. The migration
report now identifies 3.5.10.0 as the selected active configuration, making
that automatic choice visible in both the CLI and desktop preview.

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
mounted Creative and Creative_Technology_Ltd trees was identical before and
after both conversions.
