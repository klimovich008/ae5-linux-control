# AE-5 playback PCM reopen evidence

Current conclusion as of 2026-07-27: the immediate production path is
mitigated, but the kernel-level reopen defect is not fixed.

## Reproducible failure

The exact AE-5 was passed through to a Fedora guest running
`7.2.0-rc2-ae5-integrated+`. A bounded 1 kHz fixture was played through normal
raw ALSA and captured through What U Hear. No mixer control was written and
OutFX remained off.

Eight playback close/reopen trials produced:

| Trial | THD |
|---:|---:|
| 1 | 5.19850% |
| 2 | 26.40650% |
| 3 | 11.90873% |
| 4 | 0.00070% |
| 5 | 26.40744% |
| 6 | 26.40777% |
| 7 | 26.40720% |
| 8 | 26.40595% |

The approximately 26.4% state is deterministic in shape, with large even
harmonics and sample discontinuities every 16 frames. `snoop=1`,
`position_fix=1/2`, alternative period geometry, S32, open order, a candidate
ASI value, prepare-time stream setup replay, and the 2.0 speaker route did not
remove it.

## Persistent-playback control

Keeping one playback PCM open while reopening only What U Hear produced ten
clean captures at 0.00071–0.00082% THD. This separates playback close/reopen
from capture reopen.

The final guard module had SHA-256
`879ceff1d14d1e26fa1d54a199d22c19ff307a60312322ca773fd596e9c576cc`.
Two guest starts with different boot IDs then held one playback PCM open:

| Guest start | Captures | THD range |
|---|---:|---:|
| Managed PCI reset and boot | 10 | 0.00073–0.00081% |
| Full guest shutdown/start | 5 | 0.00064–0.00081% |

During the first run, raw `amixer cset` of OutFX on failed with
`Operation not supported`; three redundant off requests succeeded without
disturbing the held stream. OutFX read back off before and after.

These starts are cold-like PCI-reset tests, not a physical motherboard
power-removal cold boot.

## Host keepalive proof

The installed exact-card WirePlumber rule sets
`session.suspend-timeout-seconds = 0`. After the first host PCM open, ten
separate `pw-play` clients and ten seconds idle left the PCM `RUNNING`.
`trigger_time` remained exactly `30103.578729341` before the clients, after
client ten, and after the idle interval.

The host test proves that normal desktop clients no longer close/reopen the
hardware PCM. It is not an analog-output result: the AE-5 outputs were
physically unplugged and the user's headphones were on the motherboard
line-out.

## Production guards

- `kernel/ca0132-ae5-disable-unsafe-outfx.patch` initializes output effects
  off, rejects hardware OutFX enable, and avoids redundant off replay.
- The Rust backend rejects hardware OutFX, child output effects, hardware EQ,
  Direct Mode, and output/profile transitions before any ALSA write.
- Profile JSON retains those values for migration and software processing but
  filters them from hardware apply.
- The WirePlumber rule keeps stable S16 playback open.

A real-card application audit attempted OutFX on/off, Crystalizer on, the
current output selection, and the current speaker layout. All five commands
failed before writing; the complete ALSA mixer SHA-256 remained
`a5fea602e5dd3cc3e1d0bf3a1492e4644171e9bf81d1a5035bd71ca29e60bc2d`.

## Remaining acceptance

1. Find and fix the CA0132/HDA operation that corrupts playback on reopen.
2. Build and install the current guarded kernel on the physical host.
3. Run a true power-removal cold boot.
4. Capture the reconnected AE-5 analog output safely.
5. Complete a runtime Windows/Linux same-settings capture. Static disassembly
   already establishes that Windows Command OutFX controls the
   `CtxRFX64.dll` software APO chain rather than Linux's hardware OutFX path.
