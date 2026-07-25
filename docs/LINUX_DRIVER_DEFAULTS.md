# AE-5 Linux driver defaults

AE-5 Control provides a source-derived reset for the processing controls
initialized by Linux's CA0132 driver. It is deliberately named **Linux driver
defaults**, not **Creative factory defaults**: Sound Blaster Command's reset
contract is undocumented, so claiming identical behavior would be
unverifiable.

## Authoritative source

The baseline was derived from the exact CA0132 source corresponding to the
reference system's Linux 7.1.4 kernel:

- file: `sound/hda/codecs/ca0132.c`;
- source SHA-256:
  `7b61bcb02c4079b9ca6c82cde3147e95706cdbe958324ae383e7875d9a33a4f0`;
- hardware: PCI `1102:0012`, subsystem `1102:0051`.

`ca0132_effects[]` and `ae5_setup_defaults()` define the DSP values.
`effect_slider_defaults[]` defines the five output effect levels.
`ca0132_init_chip()` defines the processing-master, crossover, VoiceFX, and
speaker-range caches. The zero-index enum entries define Flat EQ, Normal Smart
Volume, Neutral VoiceFX, low headphone gain, and the slow-roll-off DAC filter.
The tuning table defines a 30-degree Voice Focus wedge, SVM level 74, and
zero-dB EQ bands; ALSA represents a zero-dB band as level 24.

The resulting 29-control baseline is:

| Group | Linux driver target |
| --- | --- |
| Output master | Enable OutFX |
| Surround | on, level 67 |
| Crystalizer | on, level 65 |
| Dialog Plus | off, level 50 |
| Smart Volume | on, level 74, Normal |
| X-Bass | on, level 50, crossover 8 (80 Hz) |
| Equalizer | off, Flat, all ten bands at 24 (0 dB) |
| Input master | Disable InFX |
| Voice Focus | on, wedge 30 degrees |
| Mic SVM | off, level 74 |
| Noise Reduction | on |
| VoiceFX | Neutral |
| Headphone gain | Low (16–31 ohms) |
| DAC filter | Slow Roll Off |

X-Bass is forced off instead of on when the preserved live route is Speakers
with a 2.1, 4.1, or 5.1 layout. The CA0132 driver does not allow X-Bass on a
speaker layout that already has an LFE channel. The layout itself is not
changed.

The driver initializes some ALSA effect-switch cache entries from DSP request
numbers rather than from `def_vals[0]`. That can make the initial mixer display
disagree with the DSP defaults. This baseline follows the actual DSP default
values and writes through the normal ALSA callbacks, bringing the cache and DSP
state into agreement.

## Intentionally preserved

Resetting processing does not guess or change:

- output selection or headphone auto-detect;
- input selection or microphone boost;
- speaker layout, full-range flags, or bass redirection;
- playback/capture volumes, balances, or mutes;
- PipeWire default devices or sample-rate configuration.

These values describe the user's wiring, speakers, headphones, and desktop
audio policy. Resetting them could reroute, mute, or unexpectedly change the
level of an otherwise working card.

## Safety and recovery

The CLI can print the complete target and validate it against live hardware
without writing:

```sh
cargo run -- linux-defaults-show
cargo run -- linux-defaults-check
```

Applying requires an explicit confirmation flag and a new backup path:

```sh
cargo run -- linux-defaults-apply before-reset.json --confirm
```

Restore that backup with the normal transactional profile command:

```sh
cargo run -- profile-apply before-reset.json
```

Add `--allow-high-gain` only when the backup explicitly contains the High
(150–600 ohms) headphone-gain choice.

The desktop's **Profiles → Linux driver defaults** action provides the same
preview with Cancel selected by default. After confirmation, it saves the
current valid mixer state in the native profile library before the first
write.

Before a reset can create the backup or write a control, the same preflight
used by `linux-defaults-check` captures the live mixer and proves that every
targeted current field can be represented by a profile and restored through
the live driver's advertised choices and ranges. A factory EQ preset is the
only intentional exception: selecting that preset restores its complete
driver curve, so stale individual band values are not captured or required.

This guard matters when a driver exposes an internally initialized value
outside the range advertised by its ALSA control. Such a value cannot safely
round-trip through a normal profile. The reset refuses before any write rather
than producing an incomplete recovery file.

The backup uses create-new semantics and is never overwritten. If preflight or
backup creation fails, no mixer write occurs. The existing profile transaction
then validates the projected final state, applies through typed ALSA controls,
verifies readback, and rolls back all targeted controls if a write fails.

## Validation status

Automated tests verify every source-derived value, every exclusion, the
LFE-safe X-Bass adaptation, factory-preset recovery, and that both an
incomplete backup and a backup-file failure produce zero writes.

On the physical stock-kernel target, the driver currently reports
`Wedge Angle=11` while advertising a valid range of `20..180`. Both
`linux-defaults-check` and a confirmed apply correctly refused with a
restorable-backup error. The apply created no backup file. Complete ALSA
control, simple-control, desktop-route, and PCM-state snapshots were
byte-identical before and after, and no PCM was opened.

The successful physical apply path was then tested through managed VFIO on
maintained kernel `6.18.40-ae5-lts-rgb+`, where Wedge initialized to the valid
value `30`. A controlled pre-reset state used Wedge `20`, Surround off, low
headphone gain, and Master `0`. Applying defaults changed Wedge to `30` and
enabled Surround while preserving Master `0`. The automatically saved
47-control profile restored the complete raw and simple mixer snapshots
byte-for-byte.

With defaults active, a two-second 997 Hz fixture whose peak was 10% of
digital full scale was played at Master `0` and low gain. The physical output
therefore remained effectively silent and well below the 20% safety ceiling.
The card's digital What U Hear PCM captured an exact 997 Hz FFT peak, proving
that the reset left the hardware audio path operational. All PCMs closed
afterward. The guest's original mixer state and the host's mixer, desktop
routes, WirePlumber files, and PCM state also restored byte-for-byte. The DSP
initialized once, no unit failed, and no relevant driver warning appeared.

This verifies the guarded Linux-default reset and recovery path on the
physical AE-5. Creative reset semantics remain intentionally unclaimed unless
reproducible vendor evidence becomes available.
