# AE-5 playback PCM reopen evidence

Current conclusion as of 2026-07-27: the kernel-level reopen defect is fixed
in the maintained patch queue and qualified on the physical AE-5 through
VFIO. Physical-host installation, a true power-removal cold boot, and analog
output acceptance remain separate gates.

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

## Root cause and fix

The standard CA0132 cleanup path cleared the AE-5 HDA playback converter on
every PCM close. Reassigning that converter on a later open corrupted hidden
CA0132 stream state even though the visible stream/router fields remained
correct. Skipping only that converter cleanup on AE-5 made 12/12 immediate
reopens clean.

The first candidate still failed after an idle interval because HDA runtime
autosuspend performed its own stream cleanup. Retaining only the HDA core's
cached stream state caused silence after resume, while globally disabling HDA
power saving proved clean but was too broad. The final patch takes a balanced
runtime-PM reference for the AE-5 codec only and releases it in codec teardown.
This leaves the motherboard codec and global `power_save=10` policy
untouched. System suspend still uses the normal HDA all-stream cleanup.

The implementation is
`kernel/ca0132-ae5-stable-playback-stream.patch`.

## Fixed-candidate qualification

The exact functional module loaded in the guest had SHA-256
`7089c4493d1acf530cca4fefe9860b6df96540c263fcf6874d2ab482f953be68`.
The following internal What U Hear tests used bounded fixtures with the AE-5
analog outputs unplugged:

| Matrix | Result |
|---|---|
| Immediate warm reopen | 12/12 clean, 0.000760% THD |
| Redundant safe controls and mute/unmute | all clean, 0.000760% THD |
| Two 20-second idle intervals, global `power_save=10` | all clean, 0.000760% THD; codec remained active |
| Rejected hardware-OutFX enable | 8/8 clean, 0.000760–0.000761% THD |
| Fresh host-driver to VFIO cycle | first open and 12/12 reopens clean, 0.000829% THD |
| Final reopen qualification | 50/50 clean; zero corrupt and zero silent |
| 48/96 kHz transitions | 12/12 clean; 0.000829–0.000857% THD |
| 2/6-channel transitions | 12/12 clean, 0.000829% THD |

The fresh-cycle test shut the Linux guest down, let managed hostdev rebind the
AE-5 to the host `snd_hda_intel` driver and reload its DSP, then restarted the
guest and passed the card through after VFIO reset. This exercises a fresh
driver/DSP initialization path, but it is still cold-like rather than a
motherboard power-removal cold boot.

Hardware OutFX remains deliberately fail-closed. Its enable request returns
`-EOPNOTSUPP` and readback stays off. Windows Command OutFX is instead the
master for a software APO effect chain, so the rejected Linux hardware
control is not an active-effects parity test.

## Packaged-kernel qualification

The complete side-by-side build produced
`kernel-7.1.4_ae5_stable-1.x86_64.rpm`, SHA-256
`a295451e29ee936095068b47da7c34d565a21fdc0079bc3555b0ad9bd18fbda9`.
Non-installing extraction verified release `7.1.4-ae5-stable`, 6,469 modules,
the required boot, storage, graphics, and HDA configuration, matching signed
CA0132 vermagic, and all current AE-5 source markers.

That exact RPM was installed in the Fedora passthrough guest and booted with
the physical card. Kernel taint was zero, the signed module loaded, the DSP
initialized, global `snd_hda_intel power_save=10` remained in force, and the
AE-5 runtime state remained active. Bounded internal captures then produced:

| Packaged-kernel matrix | Result |
|---|---|
| First playback after full guest reboot | clean, 0.004283% THD |
| Immediate playback reopens | 12/12 clean, 0.004245% THD |
| Playback after 20 seconds idle | clean, 0.004245% THD |
| Exact `numid=25` OutFX enable | rejected with `EOPNOTSUPP`; readback off |
| Reopens after exact rejection | 8/8 clean, 0.004520% THD |

The different absolute THD from the module-development fixture reflects a
newly generated lower-level test tone; every result remains well below the
1% corruption threshold and the reopen groups are internally identical. The
AE-5 outputs remained unplugged. This is a packaged-kernel, fresh-driver VFIO
test, not a physical motherboard power-removal test.

The verified RPM is now installed side by side on the host. Stock
`7.1.4-200.nobara.fc44.x86_64` remains running and saved/default, while
`7.1.4-ae5-stable` is selected for the next boot only. No host reboot occurred
during this qualification.

## Host keepalive proof

The installed exact-card WirePlumber rule sets
`session.suspend-timeout-seconds = 0`. After the first host PCM open, ten
separate `pw-play` clients and ten seconds idle left the PCM `RUNNING`.
`trigger_time` remained exactly `30103.578729341` before the clients, after
client ten, and after the idle interval.

The host test proves that normal desktop clients no longer close/reopen the
hardware PCM. It remains useful defense in depth after the kernel fix. It is
not an analog-output result: the AE-5 outputs were physically unplugged and
the user's headphones were on the motherboard line-out.

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

1. Complete the scheduled one-shot host boot into `7.1.4-ae5-stable` and run
   the fail-closed runtime gate before changing controls.
2. Run a true power-removal cold boot and bare-metal suspend/resume.
3. Capture the reconnected AE-5 analog output safely.
4. Complete a runtime Windows/Linux same-settings capture. Static disassembly
   already establishes that Windows Command OutFX controls the
   `CtxRFX64.dll` software APO chain rather than Linux's hardware OutFX path.
