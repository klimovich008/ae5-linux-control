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
- Source Windows files are read-only inputs. No Creative binaries, firmware,
  configuration files, identifiers, or user profile contents are committed.
- Conversion creates native profiles but does not apply them to hardware.

## Real-installation result

The active speaker selection converted to a native profile containing 21 ALSA
controls. Its Windows channel mask mapped exactly to the AE-5 `5.1` speaker
choice, and the output route mapped to `Speakers`.

The active headphone selection converted to a native profile containing 19
ALSA controls, including the `Headphone` output route. The selected Creative
headphone tuning was retained in the migration report as unsupported because
the current CA0132 ALSA interface has no mapped control for that processing.
This is a known sound-parity gap rather than a silently discarded setting.

Both generated profiles passed `ae5ctl profile-check` against the physical
AE-5 without changing any mixer value. Float equalizer values were rounded to
the nearest representable ALSA step and identified as approximate in the
report.
