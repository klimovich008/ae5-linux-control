# Software effects: matching the Windows architecture

Direction for the next phase. Rests on the finding in
[`WINDOWS_STACK_ARCHITECTURE.md`](WINDOWS_STACK_ARCHITECTURE.md): the vendor
runs a real software implementation of SBX in the `CtxRFX64.dll` Audio
Processing Object. Static analysis confirms the software module chain and
OutFX master semantics; it does not prove that every hardware module remains
idle under Windows.

We have been doing the opposite — programming the card's DSP over `dspio` —
and paying for it with an idle self-oscillation that reproduces in four of
five trials and that no Windows user reports.

The proposal is to stop fighting that path and take the one the vendor took:
compute the effects in software, in a PipeWire filter chain, and leave the
individual CA0132 effect modules disabled. This is not the same as bypassing
the complete DSP/router path. True bypass is the project's separate Direct
Mode and carries its stereo/rate/volume constraints.

## Why this is the right move, not just an alternative

**It removes the known trigger.** The measured idle oscillation is gated by
`Enable OutFX`. With effects computed upstream and OutFX off, the individual
hardware effect modules stay disabled. The normal CA0132 router/mixer remains
in the path, so stability still needs a long-running acceptance test and must
not be stated as guaranteed.

**The equalizer becomes exact rather than approximate.** A ten-band graphic
EQ is ten peaking biquads. That is not an approximation of what the hardware
does — it is the same mathematics, with coefficients we choose and can read
back. The imported Windows EQ curves would be applied exactly, where today
they pass through DSP scaling we never fully characterised.

**It makes the project kernel optional.** Today a user needs the patched
kernel. Effects and EQ in software work on a stock distribution kernel.
Direct Mode and the onboard LEDs still need
the patch queue, but the core value would install for anyone with an AE-5.
That is the difference between a one-host MVP and something usable.

**It strengthens the honesty guarantee rather than weakening it.** Verified
readback is this project's identity, and a software chain is fully
inspectable: the coefficients we set are the coefficients running. No stale
DSP, no readback that lies, no state we cannot clear without a PCI rebind.

**It narrows the safety gap.** The loud fault survived a desktop mute because
`soft-mixer` mutes samples before the card's DSP. Disabling the hardware effect
modules removes one known post-mute signal source. Only measured Direct Mode
can establish that the complete normal DSP path is absent, so the physical
hard-mute recovery procedure remains mandatory for the normal route.

## What we cannot honestly claim

Crystalizer, Smart Volume, Surround and Dialog+ are proprietary algorithms.
We can build equivalents — transient enhancement, dynamic range control,
stereo widening, presence lift — but they will not be bit-identical, and must
never be labelled as the vendor's effects. The ledger already has the right
word for this: **intentionally substituted**, which thirteen features already
carry. Anything we ship here is our processing, named as ours, measured on
its own terms.

The equalizer is the exception: it is exactly representable, and may be
claimed as such once measured.

## Phasing

**Phase A — the equalizer.** Ten `bq_peaking` biquads at the ISO centres the
existing ledger already uses, driven from the same profile JSON, with OutFX
off. This is the beachhead: exactly representable as our own filter, provably
correct against our requested curve, the most-used feature, and it works on a
stock kernel. It is not claimed to be coefficient-identical to Creative's EQ.
Acceptance is a measured response curve matching the requested curve within a
stated tolerance, taken through the existing What U Hear tap and
`acoustic-review.sh`.

**Phase B — substitutes.** Bass, presence and dynamics as honest equivalents,
each measured and each labelled as a substitute in `feature-parity.tsv`.

**Phase C — fail closed.** The software path is the only supported output
effects path. Hardware OutFX and its child output-effect controls are retained
in imported profile metadata but rejected before an ALSA write. The kernel
guard initializes the AE-5 output effects off, rejects an OutFX enable with
`-EOPNOTSUPP`, and treats redundant off replay as a no-op because even an off
write can disturb hidden DSP state. Direct Mode is also unavailable.

The exact-card WirePlumber path keeps the normal analog playback PCM open.
This is required independently of OutFX: waveform testing found that a
playback close/reopen can corrupt the normal route with every output effect
already off.

## PipeWire mechanism selected

PipeWire 1.6.8 exposes `audioconvert.filter-graph.N` on the existing AE-5
sink. The property is explicitly intended for runtime-swappable filter graphs.
Using it avoids the extra virtual sink, playback stream, desktop-default
transition, and second software-volume stage created by
`libpipewire-module-filter-chain`.

An isolated null-sink probe logged successful load and removal of a builtin
`linear` graph at order zero. The real AE-5 then accepted an identity graph
while suspended, 5%, muted, and physically unplugged. Both playback PCMs
remained closed and PipeWire/ALSA state matched after removal. No audio was
played by either probe.

## Open questions for measurement, not opinion

- Added latency of a ten-biquad chain at 48 kHz, measured, against the
  current period geometry (6016 × 4).
- CPU cost under the same geometry. Expected negligible; state the number.
- Stability and response of the normal CA0132 route with every effect module
  disabled and the playback PCM held open.
- Connected-output and suspend/resume qualification of the fixed normal-route
  lifecycle. Direct Mode remains unavailable until its bypass transition is
  reevaluated on the exact stable-playback base.
- Whether S32 becomes viable again once nothing is generating signal after
  the mute point. Do not assume it does.

## Phase A implementation status — 2026-07-27

The direct configuration and control plane are implemented, and the first
bare-metal 48 kHz response gate has passed:

- `src/eq_chain.rs` converts all ten profile EQ values through the live ALSA
  dB mapping and emits ten `bq_peaking` nodes per channel.
- The current direct-filter-v2 graph starts at the first `bq_peaking` node and
  inserts no linear preamp stage. Boosted curves can clip near full scale, so
  listening-level headroom is the user's responsibility. The retired v1
  format remains parseable only for exact rollback and one-way migration.
- The saved state pins the exact current AE-5 PipeWire node name. A missing or
  renamed target fails before any runtime graph change.
- Software EQ and OutFX are independent processing groups, matching the
  recovered Windows architecture, so they may remain active together. Direct
  Mode still blocks software EQ because it explicitly bypasses processing.
- Activation verifies that the sink exposes
  `audioconvert.filter-graph.N`, suspends that exact sink, loads the graph at
  order zero through `pw-cli set-param`, stores a per-node runtime signature,
  verifies the signature readback, and resumes the sink.
- The runtime signature contains the graph version, target, and ten gains. A
  PipeWire restart or node recreation drops the marker, so
  the UI correctly requires reapplication instead of claiming stale state.
- `ae5ctl eq-chain-enable FILE` only writes or updates the managed user
  state. `eq-chain-activate` applies it in place without changing the desktop
  default or restarting PipeWire. `eq-chain-disable` unloads order zero,
  clears and verifies the marker, then removes only the managed state file.
- There is now one user-visible volume/mute stage: the original physical AE-5
  sink. This fixes the former design in which copying 5% to a virtual sink
  while the physical sink also remained at 5% compounded PipeWire's cubic
  software attenuation and could make output effectively inaudible.
- The GTK Equalizer page chooses and applies a profile in one action, exposes
  saved/runtime state and the no-automatic-attenuation warning, and retains
  separate reapply and disable actions. The hardware EQ pill still says
  `ARMED` when its child
  switch is saved but OutFX is off.
- `ae5ctl eq-chain-response RATE` reports the exact requested filter response
  at all ten fixture frequencies. `audio-parity.sh compare-eq` compares those
  values with neutral/equalized What U Hear captures and fails when any
  measured band differs by more than 1 dB.

Validation completed before the first playback measurement:

The following v1 evidence is historical: its fixed attenuation has now been
removed and needs a fresh physical response capture.

1. The imported `Windows My profile — Headphone` curve generated `+9, +6,
   +8, +4, +1, -2, -2, 0, +6, +6 dB`, targeted
   `alsa_output.pci-0000_29_00.0.analog-stereo`, and calculated −10.80 dB
   automatic preamp.
2. `pw-config` parsed the complete graph object. Current unit tests verify
   that v2 contains no linear preamp nodes and that v1 migrates safely.
3. A separate temporary PipeWire daemon loaded and removed the direct
   audioconvert graph API against a null sink.
4. The real muted/unplugged AE-5 loaded the complete imported graph, exposed
   the exact runtime signature, and unloaded it. Both playback PCMs remained
   closed throughout.
5. The physical sink remained default, 5%, and muted. The full `wpctl` state,
   raw ALSA controls, and both PCM status files matched before and after.
   The managed state file and runtime marker were absent after cleanup.
6. The GUI-enabled Rust gate passed 125 tests, strict Clippy, release build,
   and formatting.

Bare-metal response acceptance after a true power-removal boot:

- `7.1.4-ae5-stable` ran untainted with the signed matching module, hardware
  OutFX off, all AE-5 analog outputs unplugged, and Master/Front hard-muted.
- Two neutral and two equalized What U Hear captures repeated within 0.00 dB
  at all ten fixture frequencies. The equalized captures came from separate
  `pw-play` clients while the same in-place graph remained active.
- The imported headphone curve requested `-0.30, -1.29, -0.54, -4.81, -9.09,
  -12.70, -12.87, -10.17, -4.44, -4.38 dB` after the automatic `-10.80 dB`
  preamp. Measured equalized-minus-neutral response was `-0.31, -1.32, -0.54,
  -4.80, -9.11, -12.55, -13.04, -10.51, -4.46, -4.52 dB`.
- Maximum absolute error was 0.34 dB, passing the 1 dB gate. Cleanup restored
  5% muted, Master/Front off, Low gain, OutFX off, both playback PCMs closed,
  and no managed EQ state or runtime marker.

The first Windows VM matrix proved that Creative `What U Hear` contains the
imported Acoustic Engine result, but its forced graphic-EQ captures did not
contain Command's displayed ten-band curve and differed from the Linux model
by up to 13.08 dB. That endpoint is therefore rejected for EQ parity; see
[`windows-capture/VM-OUTFX-RESULTS.md`](windows-capture/VM-OUTFX-RESULTS.md).

### Performance and soak gate

`scripts/check-software-eq-performance.sh` now measures the exact physical
sink before and after loading the managed graph. It refuses a connected-output
run without the explicit unplugged acknowledgement, requires Low gain and
OutFX off, rejects a sink above 20%, requires every playback application
closed, hard-mutes Master and Front, and watches those controls and the
software-volume ceiling throughout. A bounded −30 dBFS 997 Hz stream exercises
the graph at 5%; cleanup unloads the graph and verifies the complete mixer,
sink identity, route, volume/mute state, and PCM closure.

The accepted 7200-second physical-card qualification measured this initial
neutral/equalized comparison:

| Measurement | Neutral | Ten-band EQ | Delta |
|---|---:|---:|---:|
| PipeWire process CPU | 0.6000% | 0.9990% | +0.3990 percentage points |
| Mean sink busy time | 13.800 µs | 192.364 µs | +178.564 µs |
| Maximum sink busy time | 16.800 µs | 213.700 µs | — |
| PipeWire quantum | 2048 frames at 48 kHz | 2048 frames at 48 kHz | 0 frames |
| PipeWire errors | 0 | 0 | 0 |

The extra filter work consumed 0.4185% of the 42.667 ms quantum. The sink ID,
serial, format, rate, and quantum remained identical, so the in-place graph
added no PipeWire node or scheduling buffer. That is a measured
buffering/topology result, not a claim that an equalizer has zero
frequency-dependent phase delay.

During the following two-hour nonzero soak, 7197 timing samples averaged
200.430 µs busy with a 267.900 µs maximum, 1.1060% PipeWire process CPU, and
zero errors. There were no relevant kernel, PipeWire, or WirePlumber warnings.
Cleanup removed the runtime graph and state file, restored the byte-identical
mixer, 5% muted sink, exact Headphone/Microphone routes, and OutFX off, and
closed both playback PCMs. `result.txt` records `recovery=pass`,
`soak_seconds=7200`, and `qualification=long-duration`. The private evidence
is in
`~/.cache/ae5-control/eq-performance-20260728-021222.yNFWFZ`.

Still required before Phase A can be called generally accepted:

- repeat at 44.1 and 96 kHz and sample more embedded curves;
- complete the bounded connected-headphone suspend/resume gate;
- compare the normalized response through an independently verified post-EQ
  Windows endpoint or safely attenuated analog capture while OutFX remains
  off.
