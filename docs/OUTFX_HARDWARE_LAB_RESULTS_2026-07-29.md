# AE-5 hardware OutFX lab result — 2026-07-29

## Outcome

The cold-boot internal-capture matrix and the later user-present audible sweep
did **not** reproduce the persistent AE-5 DSP corruption. Every tested write
and playback lifecycle returned to bit-exact digital silence. The GUI verified
every requested mixer value. Apart from the lab kernel's expected
`AE-5 unsafe hardware OutFX lab enable accepted` warning, no new CA0132, HDA,
DSP, timeout, warning, or error message appeared at a post-transition
checkpoint.

The audible sweep covered each OutFX child alone, all children together,
global bypass and restore, the in-place PipeWire equalizer, safe headphone-gain
changes, and all three sound-filter choices. The user asked to interrupt the
sweep if corruption occurred; no interruption or fault report occurred.

This is a bounded stability result, not approval to enable hardware OutFX in
the production kernel or application. Sound Blaster Command's Windows OutFX
master controls a software APO chain; this lab exercises Linux's different,
raw CA0132 hardware effects path. It establishes neither Windows sound parity
nor analog-output safety.

## Test boundary

- Physical card: Creative Sound BlasterX AE-5, PCI `1102:0012`, subsystem
  `1102:0051`
- Cold-boot ID: `78082050-1d9d-4668-b61b-5cd34fc4ee8d`
- Kernel: `7.1.4-ae5-outfx-lab`, taint `0`
- Source commit at the audible follow-up:
  `69c0786dc3d161d5f1a4ddf36a804edaa59e8684`
- Lab kernel gate: `ae5_unsafe_outfx_lab=Y`
- GUI: release `outfx-lab` build, native Wayland, structured tracing enabled
- PipeWire AE-5 sink: `5%` for the internal matrix and exactly `20%` for the
  user-present sweep, never above the project's `20%` ceiling
- Internal matrix: AE-5 headphone and analog line-out jacks disconnected
- Audible follow-up: 32-ohm Philips SHP9500 on the AE-5 headphone output;
  analog line-out remained disconnected
- Direct Mode and output-route writes: blocked
- High headphone gain was excluded because it is not safe for the connected
  32-ohm headphones
- Detection: one-second CA0132 `What U Hear` samples at roughly three-second
  intervals, with hardware Master on so the internal tap remained observable
- Recovery: scoped PCI rebind helper ready; no recovery action was needed

## Completed matrix

### Cold baseline and global control

- More than five minutes at exact `-inf dBFS` before the first OutFX write.
- Enabling only the kernel gate did not change hardware OutFX.
- Global OutFX off → on and later on → off → on transitions remained silent.
- Global OutFX was also changed in both directions during active playback.

### Individual effects

Each child was enabled alone, changed, exercised across stream start/stop, and
disabled before continuing:

- Surround, including `87 → 80`
- Crystalizer, including `28 → 30`
- X-Bass, including `8 → 10 → 15` and crossover `27 → 35`
- Smart Volume, including `30 → 35 → 40` and
  `Normal → Loud → Night`
- Dialog Plus, including `0 → 10`

The live `Loud → Night` enum test was confirmed by both the GUI trace and ALSA
readback. Loud and Night intentionally ignore the adjustable Smart Volume
level in the current CA0132 contract.

### Equalizer and combined state

- Hardware Equalizer on with a custom ten-band curve.
- Factory preset `Flat → Rock → Flat` at digital silence.
- Factory preset `Flat → Rock` four seconds into an active stream.
- All five child effects plus hardware Equalizer enabled together.
- Five GUI effect switches disabled within approximately 3 ms at silence.
- The same five switches enabled within approximately 4 ms during playback.

Every grouped write was present in the GUI trace and exact in ALSA readback.
The app coalesced the resulting event burst into one refresh.

### Stream and format transitions

- Complete close/reopen with the normal delay and with a 50 ms gap
- Abrupt client termination followed by 96 kHz playback
- 44.1 kHz/S16 → 48 kHz/S32
- One client's 44.1/S16 → 48/S32 → 96/S32 → 48/S16 → 44.1/S16
  negotiation sequence
- Overlapping 48 kHz and 96 kHz S32 clients
- Five sustained reconnect, abrupt-stop, and overlap trials
- Track-like start/stop transitions with all effects active

### Playback PCM lifecycle

The final high-risk check explicitly suspended the PipeWire AE-5 sink. ALSA
playback changed from `RUNNING` to `closed`. The first subsequent fixture
reopened the PCM, completed normally, and the managed keepalive returned it to
`RUNNING`. The internal tap became finite only while the fixture was present
and then returned to exact `-inf dBFS`.

This directly exercises the close/reopen edge associated with the earlier
track-switch failure. The lab kernel includes the AE-5-only converter and
runtime-PM lifetime fix, so this pass supports that fix; it does not prove an
unpatched kernel safe.

## User-present audible follow-up

The familiar source was `Best of s0cliché 🔥` by CHAMPLOO, played from YouTube
in Brave. The physical PipeWire sink remained the default sink, at exactly
`20%`. Every hardware or graph transition was made while muted. Each audible
phase had a bounded playback window followed by mute, pause, ALSA readback,
fresh idle-monitor samples, kernel-log inspection, and a taint check.

### OutFX sequence

The short comparison sequence was:

1. neutral reference;
2. Surround `80`;
3. Crystalizer `30`;
4. X-Bass `15`;
5. Smart Volume `40`;
6. Dialog Plus `10`;
7. all five children enabled together;
8. global OutFX bypass with the five child switches still selected;
9. global OutFX restored with all five children;
10. every child disabled and global OutFX left enabled.

Every phase returned to exact `-inf dBFS` after mute. No recovery action was
needed. The final ALSA readback showed global OutFX on and all five child
switches off.

### Software EQ, gain, and filter sequence

- The saved ten-band PipeWire EQ was activated in place with its calculated
  `-10.80 dB` preamp. The existing AE-5 node retained the same default-sink
  identity, `20%` volume, and mute state. Playback stopped cleanly.
- The EQ was disabled through the GUI, then the exact original managed state
  was restored. Its SHA-256 is
  `2dbfa6b3b18118a2ee82b7deb7af9dad48f69b32e867f853ccbc83ab248c7549`;
  it is saved but not applied.
- Headphone gain was compared as Medium → Low → Medium. Low and Medium used
  the exact same source position (`02:00`) and bounded playback duration. High
  was deliberately not selected.
- Sound Filter was compared as Slow Roll Off → Minimum Phase → Fast Roll Off
  → Slow Roll Off. The exact `02:00` source position was reused.
- Hardware Equalizer remained off and Flat because it shares the unsafe raw
  OutFX path. Direct Mode remained unavailable. No output-route change was
  made.

All post-mute checks returned to exact silence. Kernel taint remained `0`.

## Monitor evidence

Machine-local evidence is intentionally not committed:

```text
~/.local/state/ae5-outfx-lab/
  78082050-1d9d-4668-b61b-5cd34fc4ee8d/
    monitor.csv
    gui-trace.log
    dual-tone-minus20.wav
    transition-fixtures/
```

At the audible-follow-up checkpoint, `monitor.csv` contained 2,559 samples
from `08:24:05` through `10:33:42` local time:

- 2,508 samples labeled `clean`
- 51 threshold alarms
- 199 finite-RMS samples in total
- 68 samples while the playback PCM was deliberately closed

All 51 alarms coincided with intentional fixture or music playback. The
monitor is an idle-oscillation detector, not a music classifier, so ordinary
test audio can cross its threshold. Smart Volume at `40` produced one such
music false-positive during the audible sweep. Immediate mute returned the tap
to exact silence for consecutive samples, with no kernel fault or recovery
action.

Some full-effect Smart Volume and format combinations reached substantially
higher internal levels than the source-only expectation. They always stopped
cleanly, but their gain behavior needs a separate measurement before claiming
level parity.

## Observed application and harness issues

- After the GUI wrote global OutFX off, controls on the Equalizer page retained
  the old cross-page sensitivity state. A later external mixer event rebuilt
  the page and enabled the valid software-EQ action. ALSA state was correct
  throughout. This is a GUI refresh defect, not DSP corruption.
- One AT-SPI node became invalid while the GUI rebuilt after a grouped effect
  change. The automation retried only after confirming the sink was muted; no
  hardware state or audio fault resulted.
- MPRIS absolute seek was ignored by Brave. The gain and filter comparisons
  therefore used a scoped virtual keyboard and verified an exact
  `120000000 µs` position before each phase.

## Verified handoff state

The live lab was left in this verified state:

- GUI open on Wayland
- monitor running
- kernel gate `Y`
- global OutFX on
- Surround, Crystalizer, X-Bass, Smart Volume, and Dialog Plus off
- hardware Equalizer off with preset Flat
- saved PipeWire software EQ present but not applied
- headphone gain Medium (`32–149 ohms`)
- sound filter Slow Roll Off
- AE-5 sink at exactly `20%` and muted
- YouTube source paused
- idle monitor samples at exact `-inf dBFS`
- SHP9500 still connected to the AE-5 headphone output; analog line-out
  disconnected

Do not treat this transient state as a persistent configuration. Before
leaving the lab kernel, follow the shutdown procedure in
[`OUTFX_HARDWARE_LAB.md`](OUTFX_HARDWARE_LAB.md): turn OutFX off, return the
gate to `N`, preserve the logs, and boot the qualified stable kernel.

## Remaining qualification

- Record the user's subjective assessment of which individual effects and
  filter changes were clearly audible; silence during the sweep confirms only
  that no fault was reported.
- Fix the stale cross-page GUI capability refresh before presenting software
  EQ activation as reliable.
- Measure Windows and Linux output level and frequency response with a
  controlled capture path. The completed sweep establishes transition
  stability, not Windows APO equivalence or loudness parity.
- Keep raw hardware OutFX behind the lab-only kernel gate until its gain
  behavior and a repeat cold-start cycle have been reviewed.
