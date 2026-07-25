# AE-5 Direct Mode investigation

## Status

An AE-5-only kernel and application candidate is implemented, statically
validated, and running against the physical card in a managed VFIO guest.
Stereo playback, DSP bypass, output selection, busy rejection, exact
Direct-to-normal restoration, and repeated transitions have passed. The
candidate remains deferred rather than release-supported until its
power-management, repeated-boot, and physical line-out gates pass.

Direct Mode deliberately exposes only stereo 48 and 96 kHz. Exact ALSA
negotiation rejects 44.1, 88.2, and 192 kHz and six-channel opens. Earlier
diagnostic candidates produced silence at 44.1 kHz and distortion at 88.2 kHz,
so those rates must not be advertised without a new hardware explanation and
acceptance run. The analog PCM does not expose 384 kHz.

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
- constrains the next Direct Mode PCM open to stereo 48 or 96 kHz;
- prepares stream `0x14`, the HDA converter, clock, rate, and ASI state for
  each Direct PCM;
- snapshots only the router entries overwritten after stream `0x05` changes
  to the direct source, then restores those exact entries transactionally;
- verifies stream `0x14` and `0x18` state around their respective transitions;
- reapplies the cached playback-processing edge and rebinds the HDA converter
  on the first normal PCM prepare after Direct Mode;
- reports zero CA0132 DSP latency while Direct Mode is active;
- restores the requested direct state after codec reinitialization;
- leaves the existing analog PCM sample-width capabilities unchanged.

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
levels, and playback-effect enables that are ineffective in Direct Mode,
including `PCM`, Master, and channel volumes.
Recording and CrystalVoice controls remain available because the reconstructed
stream `0x18` behavior does not prove that capture processing is bypassed.
Output selection, headphone gain, DAC filter, and the Direct Mode switch remain
available. A hardware-backed regression found and fixed a route-validation bug
where an already-enabled X-Bass made `Output Select` appear unavailable during
Direct Mode. Native profiles capture the control automatically when the
patched kernel exposes it.

## Static validation

On 2026-07-25, for the physically tested patch:

- the kernel patch passed `git diff --check`;
- strict `scripts/checkpatch.pl` reported zero errors, warnings, or checks
  across 643 checked lines;
- the patch applied after all five production CA0132 patches and to the
  maintained 6.18.40 backport trees;
- applying the regenerated repository patch to the production-plus-RGB
  baseline reproduced the physically tested `ca0132.c` byte for byte;
- the combined `ca0132.o` and DSP-image parser-test object compiled with
  `W=1` and warnings treated as errors;
- a complete `make modules` pass rebuilt the in-tree signed
  `snd-hda-codec-ca0132.ko` with `W=1 KCFLAGS=-Werror` using the test-kernel
  configuration;
- `cargo test --all-features` passed 47 library and 12 GTK tests;
- `cargo clippy --all-targets --all-features -- -D warnings` passed;
- an isolated Fedora 44 RPM build passed its release tests and metadata checks,
  and the package requires the `pulseaudio-utils` provider of `pactl`;
- the physically tested patch SHA-256 was
  `c05d55c3c827dc035c36614d0c67bd59c14943942d4a9b670dd2c720c65e3257`.

On 2026-07-26, the raw diff was regenerated from pristine ALSA `for-next`
after an independent apply check found that its first context hunk
unnecessarily assumed the LED patch was already present. The current patch:

- applies standalone to `for-next` at `61471f29f315`, after the complete
  production/RGB stack, to clean Linux 6.18.40, and after its maintained
  production/RGB backport stack;
- passes `git diff --check` and strict `checkpatch.pl` with zero findings
  across 642 checked lines;
- compiles standalone and in the complete stack with
  `W=1 KCFLAGS=-Werror`, including the DSP-image parser-test object;
- differs from the physically tested full-stack source only by relocating the
  private `AE5_DIRECT_MAX_ROUTER_ENTRIES` definition, with no executable C
  statement changed;
- has SHA-256
  `49e571c51b035d4feb453ccabb9c42e8b28b699ca1b00ebac9dc34e7d6cbf23a`.

## Physical validation

The guarded test used the exact `1102:0012/1102:0051` AE-5 in a Fedora 44
guest running `6.18.40-ae5-lts-rgb-direct+`. The loaded signed module had
SHA-256
`0a3d637a07b1834dc830e21325b0557a68530b3902aa32a16ac0f7853941db67`.
Headphone gain was Low, headphones were not worn, and the playback fixture
peaked at `-26.02 dBFS`, approximately 5% digital amplitude. Future audio
tests are capped at 20% for both hardware and software volume.

Exact ALSA parameter negotiation in Direct Mode produced:

| Request | Result |
|---|---|
| 48 kHz, stereo | accepted |
| 96 kHz, stereo | accepted |
| 44.1 kHz, stereo | rejected with `EINVAL` |
| 88.2 kHz, stereo | rejected with `EINVAL` |
| 192 kHz, stereo | rejected with `EINVAL` |
| 48 or 96 kHz, six channels | rejected with `EINVAL` |

Both signed 16-bit and signed 32-bit stereo playback succeeded at 48 kHz.
External microphone captures placed Direct 96 kHz within 0.20 dB of Direct
48 kHz.

Normal-route restoration was measured in one
normal-before → Direct → normal-after sequence. The 48 kHz channel levels were
`-65.86/-65.88 dB` before, `-49.10/-49.12 dB` in Direct Mode, and
`-66.01/-66.04 dB` on the first normal playback. A second normal playback was
`-65.76/-65.79 dB`. Both normal results were within 0.16 dB of the baseline,
without a manual ALSA output toggle.

The DSP-bypass pair measured `-49.26/-49.27 dB` with `Enable OutFX` off and
`-49.25/-49.27 dB` with it on, a maximum 0.01 dB difference. Restored normal
effects measured `-66.19/-66.20 dB`, 16.93 dB below the Direct reference.

The route regression cycle selected Speakers and Headphone through the Rust
application while Direct Mode remained enabled. PipeWire and ALSA agreed after
each change. A narrow 1 kHz measurement was
`-114.31/-114.43 dBFS RMS` at the headphone microphone with Speakers selected
and `-78.81/-78.83 dBFS RMS` with Headphone selected, a 35.5 dB minimum
separation. Disabling Direct Mode restored normal playback, the complete mixer
hash, the Headphone PipeWire route, and a closed PCM.

Additional operational gates passed:

- an application transition while raw PCM was running failed because the
  analog PCM remained open after scoped PipeWire suspension;
- a raw ALSA control write while PCM was open returned `EBUSY`;
- Direct Mode remained unchanged after both failures and toggled normally once
  the holder closed;
- ten Direct-to-normal cycles completed 20 transitions and 20 real PCM
  lifecycles with the complete mixer hash unchanged;
- three consecutive warm guest boots loaded the exact signed module, restored
  the same safe post-PipeWire mixer hash, and each completed one Direct and one
  normal PCM lifecycle with Master at 20%, PipeWire at 5%, and a 5% fixture;
- a post-Direct normal What U Hear capture retained a measurable 1 kHz
  component, then closed both playback and capture PCMs with the complete mixer
  hash unchanged;
- final cleanup restored the original guest mixer hash
  `54d927161482ce452096f0e9a66dd958b2e9cde4aa58bd366053308d7ace8b08`,
  shut the guest down, removed its persistent host device, rebound the AE-5 to
  the host stock driver, and restored the host mixer hash
  `3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`;
- the restored host retained the AE-5 default sink, FIFINE default source,
  analog-stereo duplex profile, matched Headphone/Microphone routes, and all
  five active PipeWire/WirePlumber units;
- the final kernel log contained no CA0132, HDA, DSP, timeout, or lock warning
  beyond the expected out-of-tree taint and existing HDA IRQ timing notice.

## Remaining acceptance

Direct Mode stays deferred until all of these complete:

1. At least one host cold boot with automatic card and mode recovery.
2. Twenty bare-metal suspend/resume cycles with playback and exact mixer
   checks. The managed QEMU machine explicitly disables S3 and S4, so a guest
   result cannot satisfy this gate.
3. A connected physical line-out or speaker receiver proving signal on the
   Speakers route, not only headphone suppression.

Every non-silent stream in these remaining gates must first pass the shared
`scripts/audio-parity.sh playback-preflight` against its exact fixture.

Only after those gates pass should any additional Direct rate be investigated.
Static Windows acceptance alone is not a Linux support claim.
