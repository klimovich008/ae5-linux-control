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
kernel. Effects and EQ in software work on a stock distribution kernel, which
is what this host is now running. Direct Mode and the onboard LEDs still need
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

## Validated before writing this

PipeWire 1.6.8 on this host, `libpipewire-module-filter-chain` present. A
two-band `bq_peaking` chain declared in a
`~/.config/pipewire/pipewire.conf.d/` drop-in loads and appears as a real
sink and stream pair. The probe was removed and the card returned to matched
routes at 20%. The mechanism works; what remains is engineering.

## Open questions for measurement, not opinion

- Added latency of a ten-biquad chain at 48 kHz, measured, against the
  current period geometry (6016 × 4).
- CPU cost under the same geometry. Expected negligible; state the number.
- Stability and response of the normal CA0132 route with every effect module
  disabled and the playback PCM held open.
- The kernel cause of the normal-route PCM-reopen corruption. Direct Mode must
  remain unavailable until this is resolved.
- Whether S32 becomes viable again once nothing is generating signal after
  the mute point. Do not assume it does.

## Phase A implementation status — 2026-07-27

The configuration and control plane are implemented. This is not yet the
physical response acceptance:

- `src/eq_chain.rs` converts all ten profile EQ values through the live ALSA
  dB mapping and emits twenty `bq_peaking` nodes: ten for left and ten for
  right.
- The filter playback stream uses `target.object` for the exact current AE-5
  PipeWire sink plus `node.dont-fallback=true`. A missing or renamed target
  therefore fails closed instead of sending processed audio to another
  device.
- Enabling refuses to create a second processing path unless live
  `Enable OutFX` is readable and off.
- The generated virtual sink carries a deterministic signature containing the
  physical target and ten gains. Activation verifies that this live signature
  equals the managed file, so an old graph cannot be selected after an EQ
  update without a PipeWire restart.
- `ae5ctl eq-chain-enable FILE` only writes or updates the managed user
  fragment. `eq-chain-activate` separately verifies the target, signature, and
  OutFX state before changing the desktop default.
- `eq-chain-disable` restores the physical AE-5 first when the software sink
  is the default, then removes only the managed fragment.
- The GTK Equalizer page presents the same install, restart, activate, and
  disable workflow. Disabled actions remain non-interactive and explain the
  blocking state. The hardware EQ pill says `ARMED` when its child switch is
  saved but OutFX is off.

Validation completed without opening an audio stream:

1. The real `EQ · SHP9500 test` profile generated `+9, +6, +10, 0, +1, -2,
   0, -3, 0, +1 dB` against the live card and targeted
   `alsa_output.pci-0000_29_00.0.analog-stereo`.
2. `pw-config` parsed the complete generated fragment.
3. A separate temporary PipeWire daemon loaded the twenty-node graph and
   exposed the exact target/gain signature on `ae5_software_equalizer`. It had
   no WirePlumber session manager, no hardware node, and no playback stream.
4. The real per-user configuration, desktop default, ALSA mixer, and PipeWire
   services were left unchanged.

Still required before Phase A can be called accepted:

- install one selected profile in the real per-user configuration, restart
  PipeWire, and confirm that the live sink signature matches;
- make the virtual sink default through the guarded action and verify desktop
  routing;
- measure the requested versus captured response curve and state the
  tolerance;
- measure latency and CPU cost;
- run the stated long-duration stability gate with every CA0132 effect module
  disabled;
- repeat the safe disable/restart path and prove that the physical AE-5
  default is restored.
