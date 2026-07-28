# AE-5 playback PCM reopen evidence

Current conclusion as of 2026-07-27: the kernel-level reopen defect is fixed
in the maintained patch queue and qualified on the physical AE-5 through VFIO
and a true motherboard power-removal boot. Analog-output and suspend/resume
acceptance remain separate gates.

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

The verified RPM was installed side by side on the host. Stock
`7.1.4-200.nobara.fc44.x86_64` remains saved/default; the accepted one-shot
test boot runs `7.1.4-ae5-stable`.

## Reproducible host acceptance harness

[`scripts/check-ae5-playback-stability.sh`](../scripts/check-ae5-playback-stability.sh)
turns the packaged-kernel matrix into one fail-closed command. It:

- runs the exact kernel identity, taint, signed-module, PCI, LED, Direct Mode,
  and OutFX-off runtime gate before changing state;
- requires `AE5_ANALOG_OUTPUTS_UNPLUGGED=1` as an explicit physical-topology
  acknowledgement;
- hard-mutes Master and Front, selects Low headphone gain, and stops only the
  desktop audio services that were active before the run;
- generates a four-second 1000 Hz S16 fixture at −30 dBFS and captures the
  exact What U Hear PCM as S32;
- opens and closes the exact normal analog playback PCM with the qualified
  6016/24064 frame geometry;
- rejects silence, clipping, or THD above 1%; and
- discovers the unique raw `Enable OutFX Playback Switch`, requires an enable
  request to fail with `EOPNOTSUPP`, and verifies simple-control readback
  remains off.

The companion [`tools/tone-thd.py`](../tools/tone-thd.py) analyzer has a
cardless self-test that distinguishes a clean generated fixture from one with
an injected 20% second harmonic.

A fresh managed-VFIO boot of the physical card into
`7.1.4-ae5-stable` first passed a shortened first-open/warm/idle/rejected-OutFX
smoke. The final harness was then run from a second fresh managed boot and
produced:

| Harness group | Captures | THD range |
|---|---:|---:|
| First open | 1 | 0.003327% |
| Immediate warm reopens | 12 | 0.003304–0.003352% |
| Reopen after 20 seconds closed | 1 | 0.003335% |
| Reopens after rejected OutFX enable | 8 | 0.003331–0.003335% |

All 22 final-run captures had the same 3.130257% internal peak. Kernel taint
remained zero. The first playback emitted one generic HDA controller
information line activating its existing IRQ timing workaround and suggesting
a larger `bdl_pos_adj`; it did not recur during the following 21 captures. No
waveform fault accompanied it.

After clean guest shutdown, the card rebound to host `snd_hda_intel`, both
system guests were off, all user audio services were active, the AE-5 sink was
5% and muted, Master and Front were off, gain was Low, and both playback PCMs
were closed. The host raw and simple mixer snapshots were byte-identical to
their pre-cycle files. This remains cold-like managed PCI reset evidence, not
a physical motherboard power-removal cold boot.

## Bare-metal power-removal qualification

The host was fully shut down and motherboard power was removed before booting
the installed `7.1.4-ae5-stable` package. The runtime gate accepted the exact
untainted release, signed matching CA0132 module, AE-5 PCI identity and
`snd_hda_intel` binding, five LED interfaces, Direct Mode absence, and OutFX
off.

The fail-closed host harness then produced:

| Bare-metal group | Captures | THD range | Peak |
|---|---:|---:|---:|
| First open | 1 | 0.002724749% | 3.130447865% |
| Immediate warm reopens | 12 | 0.002720620–0.002724749% | 3.130447865% |
| Reopen after 20 seconds closed | 1 | 0.002720620% | 3.130447865% |
| Reopens after rejected OutFX enable | 8 | 0.002720620–0.002724749% | 3.130447865% |

All 22 captures passed the 1% corruption threshold. The exact OutFX enable
request returned `EOPNOTSUPP` and readback remained off. There were no new
kernel audio warnings. Cleanup restored the AE-5 sink to 5% muted, Master and
Front off, Low gain, OutFX off, and both playback PCMs closed. Compact local
evidence is under
`~/.cache/ae5-control/host-stability-20260727-230047.JgWYEe`; raw captures
remain outside Git.

The harness initially stopped before playback because the general runtime
gate invoked the connected-headphone routing preflight, whose requirements
conflict with the harness's deliberately unplugged and hard-muted topology.
The harness now supplies an exact-card hardware-only runtime snapshot. The
gate reports `routing_preflight=not-run` instead of claiming route acceptance;
connected-headphone cold/suspend campaigns still require the full routing
preflight.

The user also reported that, before this cold boot, the same audio fault was
audible after warm-booting into Windows and disappeared only after complete
power removal. This was not an instrumented Windows capture, but it rules out
a Linux/PipeWire-only explanation for that incident and is consistent with
AE-5 DSP or PCI state persisting across a warm OS switch.

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
- The Rust backend always rejects hardware OutFX, child output effects,
  hardware EQ, and Direct Mode before any ALSA write. Direct output-route
  controls are admitted only on the exact clean `7.1.4-ae5-stable` kernel;
  every other kernel remains fail-closed.
- Profile JSON retains those values for migration and software processing but
  filters output routes and unsafe DSP controls from hardware apply.
- The WirePlumber rule keeps stable S16 playback open.

A real-card application audit attempted OutFX on/off, Crystalizer on, the
current output selection, and the current speaker layout. All five commands
failed before writing; the complete ALSA mixer SHA-256 remained
`a5fea602e5dd3cc3e1d0bf3a1492e4644171e9bf81d1a5035bd71ca29e60bc2d`.

## Remaining acceptance

1. Run the bounded connected-headphone bare-metal suspend/resume campaign.
2. Capture the reconnected AE-5 analog output safely.
3. Complete a runtime Windows/Linux same-settings capture. Static disassembly
   already establishes that Windows Command OutFX controls the
   `CtxRFX64.dll` software APO chain rather than Linux's hardware OutFX path.
