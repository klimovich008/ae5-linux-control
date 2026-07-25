# AE-5 Direct Mode investigation

## Status

An AE-5-only kernel and application candidate is implemented, statically
validated, and ready for an isolated physical-card test. It is not yet a
verified feature. The running stock kernel was not modified and no live mixer
or PipeWire state was changed while developing the candidate.

The candidate deliberately does not advertise 192 or 384 kHz. Its first
hardware gate is reliable stereo playback at rates already exposed by the
Linux analog PCM, followed by an exact return to the normal DSP path.

## Evidence boundary

The implementation is an independent behavior-level reconstruction. No
Creative binary, decompiler output, or copied proprietary control flow is
stored in this repository.

The locally installed, user-owned Windows driver used for interoperability
analysis was:

| Package | Version | SHA-256 |
|---|---|---|
| Active `CtxHda.sys` | `6.0.105.0065` | `4be35390a2de694041cd20317ed5a148d4852e46f201945a346a8b2a2c79dccf` |
| DriverStore `CtxHda.sys` | `6.0.105.0064` | `3e250aa313f15d960d9717ca93a37783ccada108d02c5f8cb6de9a453367b79c` |
| DriverStore `CtxHda.sys` | `6.0.105.0055` | `9273eb1c873224cc99de7fd8398924c4e8e86fa0a9f81639a0970dd2c730f201` |

Analysis was performed offline with official Ghidra `12.1.2`; its downloaded
archive had SHA-256
`b62e81a0390618466c019c60d8c2f796ced2509c4c1aea4a37644a77272cf99d`.
The local reports remain outside the Git repository. Comparing `.0055`,
`.0064`, and `.0065` separated stable device behavior from a later
Windows-transition fix without importing implementation code.

The Linux side is based on `sound.git` `for-next` commit
`61471f29f3157f33a61194bf82b4a289cc03e1f1`. Its existing
`ae5_post_dsp_stream_setup()` already contains the normal route reconstructed
below.

## Independent behavior specification

Direct Mode is a route transition, not merely an effects switch:

1. Windows identifies the exact AE-5 backend through subsystem
   `1102:0051`.
2. Enabling the direct family stops CA0132 ChipIO stream `0x18`, bypassing the
   DSP playback path.
3. Returning to normal mode reconstructs:
   - stream `0x05`, source `0x43`, destination `0x00`;
   - stream `0x18`, source `0x09`, destination `0xd0`;
   - connection point `0xd0` at 96 kHz;
   - stream `0x18` with six channels and enabled;
   - ASI control parameter `23` with value `7`.
4. The newer Windows path quiesces the endpoint around this transition. That
   sequencing is relevant because the preceding driver release documented
   distortion or loss of sound after some Windows 11 Direct Mode toggles.
5. Direct formats are stereo. The static Windows policy recognizes higher
   rates, but that alone is not enough evidence to expand Linux PCM
   capabilities.
6. DSP volume and effects are bypassed. Stream or software volume is therefore
   the effective volume control.

The stream source/destination, `0xd0` rate, channel-count, and stream-enable
parts of item 3 independently match the existing Linux AE-5 startup sequence.
The final ASI value does not: current Linux startup writes `4`, while the
Windows Direct-to-normal transition writes `7`. The candidate follows the
transition-specific value, which remains a physical acceptance gate.

## Candidate implementation

[`kernel/ca0132-ae5-direct-mode.patch`](../kernel/ca0132-ae5-direct-mode.patch)
adds the standard ALSA boolean control
`AE-5: Direct Mode Playback Switch`. ALSA simple-mixer clients, including this
application, see it as `AE-5: Direct Mode`.

The kernel side:

- exposes the control only for `QUIRK_AE5`, not the AE-7 or other CA0132
  devices;
- serializes the transition against analog PCM open and close;
- returns `-EBUSY` instead of rerouting an open playback stream;
- constrains the next Direct Mode PCM open to two channels;
- verifies stream `0x18` state after each transition;
- reports zero CA0132 DSP latency while Direct Mode is active;
- restores the requested direct state after codec reinitialization;
- leaves the existing analog PCM rate and sample-width capabilities unchanged.

The Rust backend detects the control at runtime, so stock kernels continue to
work without an empty or misleading UI. Before changing Direct Mode it:

1. locates only the PipeWire sink belonging to the discovered AE-5 card;
2. returns immediately when the requested state already matches the control;
3. preserves an already-suspended sink;
4. otherwise suspends that sink through `pactl`;
5. waits up to one second for `/proc/asound/cardN/pcm0p/sub0/status` to report
   `closed`;
6. writes and reads back the ALSA control;
7. resumes the sink, including a best-effort retry during error unwinding.

The GTK Playback page explains the stereo DSP bypass and disables choices,
levels, and playback-effect enables that are ineffective in Direct Mode.
Recording and CrystalVoice controls remain available because the reconstructed
stream `0x18` behavior does not prove that capture processing is bypassed.
Output selection, headphone gain, DAC filter, and the Direct Mode switch remain
available. Native profiles capture the control automatically when the patched
kernel exposes it.

## Static validation

On 2026-07-25:

- the kernel patch passed `git diff --check`;
- strict `scripts/checkpatch.pl` reported zero errors, warnings, or checks
  across 218 changed lines;
- the patch applied after all five production CA0132 patches and to the
  maintained 6.18.40 backport trees;
- the combined `ca0132.o` and DSP-image parser-test object compiled with
  `W=1` and warnings treated as errors;
- `sound/hda/codecs/ca0132.o` compiled successfully in a fresh out-of-tree
  x86-64 build using the running Nobara configuration;
- `cargo test --all-features` passed 47 library and 12 GTK tests;
- `cargo clippy --all-targets --all-features -- -D warnings` passed;
- an isolated Fedora 44 RPM build passed its release tests and metadata checks,
  and the package requires the `pulseaudio-utils` provider of `pactl`;
- the live host mixer baseline remained
  `3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`.

A standalone `.ko` modpost was not claimed: the lightweight object build has
no `vmlinux` symbol table. The C compilation itself completed without an
error.

## Physical acceptance sequence

Use the managed VFIO guest first, with the default host kernel retained as the
recovery path.

1. Apply the production patch series and this Direct Mode patch to the pinned
   kernel. Do not include the diagnostic SpeakerEQ probe.
2. Build the complete kernel and modules with warnings treated as errors.
3. Boot the guest with the physical AE-5 passed through and save the complete
   guest mixer, PipeWire state, and kernel log baseline.
4. Confirm exactly one new simple control named `AE-5: Direct Mode`, initially
   off.
5. Hold analog playback open and prove that a raw Direct Mode write returns
   `EBUSY` without changing the control or producing a driver warning.
6. Stop the direct ALSA client. Toggle through the application and prove that
   its scoped PipeWire suspension permits the write and that the sink resumes.
7. In Direct Mode, verify stereo playback through the physical headphone
   output with the external microphone fixture. Test 44.1, 48, and 96 kHz
   separately; test 88.2 kHz only after those pass.
8. Prove a six-channel open is rejected in Direct Mode while stereo opens
   successfully.
9. Compare an effects-on/off pair. Direct Mode must remain unchanged within
   the measurement tolerance; normal mode must restore the previously proven
   effect response.
10. Toggle Direct Mode off and prove normal playback, DSP effects, output
    selection, and What U Hear return without a manual ALSA toggle.
11. Repeat across headphone and line-out, 20 safe mode transitions, suspend
    and resume, three warm boots, and at least one cold boot.
12. Restore the saved guest state and require an exact mixer hash, no CA0132,
    HDA, DSP, timeout, lockdep, or kernel warning, and automatic host recovery
    after VFIO shutdown.

Only after that matrix passes should 192 or 384 kHz support be investigated.
Those rates require converter-capability and physical-output evidence; static
Windows acceptance alone is not a Linux support claim.
