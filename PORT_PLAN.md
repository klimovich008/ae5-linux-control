# AE-5 Linux Control: Feasibility and Delivery Plan

## 1. Feasibility verdict

A useful Linux replacement is feasible. A literal port of **Sound Blaster
Command** is not: Creative does not publish its source, the Windows application
depends on Creative's Windows driver, and copying its binaries, branding, or
licensed components would create technical and legal problems.

The practical target is a new Linux-native application, provisionally called
**AE-5 Control**, which uses the controls already provided by Linux and adds
small upstream driver changes only where the hardware capability is genuinely
missing.

The application will be written in Rust and will talk directly to ALSA through
`libasound`; it will not parse `amixer` output in production. Rust keeps the
control path fast and memory-safe without adding a managed runtime.

This is more promising than starting a driver from scratch:

- Linux already recognizes the original AE-5 and AE-5 Plus in the upstream
  `snd_hda_codec_ca0132` driver.
- The driver already exposes analog and digital audio, speaker/headphone and
  input selection, 5.1 configuration, headphone gain, DAC filter choice, mixer
  controls, EQ, output effects, and several microphone effects.
- The original AE-5 support was hardware-tested for front/rear headphones,
  line-out, surround, and digital output when it was submitted upstream.
- RGB, true Direct Mode, some speaker calibration functions, Scout Mode, and
  licensed Dolby/DTS encoding are not equivalent to ordinary ALSA mixer
  controls and must be treated as separate features.

Therefore the project has two tracks:

1. Ship the smallest useful userspace controller over the existing ALSA
   controls.
2. Investigate only the missing high-priority functions, one at a time, in the
   upstream kernel driver.

No claim of full parity will be made until the exact card revision has been
tested against a Windows reference and on real Linux hardware.

## 2. Scope

### Version 1: required

- Detect the exact AE-5 card without relying on a fixed ALSA card number.
- Display current hardware/driver capabilities.
- Select speakers, rear headphones, and front-panel headphones where exposed.
- Select microphone, line-in, and front microphone where exposed.
- Control playback/capture volume, mute, channel balance, and mic boost.
- Select 2.0, 2.1, 4.0, 4.1, and 5.1 speaker modes supported by the driver.
- Control full-range speaker flags, bass redirection, and crossover.
- Select low/medium/high headphone gain with an explicit safety warning.
- Select AE-5 DAC roll-off filter.
- Control the available output DSP:
  Surround, Crystalizer, Dialog Plus, Smart Volume, X-Bass, and Equalizer.
- Control the available input DSP:
  Voice Focus, Mic Smart Volume, Noise Reduction, and VoiceFX.
- Edit the ten exposed EQ bands and use/save presets.
- Save, load, import, and export native named profiles.
- Import compatible Sound Blaster Command JSON exports, preview their mapped
  Linux values, and report every setting that cannot be transferred.
- Produce working headphone output on a cold boot without requiring an initial
  ALSA toggle.
- Match the Windows audio path as closely as the hardware, firmware, and
  measurable settings permit; unexplained differences are driver bugs, not GUI
  presets.
- Produce a diagnostics bundle that excludes personal data by default.
- Run without root privileges during normal use.

### Investigate after Version 1

- A genuine DSP-bypass Direct Mode and its supported sample formats.
- Per-channel speaker level and delay calibration.
- Reliable desktop echo cancellation.
- AE-5 card and external-strip RGB control.
- PipeWire-based substitutes where the hardware DSP has no usable interface.

### Not part of the first release

- Creative account, registration, warranty, telemetry, or application updater.
- Creative branding, icons, profile artwork, or proprietary binaries.
- Scout Radar/mobile integration.
- Dolby Digital Live or DTS Connect. These are licensed technologies and are
  not exposed by the current Linux driver.
- Supporting every Creative card. The first target is the user's exact AE-5
  revision; nearby models are added only after separate hardware validation.

## 3. Current capability map

This map is based on the current upstream `ca0132` source. Phase 0 will verify
what the installed kernel actually exposes on the user's card.

| Sound Blaster function | Current Linux position | Initial implementation |
|---|---|---|
| Stereo/5.1 analog playback | In upstream driver | Use existing ALSA PCM and controls |
| Microphone/line capture | In upstream driver | Use existing ALSA PCM and controls |
| S/PDIF PCM output | In upstream driver | Expose status and routing |
| Speaker/headphone switching | Exposed as ALSA control | Wrap existing control |
| Headphones require a first-use ALSA toggle | Two 2026 upstream fixes address default auto-detect and manual selection behavior | Test current upstream, then backport or continue root-cause work |
| Input selection | Exposed as ALSA control | Wrap existing control |
| Headphone gain | AE-5-specific ALSA enum | Wrap with a volume safety guard |
| Slow/minimum/fast DAC filter | AE-5-specific ALSA enum | Wrap existing control |
| Surround/Crystalizer/Dialog+ | Exposed switches and levels | Wrap existing controls |
| Smart Volume/X-Bass | Exposed switches, modes, and levels | Wrap existing controls |
| Ten-band EQ and presets | Exposed by driver | Wrap bands and store user presets |
| Speaker modes/bass redirection | Exposed by driver | Wrap existing controls |
| Noise reduction/Mic SVM/VoiceFX | Exposed by driver | Wrap controls and presets |
| Echo cancellation | Code exists, but is deliberately skipped on desktop cards because it is known to break them | Keep disabled until a driver fix is proven |
| What U Hear | Exposed as capture PCM/mixer controls | Test and expose if present |
| True Direct Mode | No clear public ALSA control | Driver research gate |
| Speaker distance calibration | Hardware requests are known internally but not offered as a stable public control | Driver or PipeWire research gate |
| RGB/Aurora | No current `ca0132` interface | Separate kernel/OpenRGB workstream |
| Scout Mode | No current upstream interface | Exclude initially |
| Dolby/DTS live encoding | No current upstream interface | Exclude |
| Windows profile migration | Command documents JSON profile export/import, and the AE-5 software release added EQ import/export | Parse verified exports and convert supported fields |
| Windows/Linux sound mismatch | Likely spans driver initialization, DSP/speaker-EQ state, Direct Mode, and desktop resampling | Measure each layer and fix the first divergent shared path |

## 4. Minimal architecture

```text
Sound Blaster Command JSON ── migration importer ─┐
                                                 │
Rust/GTK application ─┐                           │
                     ├─ shared Rust backend ──────┴─ libasound ── ALSA controls
CLI / test commands ─┘          │
                                ├─ wpctl (routing/default-device actions only)
                                └─ native JSON profiles in the user's XDG config

PipeWire/WirePlumber ── ALSA PCM devices ── snd_hda_intel + ca0132 ── AE-5
```

Implementation defaults:

- Stable Rust in one Cargo package, with the GUI and diagnostic commands
  sharing the same backend.
- GTK 4 through `gtk4-rs`, using distribution-provided GTK libraries.
- The Rust `alsa` crate over `libasound` for card discovery, typed controls,
  value read/write, polling descriptors, and change notifications.
- `serde`/`serde_json` for validated native and migrated profiles.
- `amixer` only as an independent diagnostic/reference tool.
- `wpctl` only for PipeWire routing that ALSA does not own.
- Exact ALSA control names and runtime capability discovery, not hard-coded
  `numid` values or card indexes.
- No daemon, database, web service, Electron runtime, or root helper.
- No custom kernel module unless a missing function first proves that an
  upstreamable `ca0132` change is required.

Optimization is measured rather than assumed:

- Event-driven ALSA notifications; no idle polling loop.
- No application-side audio processing in Version 1—the hardware DSP and
  PipeWire remain in the audio path, while the app only changes controls.
- Release builds use normal Rust optimization and Thin LTO if measurement
  shows a useful size/startup improvement.
- Initial reference targets: effectively zero idle CPU, control readback shown
  within 100 ms, cold start below one second, and resident memory below 100 MiB
  on the agreed reference system.
- The reproducible five-run baseline and before/after profile are recorded in
  [`docs/GUI_PERFORMANCE.md`](docs/GUI_PERFORMANCE.md).
- A slower metric is profiled before adding caches, threads, or unsafe code.

### 4.1 Windows settings migration

Creative documents Sound Blaster Command profile export/import as JSON on
supported products, and the AE-5 Command release notes explicitly include
EQ-profile import/export. Exact fields can still vary by card and application
version, so the importer will recognize observed schemas rather than assume
that every Command JSON file is identical.

Preferred migration flow:

1. In Windows, export each SBX and EQ profile that Command allows.
2. Copy the original JSON files to Linux without modifying them.
3. Select **Import from Sound Blaster Command**.
4. Validate the JSON structure, source application/card metadata when present,
   numeric ranges, and file size.
5. Show a preview containing:
   profile name, exact mappings, approximations, unsupported fields, and the
   current Linux values that would change.
6. Snapshot the current ALSA state, apply the accepted mapping, read it back,
   and roll back if any write fails.
7. Save the successful conversion as a native AE-5 Control profile while
   leaving the source file untouched.

Initial mappings:

| Windows setting | Linux target |
|---|---|
| Profile name | Native profile name |
| EQ enabled/preamp/ten bands | Equalizer switch and exposed EQ controls |
| Surround, Crystalizer, Bass, Dialog+ switches and levels | Corresponding `ca0132` switches and level controls |
| Smart Volume switch, level, and mode | Smart Volume controls and Normal/Loud/Night enum |
| Output, speaker mode, bass redirection, gain, or filter when present | Apply only when the exact ALSA capability exists |
| Unknown, licensed, RGB, Scout, or unsupported fields | Preserve in the migration report; never silently pretend they were applied |

The importer will not execute content, follow paths embedded in a profile, or
guess at an unknown schema. A malformed or newer file gets a clear
“unsupported format” report and changes nothing.

If the AE-5 build does not export a required setting, the first fallback is a
guided migration form based on Windows screenshots. A small read-only Windows
collector will be considered only after Phase 0 proves where those settings
are stored and that reading them is more reliable than manual entry. No broad
registry/configuration scraper is part of Version 1.

## 5. Delivery phases

### Phase 0 — Hardware and reference audit

Goal: identify the exact card, installed audio stack, and Windows behavior
before writing application code.

On Linux, collect:

```sh
uname -a
lspci -nnk -d 1102:
cat /proc/asound/cards
aplay -l
arecord -l
amixer -c <card> controls
amixer -c <card> contents
wpctl status
alsa-info.sh --no-upload
journalctl -k -b | grep -Ei 'ca0132|snd|hda|firmware'
```

Record:

- AE-5, AE-5 Pure, or AE-5 Plus model and board revision.
- PCI ID, HDA codec ID, and subsystem ID.
- Distribution, kernel, PipeWire, WirePlumber, ALSA, and firmware versions.
- Every connected jack, speaker layout, headphone impedance, microphone, and
  whether the front-panel HD Audio header is used.
- Current failures: missing device, no output, wrong channel map, lost state,
  noise, microphone problems, or only the lack of a control application.
- Whether the running kernel contains upstream commits `778031e1658d`
  (auto-detect default) and `6fd9f6e870ea` (manual output selection).

On Windows, if dual boot remains available:

- Record the exact Sound Blaster Command and Creative driver versions.
- Export or screenshot every page shown for this exact card.
- Record every enum choice and numeric range.
- Use Command's own export action for every available SBX and EQ profile, and
  retain the original JSON files as migration fixtures.
- Capture a clean baseline with all processing off, then one feature at a time.
- Use identical test audio, sample rate, volume, output, and physical cabling
  for the later Linux comparison.

Deliverables:

- `hardware-report.txt`
- `linux-controls.txt`
- `feature-parity.tsv`
- `windows-exports/` containing unmodified, user-approved profile exports
- Windows reference screenshots supplied by the user

Exit criterion: the exact hardware identity is known and basic Linux stereo
playback either works or has been reduced to a reproducible driver bug. If
basic playback is broken, application work pauses and the driver bug becomes
the first fix.

### Phase 1 — Zero-code control proof

Goal: prove which functions already work through stock ALSA.

For every reported ALSA control:

1. Read its type, valid range, enum labels, channel count, and current value.
2. Save the original value.
3. Apply safe low/middle values while playing or recording a known fixture.
4. Read the value back.
5. Verify the audible or measurable hardware change.
6. Restore the original value.

Tests start with speakers or disposable low-cost headphones at low volume.
High headphone gain is never selected automatically.

Deliverable: a confirmed mapping from each Linux control to the corresponding
Command feature, with one of four states:

- supported and verified;
- exposed but defective;
- absent but replaceable in PipeWire;
- absent and requires driver/reverse-engineering work.

Exit criterion: every Version 1 control has a known Linux mechanism or is
explicitly removed from Version 1.

### Phase 1A — Driver stabilization gate

Goal: fix the two reported blockers before hiding them behind an application.

#### Headphones require an initial ALSA toggle

The reported symptom closely matches an upstream `ca0132` bug fixed in 2026:
HP/Speaker auto-detect had historically defaulted off, so jack detection worked
only after the user enabled it in `alsamixer`. A follow-up fix ensures that an
explicit manual output selection disables auto-detect and actually changes the
route.

Test in this order:

1. Reproduce on the user's distribution kernel and save mixer state, HDA pin
   configuration, jack state, dynamic-debug output, and kernel log.
2. Test the same cold-boot case on a kernel containing both upstream fixes.
3. Cover rear headphones present at boot, front-panel headphones present at
   boot, plugging after boot, unplugging, manual speaker selection, suspend,
   resume, and DSP reload.
4. If the two commits solve it, backport them to the target maintained kernel;
   do not write a competing workaround.
5. If it remains, trace the common initialization path from
   `ca0132_init()` through `ca0132_alt_select_out()`, including the
   auto-detect flag, detected jack, selected output, AE-5 MMIO route, pin
   controls, and final DSP unmute.
6. Patch the earliest incorrect shared state transition, add a hardware
   regression script, and submit the fix upstream.

Pass condition: with headphones already connected, ten cold boots and twenty
suspend/resume cycles produce audio on the correct jack without opening a
mixer or toggling any control.

#### Sound does not match Windows

“Performance” is converted into measurements before changing code:

- output level and channel balance;
- frequency response;
- noise floor and THD+N within the measurement interface's limits;
- channel separation;
- sample format/rate and unexpected resampling;
- end-to-end latency;
- DSP, EQ, gain, DAC filter, and speaker/headphone profile state.

Isolation order:

1. Use identical Windows and Linux source files, physical output, cable,
   capture interface, gain, sample rate, and disabled effects.
2. Compare direct ALSA `hw:` playback with normal PipeWire playback. If only
   PipeWire differs, fix its profile/rate configuration rather than the kernel.
3. Confirm the AE-5 desktop DSP firmware loaded successfully and record every
   relevant ALSA control and driver default.
4. Compare DSP-disabled stereo, DSP-enabled flat, each DAC filter, each
   headphone gain, and representative SBX settings separately.
5. Investigate known upstream gaps as hypotheses: true Direct Mode is not
   exposed, the driver comments that `ctspeq.bin` speaker/headphone EQ data is
   unused, and output selection currently clears
   `SPEAKER_TUNING_USE_SPEAKER_EQ`.
6. Reproduce the Windows register/DSP sequence for the first divergent mode,
   implement only the understood missing step in `ca0132`, and rerun the full
   matrix.

The app will not ship a compensating EQ that merely masks a driver routing or
DSP-initialization bug.

Initial parity targets with processing disabled are: output level within
0.5 dB, frequency-response delta within 1 dB over 20 Hz–20 kHz, no unexplained
sample-rate conversion, and noise/THD results within 3 dB or the repeatability
limit of the measurement setup. Any larger remaining difference must have an
identified cause and documented limitation.

Exit criterion: headphone startup is reliable, and the Windows/Linux
measurement report either meets the parity targets or identifies a specific
remaining hardware/firmware feature rather than a generic subjective mismatch.

### Phase 1B — Reference-source and interoperability research

Goal: recover the missing hardware behavior from public source and observable
interfaces without copying Creative's proprietary implementation.

Use sources in this order:

1. The current upstream Linux `ca0132` driver and its Git history. This is the
   authoritative implementation base and already contains AE-5 MMIO, GPIO,
   routing, DSP requests, mixer controls, and comments recording earlier
   Windows-reference findings.
2. Connor McAdams' `ca0132-tools`. These provide CA0132 register, ChipIO, 8051,
   DSP-command, and frame-inspection utilities. Respect the repository's
   explicit warning not to run its DSP disassembler on Creative firmware.
3. Connor McAdams' `QemuHDADump`, which captures HDA CORB verbs issued by a
   driver in a QEMU guest. It is a better reference for state transitions than
   guessing from a decompiler, but use it with the Creative Windows driver only
   after the licensing gate below is cleared.
4. The merged OpenRGB AE-5/AE-5 Plus implementation. Its GPL source documents
   PCI IDs, five internal LEDs, the command packet layout, and the Windows
   driver IOCTL used for lighting. The merged implementation is Windows-only,
   so Linux RGB still needs a narrow kernel interface.
5. Creative's public manuals, profile exports, firmware already distributable
   through `linux-firmware`, and hardware measurements.
6. Targeted static or dynamic analysis of a legally obtained Windows package
   only when all earlier sources leave a specific behavior unknown.

Initial findings:

- No Creative-published Windows driver source has been located.
- The upstream source is unusually valuable: it identifies requests for
  speaker level, delay, inversion, bass management, and a speaker/headphone EQ
  upload path. It also notes that `ctspeq.bin` is currently unused and that
  Windows enables `SPEAKER_TUNING_USE_SPEAKER_EQ` after uploading a profile.
  This is a concrete hypothesis for the measured Windows/Linux sound gap.
- The two 2026 upstream headphone-selection commits are direct candidate fixes
  for the first-use toggle symptom and should be tested before new code.
- OpenRGB merge request !2997 adds an AE-5 command structure and a Creative
  driver IOCTL for lighting, but not a Linux hardware backend.

Licensing and clean-room gate:

- Creative's official package page offers
  `AECMDMasterInstaller_3.4.92.00.exe`, but the agreement displayed before
  download restricts decompilation, disassembly, memory dumps, and reverse
  engineering, and directs EU users seeking interoperability information to
  ask Creative. The project will request that information first.
- Do not accept the agreement, download the package, extract binaries, or run a
  decompiler on another person's behalf without confirming that the person
  doing the analysis has a lawful basis or Creative's permission. This plan is
  not legal advice; local law and the applicable licence must be checked.
- Do not commit, quote, translate, or redistribute Creative object code,
  decompiler output, symbols, private data, or firmware not already licensed
  for Linux distribution.
- Record only independently usable interface facts: device IDs, HDA verbs,
  IOCTL/property identifiers, packet layouts, register addresses, DSP
  parameter IDs, state ordering, timing, inputs, and observed outputs.
- Keep the implementation reviewable against public source and behavioral
  tests. A contributor who reads proprietary decompiler output must not copy
  control flow or code into the GPL kernel patch.

If targeted proprietary analysis is legally cleared, use a bounded workflow:

1. Record package URL, version, signer, timestamp, and SHA-256; keep the package
   outside the repository and never execute it on the Linux development host.
2. Extract it in an isolated Windows/QEMU analysis environment and inventory
   only relevant signed `INF`, `CAT`, `SYS`, `DLL`, firmware, and profile files.
3. Parse `INF` device IDs, services, interfaces, and registry defaults before
   opening any binary in a decompiler.
4. Diff versions and search imports, exports, strings, resources, and constants
   to narrow the question. Do not attempt a full application decompile.
5. Inspect only entry points related to the named gap: PnP/power callbacks,
   property/IOCTL dispatch, HDA verb submission, DSP upload, output selection,
   or speaker/headphone profile loading.
6. Correlate static findings with one-setting-at-a-time Windows traces and
   audio measurements. Mark facts as observed, inferred, or still unknown.
7. Write a hardware behavior specification and regression test from those
   facts, then implement the smallest independent change in `ca0132`.

Deliverables:

- A versioned source/reference inventory with licences and exact commit IDs.
- A protocol notebook containing only permitted interface facts and evidence.
- A Windows-versus-Linux state-sequence diff for each investigated feature.
- A test that fails before and passes after any resulting independent patch.

Exit criterion: every fact used by the implementation is traceable to public
source, permitted observation, or explicitly cleared analysis, and no
proprietary material enters the repository.

### Phase 2 — Backend and command-line MVP

Goal: make the verified controls safe, repeatable, and scriptable before
building the GUI.

Implement:

- Card discovery by vendor/model/control fingerprint.
- A capability query returning only controls that really exist.
- Typed Rust getters/setters for allow-listed ALSA controls through
  `libasound`.
- Event-driven notification when controls change outside the application.
- Range and enum validation before calling ALSA.
- Snapshot, restore, and profile apply.
- Rollback to the pre-apply snapshot when a multi-control profile fails.
- A Sound Blaster Command JSON importer with preview and migration report.
- Human-readable errors for missing permissions, missing firmware, renamed
  controls, and unsupported kernel versions.
- A diagnostics command.

Profiles store semantic names and values, not ALSA card indexes or `numid`s.
Unknown fields are ignored on import; invalid and unsafe values are rejected.

Checks:

- One control-discovery/profile test using a recorded fake ALSA backend.
- Golden conversion tests using sanitized AE-5 Command exports.
- Rejection tests for malformed, oversized, unknown-version, and out-of-range
  Command profiles.
- One hardware smoke script that reads all controls, changes a safe control,
  confirms readback, and restores the snapshot even after failure.

Exit criterion: all supported functions can be operated and restored from the
CLI without root and without leaving the card in an unknown state.

### Phase 3 — Native desktop application

Goal: expose the proven backend without inventing a second control path.

Pages:

- Device and diagnostics
- Playback/output
- Speakers and bass management
- SBX/output effects
- Equalizer and profiles
- Recording/input effects

Behavior:

- Provide an **Import from Sound Blaster Command** preview that separates exact,
  approximate, and unsupported mappings before anything is applied.
- Hide unsupported controls instead of displaying nonfunctional switches.
- Refresh the UI when a control changes outside the app.
- Disable mutually incompatible settings, such as X-Bass and 5.1 bass
  redirection, with a short explanation.
- Warn before raising headphone gain.
- Provide keyboard operation, labels for screen readers, visible focus, and
  sufficient contrast.
- Never overwrite a saved profile without confirmation.

Exit criterion: a user can reproduce all verified Version 1 settings without
opening a terminal, and the GUI passes the same hardware smoke test through
the shared backend.

### Phase 4 — Targeted kernel work

Goal: implement only gaps that cannot be solved correctly in userspace.

Each feature is a separate investigation and patch series:

1. Reproduce and document the missing behavior.
2. Compare the current `ca0132` path, existing reverse-engineered requests,
   known firmware behavior, and a Windows hardware reference.
3. Add the smallest typed ALSA or kernel-subsystem interface that represents
   the feature safely.
4. Reject invalid values and restore a safe state after errors, suspend, and
   driver unload.
5. Build with warnings enabled, run kernel style/static checks, and test on
   the exact hardware.
6. Submit the change upstream rather than making users depend permanently on
   a private kernel build.

Candidate order:

1. Remaining measured Windows-parity gap from Phase 1A.
2. True Direct Mode and supported high-resolution formats.
3. Speaker level/delay calibration.
4. Desktop echo cancellation, only if it can be made reliable.
5. RGB through an appropriate kernel LED interface, then reuse it from
   OpenRGB or AE-5 Control.

RGB work must build on the existing OpenRGB investigation, which has Windows
support but still identifies Linux as needing a kernel-driver interface. It
must not use `/dev/mem` or unrestricted userspace MMIO.

Temporary test kernels remain boot-menu alternatives to the known-good stock
kernel. Proprietary Creative binaries or firmware are not copied into the
repository.

Exit criterion for each feature: the new interface has readback, validation,
power-management coverage, clean kernel logs, and repeatable hardware results.

### Phase 5 — Packaging and wider compatibility

Goal: make the verified application easy to install without expanding the
support claim prematurely.

- Package first for the user's distribution.
- Add a desktop entry and only the runtime dependencies actually used.
- Use normal audio-group/session permissions; do not install a setuid helper.
- Test the current stable kernel and one maintained LTS kernel.
- Add other distributions and AE-series cards only with contributed hardware
  reports and smoke-test results.
- Consider Flatpak/AppImage only if their device-access model works without
  broad permissions; native packages come first.

Exit criterion: a clean machine can install, detect, configure, uninstall, and
return to standard ALSA behavior without manual cleanup.

## 6. Test strategy

### 6.1 Software tests without hardware

- Exercise discovery and typed values through a fake ALSA-control backend built
  from real, sanitized card snapshots.
- Compare diagnostic snapshots against independent `amixer` output.
- Reject wrong devices that happen to contain similarly named controls.
- Validate every enum and numeric boundary.
- Verify profile round trips and schema migration.
- Convert sanitized Command JSON fixtures to expected native profiles.
- Reject malformed, deeply nested, oversized, unknown-schema, non-finite, and
  out-of-range imported values without changing ALSA state.
- Verify unsupported source fields appear in the migration report.
- Inject a failed control write and verify rollback order.
- Verify commands are invoked without shell interpolation.
- Verify diagnostics redact user names and unrelated devices by default.

### 6.2 Hardware functional matrix

Run through direct ALSA and the normal PipeWire desktop path where applicable:

| Area | Cases |
|---|---|
| Output | rear line-out, rear headphone, front-panel headphone, S/PDIF |
| Speaker layout | 2.0, 2.1, 4.0, 4.1, 5.1 |
| Input | rear mic, rear line-in, front mic, What U Hear |
| Formats | 44.1/48/96 kHz at supported depths; higher rates only after Direct Mode proof |
| Controls | min/middle/max safe values, mute, balance, external-change refresh |
| DSP output | each effect alone, combined profile, all effects off |
| DSP input | each available effect alone, VoiceFX preset, all effects off |
| Lifecycle | cold boot, warm reboot, suspend/resume, app restart, driver reload in a controlled test |
| Concurrency | change settings during playback, recording, and full-duplex use |

Use `speaker-test` for channel identity and known WAV fixtures for playback.
Use `arecord`/`pw-record` for capture. Tests must restore the starting mixer
snapshot.

### 6.3 Objective audio comparison

Use a fixed reference set:

- logarithmic sine sweep;
- individual 60 Hz–16 kHz tones matching EQ bands;
- pink noise;
- transient/level-step signal for Smart Volume;
- speech plus controlled background noise for microphone processing;
- discrete six-channel identification file.

Preferred measurement path:

1. AE-5 output into a separate calibrated USB audio interface.
2. Fixed analog gain, sample rate, cabling, and capture interface.
3. Windows capture with processing off and then one feature at a time.
4. Linux capture with the corresponding setting.
5. Compare frequency response, RMS/peak level, channel routing, noise floor,
   latency, and time-varying gain.

A line-out-to-line-in loopback may be used for an initial functional check, but
not as the final noise/distortion measurement because the AE-5 input becomes
part of the result.

### 6.4 Stability and regression

- Two hours of simultaneous playback and capture with no unexpected XRUNs.
- Ten cold/warm boot checks.
- Twenty suspend/resume cycles.
- Repeated output switching during playback at low volume.
- Profile apply/restore loop for every saved profile.
- Verify no new kernel warnings, codec timeouts, firmware failures, or stack
  traces.
- Re-run the matrix after every kernel patch and on the packaged build.

### 6.5 WSL, Docker, virtual machine, and hardware roles

No single environment can test every layer:

| Environment | Use it for | It cannot prove |
|---|---|---|
| WSL 2 + WSLg | Rust builds, `cargo test`, `clippy`, profile migration, fake-ALSA tests, GTK layout/input/accessibility checks, kernel compilation and static checks | AE-5 discovery, ALSA hardware controls, PCI/MMIO, jack detection, real audio, suspend/resume |
| Docker/Podman | Reproducible Ubuntu/Fedora build images, dependency checks, Rust tests, kernel compilation, `checkpatch`, `sparse`, and packaging tests | An isolated replacement audio driver; containers use the host kernel |
| Container with `/dev/snd` on a Linux AE-5 host | Userspace integration against the host's already-loaded driver | Testing a different kernel driver safely or Windows-quality audio parity |
| QEMU/KVM with VFIO PCI passthrough | Booting test kernels with the real AE-5 assigned to a disposable Linux VM | WSL/Windows-host testing without PCI passthrough; complete protection from bad hardware writes |
| Bare-metal Linux or a Linux live/dual-boot installation | Final driver, cold-boot, jack, suspend/resume, PipeWire, latency, and analog measurement tests | Nothing in the required hardware matrix; this is the release gate |

WSL can boot a custom kernel and modules image, which is useful for build and
generic boot checks. It does not receive the physical AE-5 PCI function.
WSLg presents audio through a PulseAudio server and RDP audio transport, so
hearing sound from the GUI in WSL tests Windows' audio path, not `ca0132`.

Docker device access is not a safety boundary for driver work. Passing
`/dev/snd` exposes devices owned by the host kernel, while `--privileged` grants
broad host access and will not be used for routine tests.

Recommended workflow:

1. WSLg for daily Rust development and interactive GUI checks.
2. Containers for reproducible build, lint, static-analysis, and packaging
   jobs.
3. QEMU/KVM PCI passthrough when a Linux host and suitable IOMMU group are
   available.
4. A stock-kernel boot entry plus a separate test-kernel entry on bare metal
   for mandatory final validation.

## 7. Acceptance criteria

Version 1 is accepted only when:

1. The exact card is detected correctly on every test boot.
2. Headphones connected before boot work immediately without an ALSA toggle,
   and automatic/manual output selection remains correct after resume.
3. Processing-disabled Linux output meets the Phase 1A Windows-parity targets,
   or every remaining difference is tied to a named unsupported feature.
4. Basic stereo, every connected physical output, and every connected input
   pass a real signal.
5. The 5.1 channel test reaches the correct physical speakers with no swapped
   channels.
6. Every displayed setting passes write/readback and produces the expected
   hardware or DSP change.
7. EQ measurements match the requested band gains within 1 dB over the
   relevant test frequencies.
8. Output and microphone effects have repeatable on/off differences consistent
   with the Windows reference; nonlinear effects are compared by behavior,
   not bit identity.
9. A verified Command export imports to the expected Linux controls, leaves
   the source file unchanged, and lists every unsupported field.
10. Profiles survive app restart and reboot, and partial failures roll back.
11. Normal use requires no root access.
12. The Rust release build meets the agreed startup, idle CPU, response, and
    memory budgets on the reference system.
13. The stability suite completes with no new kernel error and no app crash.
14. The feature-parity table names every Sound Blaster Command feature as
    verified, intentionally substituted, deferred, or unsupported.

“Works as intended” means these results are saved with command output, logs,
and measurement artifacts. It does not mean “the UI opened” or “sound was
heard once.”

## 8. Risk gates and fallbacks

| Gate | Decision |
|---|---|
| Card subsystem ID is not mapped correctly | Add/test the smallest upstream `ca0132` quirk before app work |
| Stock driver cannot play stable stereo | Diagnose/fix the driver first |
| ALSA control exists but is unsafe or unreliable | Hide it in the app and fix the shared driver path |
| Feature can be implemented cleanly in PipeWire | Use PipeWire rather than adding kernel code |
| Feature requires proprietary algorithm or license | Exclude it or offer a clearly named open substitute |
| RGB requires raw MMIO from userspace | Do not ship it; add a constrained kernel interface first |
| A private kernel patch regresses the system | Boot the stock kernel and restore the saved ALSA state |
| No access to the physical card | Continue software tests, but do not mark hardware milestones complete |
| A test passes only in WSL or Docker | Count it as a software/build result, never a driver or audio-quality result |

## 9. Inputs needed when implementation starts

- Exact AE-5 model and the Phase 0 Linux report.
- Intended Linux distribution and desktop.
- Speaker/headphone/microphone wiring and headphone impedance.
- A ranked list of the settings that block migration to Linux.
- Sound Blaster Command/driver versions and at least one unmodified exported
  profile, with any personal naming removed by the user if desired.
- Whether Windows dual boot is available for reference captures.
- Whether bare-metal Linux or QEMU/KVM PCI passthrough is available for driver
  tests; WSL/Docker alone are insufficient.
- Whether Creative has supplied interoperability documentation or the user has
  obtained legal clearance for any proprietary-driver analysis.
- Whether RGB, Direct Mode, 5.1, microphone processing, or a particular SBX
  profile is essential to the first usable release.

If the stock controls behave as the upstream source indicates, a useful audio
control MVP should be much smaller than a full driver project. Full
Sound Blaster Command parity remains an open-ended reverse-engineering project,
mainly because of Direct Mode, RGB, Scout features, and licensed encoding.

## 10. Primary references

- [Current upstream Linux `ca0132` driver](https://github.com/torvalds/linux/blob/master/sound/hda/codecs/ca0132.c)
- [Upstream fix: default HP/Speaker auto-detect from the headphone pin](https://github.com/torvalds/linux/commit/778031e1658d)
- [Upstream fix: make manual output selection disable auto-detect](https://github.com/torvalds/linux/commit/6fd9f6e870ea)
- [Original AE-5 Linux support patch series and hardware test report](https://lkml.iu.edu/hypermail/linux/kernel/1809.2/02714.html)
- [CA0132 hardware and DSP inspection tools](https://github.com/Conmanx360/ca0132-tools)
- [QEMU HDA verb capture tool](https://github.com/Conmanx360/QemuHDADump)
- [GTK's official Rust bindings](https://www.gtk.org/docs/language-bindings/rust)
- [Microsoft WSL custom-kernel/modules configuration](https://learn.microsoft.com/windows/wsl/wsl-config)
- [Microsoft WSLg architecture and virtual audio path](https://github.com/microsoft/wslg)
- [Docker device access and `/dev/snd` example](https://docs.docker.com/engine/containers/run/)
- [Creative Sound Blaster Command Software Guide](https://download.creative.com/manualdn/Manuals/TSD/14190/qfWrGlGToW/Sound%20Blaster%20Command%20Software%20Guide.pdf)
- [Creative Command AE-5 release notes: EQ profile import/export](https://support.creative.com/downloads/DriverDetails.aspx?driverID=100330)
- [Creative Command profile JSON export/import documentation](https://support.creative.com/kb/ShowArticle.aspx?sid=200385)
- [Creative AE-5/AE-5 Plus technical specifications](https://support.creative.com/kb/ShowArticle.aspx?sid=138952)
- [OpenRGB AE-5/AE-5 Plus investigation](https://gitlab.com/CalcProgrammer1/OpenRGB/-/work_items/2378)
- [Merged OpenRGB AE-5/AE-5 Plus Windows support](https://gitlab.com/CalcProgrammer1/OpenRGB/-/merge_requests/2997)
