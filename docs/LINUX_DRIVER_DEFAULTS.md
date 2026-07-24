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

The backup uses create-new semantics and is never overwritten. If backup
creation fails, no mixer write occurs. The existing profile transaction then
validates the projected final state, applies through typed ALSA controls,
verifies readback, and rolls back all targeted controls if a write fails.

## Validation status

Automated tests verify every source-derived value, every exclusion, the
LFE-safe X-Bass adaptation, and that a backup failure produces zero writes.
On the physical target, `linux-defaults-check` validated all 29 controls
against the live AE-5 without changing hardware.

The reset itself has not been invoked during development. One
user-authorized reset, backup restore, and audio check remains before calling
the physical apply path verified.
