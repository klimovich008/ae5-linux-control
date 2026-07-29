# AE-5 hardware OutFX lab result — 2026-07-29

## Outcome

The first cold-boot, internal-capture-only OutFX matrix did **not** reproduce
the persistent AE-5 DSP corruption. Every tested write and playback lifecycle
returned to bit-exact digital silence. The GUI verified every requested mixer
value. Apart from the lab kernel's expected one-time
`AE-5 unsafe hardware OutFX lab enable accepted` warning, no new CA0132, HDA,
DSP, timeout, warning, or error message appeared at a post-transition
checkpoint.

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
- Source commit: `0926b6051cff13bc0954d559bce203721047351f`
- Lab kernel gate: `ae5_unsafe_outfx_lab=Y`
- GUI: release `outfx-lab` build, native Wayland, structured tracing enabled
- PipeWire AE-5 sink: capped at `5%`, never above the project's `20%` ceiling
- AE-5 headphone and analog line-out jacks: physically disconnected
- Direct Mode and output-route writes: blocked
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

At the recorded handoff, `monitor.csv` contained 1,180 samples from
`08:24:05` through `09:23:56` local time:

- 1,133 samples labeled `clean`
- 47 threshold alarms
- 124 finite-RMS samples in total
- 10 samples while the playback PCM was deliberately closed

All 47 alarms coincided with intentional fixture playback. The monitor is an
idle-oscillation detector, not a music classifier, so ordinary test audio can
cross its threshold. The two automatically captured incident directories,
`20260729-085759` and `20260729-085930`, are retained as false-positive
examples: signal was active at their timestamps and later samples returned to
exact silence without a recovery action.

Some full-effect Smart Volume and format combinations reached substantially
higher internal levels than the source-only expectation. They always stopped
cleanly, but their gain behavior needs a separate measurement before claiming
level parity.

## Verified handoff state

The live lab was left ready for a deliberate follow-up:

- GUI open on Wayland
- monitor running
- kernel gate `Y`
- global OutFX on
- Surround, Crystalizer, X-Bass, Smart Volume, and Dialog Plus off
- hardware Equalizer off with preset Flat
- AE-5 sink at `5%` and muted
- idle monitor samples at exact `-inf dBFS`
- analog outputs still physically disconnected

Do not treat this transient state as a persistent configuration. Before
leaving the lab kernel, follow the shutdown procedure in
[`OUTFX_HARDWARE_LAB.md`](OUTFX_HARDWARE_LAB.md): turn OutFX off, return the
gate to `N`, preserve the logs, and boot the qualified stable kernel.

## Remaining acceptance step

The next step is one low-volume audible A/B with the user present:

1. keep every child effect and Equalizer off;
2. connect one output only, with headphones off the user's head;
3. verify `5%` and muted again after the physical connection;
4. start a familiar, bounded source and unmute;
5. enable one child effect, make one small change, then disable it;
6. stop playback and wait for a fresh silent monitor sample;
7. stop immediately on buzz, distortion, failed write, or a non-silent idle
   sample.

An audible pass can assess analog behavior and subjective effect changes. It
still cannot make Linux hardware OutFX equivalent to the Windows APO.
