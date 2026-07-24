# AE-5 headphone tuning investigation

This document separates verified CA0132 behavior from hypotheses about
Sound Blaster Command's named headphone tuning. It defines the evidence and
tests required before changing the kernel driver or loading another DSP image.

The investigation was last verified on 2026-07-24.

## Result so far

Do not load the packaged `ctspeq.bin` on the AE-5.

The file is not an AE-5 headphone-profile database. Its
[upstream ALSA firmware commit](https://github.com/alsa-project/alsa-firmware/commit/cbb9d36a7cdb36697e0db2f8455465bdaa3008c2)
identifies it as a single SpeakerEQ coefficient preset tuned for Chromebook
Pixel hardware. Other CA0132 systems were explicitly expected to run without
it.

The active Windows settings migration proves only that Sound Blaster Command
can select a named headphone tuning that has no current ALSA equivalent. It
does not prove that this selection uses `ctspeq.bin`, that it uses the same
binary format, or that the packaged Chromebook coefficients are safe for an
AE-5.

The independent fast-load parser hardening from step 3 is now implemented as
an unsubmitted candidate patch. It validates complete images before DSP reset
and has build and KUnit coverage, but it has not been loaded into the running
kernel. Parser safety does not establish the meaning or suitability of any
headphone coefficient image.

## Verified public-source behavior

The exact CA0132 source used by the running kernel is byte-identical to Linux
stable `v7.1.4`. The relevant logic is unchanged in the recorded upstream
`master` snapshot.

The driver defines:

- master-control module `0x80`;
- request `60` to query a SpeakerEQ relocation address;
- output-effects module `0x96`;
- request `0x1f` to enable or disable the loaded SpeakerEQ coefficients.

The generic SCP helper can issue a GET and validate a one-word reply. The
driver uses that mechanism to allocate DSP DMA channels, but it never sends the
SpeakerEQ address query. It requests only `ctefx.bin`, `ctefx-desktop.bin`, or
`ctefx-r3di.bin`; `ctspeq.bin` is absent from both `MODULE_FIRMWARE` and every
`request_firmware()` call.

Every alternative-card output selection, including the AE-5 path, writes
floating-point zero to request `0x1f`. Therefore changing between speakers and
headphones always disables SpeakerEQ use.

The comments connecting the address query to Windows headphone profiles were
added by
[Linux commit `896e361e8242`](https://github.com/torvalds/linux/commit/896e361e82423aed4490f485dc25de1958c724ed).
That commit labels the relationship to `ctspeq.bin` as a belief and the
uploaded data as presumed EQ data. The older, first-party ALSA firmware commit
gives the binary a narrower Chromebook SpeakerEQ purpose. The first-party
provenance wins over the later conjecture.

## Why a headphone loader patch is not ready

The existing fast-load parser takes a pointer to a `dsp_image_seg`, not the
firmware byte length. Segment iteration trusts each embedded word count and
advances until it encounters a zero-count terminator. Its magic and target
address checks are useful, but they do not prevent an out-of-bounds read from a
truncated or malformed image.

The current firmware is trusted and packaged, so this is a hardening gap rather
than proof of an exploitable user-controlled path. The upstream source still
has the gap. The project now carries
[`ca0132-dsp-image-bounds.patch`](../kernel/ca0132-dsp-image-bounds.patch),
which passes the firmware size into `dspload_image()` and preflights the whole
image before resetting the DSP. A new optional overlay must not proceed unless
that hardening, or an upstream equivalent, is in the kernel being tested.

The driver also has no evidence that a named AE-5 headphone preset has the same
layout, relocation behavior, or coefficient count as the Chromebook image.
Blind loading risks a DSP timeout, silent output, or a device state that
requires a power cycle.

## Safe implementation sequence

### 1. Establish the audible gap

Use the existing audio-parity harness to capture the same 48 kHz/24-bit
reference through:

1. Windows with all enhancements disabled;
2. Linux with all enhancements disabled;
3. Windows with only the named headphone tuning enabled;
4. Linux with the imported graphic EQ and otherwise identical controls.

Record output, gain range, DAC filter, sample rate, channel mode, mixer values,
and analog capture chain. Compare level, frequency response, noise floor, and
channel balance. Do not infer a DSP defect from a volume mismatch.

Stop if the Windows/Linux neutral captures match within the measurement
tolerance. In that case the remaining difference is a preset translation
problem and belongs in the Rust importer, not the kernel.

### 2. Add read-only kernel instrumentation — candidate implemented

The repository now carries
[`ca0132-speaker-eq-address-probe.patch`](../kernel/ca0132-speaker-eq-address-probe.patch).
It:

- sends one `MASTERCONTROL_QUERY_SPEAKER_EQ_ADDRESS` GET after the DSP reaches
  `DSP_DOWNLOADED`;
- accepts exactly one 32-bit response;
- logs success, error, and address through `codec_dbg()`;
- exposes no writable ALSA control and performs no DSP upload;
- is gated to AE-series quirks, with output available through dynamic debug.

The unmodified and patched Linux stable `v7.1.4` source both compile as
external modules against the running Nobara `7.1.4` kernel-devel tree with
`W=1` and warnings treated as errors. No module was installed or loaded.
Loading an alternate module changes the live audio driver and therefore still
requires an explicit test session with a known-good kernel available.

Acceptance criteria:

- probe, playback, 50 speaker/headphone switches, suspend/resume, and module
  removal complete without a timeout or kernel warning;
- the query result is stable across three cold boots;
- a control run without the query has an identical neutral frequency response;
- failures leave normal playback available.

### 3. Bound the existing fast-load parser — candidate implemented

The independent candidate passes the firmware length into `dspload_image()` and
validates every segment before `dsp_reset()`. It rejects:

- a header that does not fit;
- `count * sizeof(u32)` overflow;
- a segment extending beyond the firmware;
- an HCI address/data list with an odd word count;
- a missing in-bounds terminator;
- invalid terminator magic, an empty image, and nonzero trailing data;
- any relocation or word range outside supported DSP memory.

The KUnit suite covers empty, truncated, oversized, overflow, missing
terminator, odd and consecutive HCI lists, invalid magic, invalid ranges,
relocation overflow, zero padding, and valid multi-segment images. It passes
under x86-64 QEMU. A metadata-only compatibility check accepts all three
currently requested CA0132 firmware images, plus the outer structure of
`ctspeq.bin`.

This hardening is useful even if no SpeakerEQ loader is ever added.

The remaining acceptance gate is a controlled alternate-kernel test proving
that normal CA0132 firmware loading, playback, routing, suspend/resume, and
recovery remain intact on the physical AE-5. That changes the live driver and
requires an explicit test session.

### 4. Identify an AE-5 coefficient source lawfully

Acceptable evidence is:

- Creative documentation or interoperability information;
- a redistributable, device-specific firmware/preset with explicit provenance;
- one-setting-at-a-time hardware observations that produce an independent
  behavior specification;
- measured target frequency response that can be approximated with the
  already-exposed ten-band hardware EQ.

Do not decompile Creative software, disassemble firmware, parse proprietary
binary preset files, or copy a Windows driver's implementation. Do not add a
raw DSP-upload userspace API.

If only the target response is available, implement an approximate named
preset in the Rust migration layer and label it approximate. That is safer,
testable, and reversible.

### 5. Consider a kernel overlay only with matching evidence

A future fixed-function kernel path would have to:

- request a specifically named, redistributable image;
- validate its complete structure and expected size before touching hardware;
- query the relocation address;
- mute the DSP, upload through the existing overlay DMA path, and verify
  completion;
- enable request `0x1f` only after a successful upload;
- disable it before output changes, suspend, shutdown, and recovery;
- unwind every DMA, stream, port, firmware, and mute state on error.

It must not load the Chromebook preset merely because the file is installed.

## Virtualization limits

Docker and WSL cannot validate this kernel path because they do not own the
physical HDA codec and PCIe bridge.

A Linux host can assign the AE-5 to a Windows KVM/QEMU guest with VFIO if the
card and its bridge are isolated in a safe IOMMU group. The host loses the card
while it is assigned. VFIO also lets the guest access the device directly, so
ordinary QEMU HDA logging does not observe the passed-through device's private
transactions. Passthrough is useful for controlled Windows audio measurements,
not as proof of the kernel command sequence.

Final validation must use the physical AE-5 on both operating systems.

## Current stop condition

No live driver experiment or SpeakerEQ firmware upload is justified yet. The
bounded-parser candidate is complete at build/KUnit level, and the read-only
address-query candidate is complete at build-only level. After the pending
cold-boot routing capture, the next safe driver action is an explicitly
authorized alternate-kernel or module session for the address query. The next
safe application action is objective Windows/Linux response measurement
followed by an approximate Rust-side preset only if the measurements support
one.
