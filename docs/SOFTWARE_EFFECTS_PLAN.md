# Software effects: matching the Windows architecture

Direction for the next phase. Rests on the finding in
[`WINDOWS_STACK_ARCHITECTURE.md`](WINDOWS_STACK_ARCHITECTURE.md): the vendor
does not run SBX effects on the CA0132 DSP under Windows. It runs them on the
CPU, as a `CtxRFX64.dll` Audio Processing Object in the Windows audio engine.

We have been doing the opposite — programming the card's DSP over `dspio` —
and paying for it with an idle self-oscillation that reproduces in four of
five trials and that no Windows user reports.

The proposal is to stop fighting that path and take the one the vendor took:
compute the effects in software, in a PipeWire filter chain, and leave the
hardware DSP bypassed.

## Why this is the right move, not just an alternative

**It is a fix, not a workaround.** The oscillation is gated by
`Enable OutFX`. With effects computed upstream and OutFX off, the fault
cannot occur — there is no active DSP effect chain to destabilise. The card
becomes what it is genuinely excellent at: a clean DAC and headphone amp.

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

**It closes the safety gap.** The loud fault survived a desktop mute because
`soft-mixer` mutes samples while the DSP was generating signal after them.
With no DSP generation in the path, a software mute silences everything —
which is the guarantee the user actually needs from this application.

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
off. This is the beachhead: exactly representable, provably correct, the
most-used feature, and it works on a stock kernel. Acceptance is a measured
response curve matching the requested curve within a stated tolerance, taken
through the existing What U Hear tap and `acoustic-review.sh`.

**Phase B — substitutes.** Bass, presence and dynamics as honest equivalents,
each measured and each labelled as a substitute in `feature-parity.tsv`.

**Phase C — the switch.** Software path default; hardware DSP path available
for anyone who wants it, with the oscillation risk stated. Both readable in
the signal-path spine, which already has a stage for processing.

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
- Whether the DSP must be bypassed via `Enable OutFX` alone, or whether any
  residual hardware processing remains that colours the output.
- Whether S32 becomes viable again once nothing is generating signal after
  the mute point. Do not assume it does.
