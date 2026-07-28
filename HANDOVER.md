# AE-5 Linux Control maintainer handover

This is the shortest authoritative entry point for a new maintainer and
supersedes older current-state claims elsewhere as of the snapshot date. The
main README contains useful cumulative evidence, but some passages describe
earlier milestones rather than the current development host.

Snapshot date: **2026-07-28**

## Start from the correct revision

- Public repository: <https://github.com/klimovich008/ae5-linux-control>
- Active integration branch: `agent/refine-gtk-ui`
- Minimum pre-fix checkpoint: `4d22771` (`Qualify fail-closed S16 transition baseline`)
- Use the active branch head for the guarded PipeWire software-EQ Phase A
  implementation described below.
- Active review: [draft PR #75](https://github.com/klimovich008/ae5-linux-control/pull/75)
- PR #75 is stacked on `agent/import-windows-settings`.
- The default `main` branch is more than 140 commits behind the active
  integration branch. Do not start new work from `main` or retarget/rebase the
  stacked PR without first understanding that history.

Use:

```sh
git clone https://github.com/klimovich008/ae5-linux-control.git
cd ae5-linux-control
git switch agent/refine-gtk-ui
git log -1 --oneline
```

## What this project is

This repository contains:

- a Rust CLI, `ae5ctl`;
- a native Rust/GTK 4 desktop application, `ae5-control`;
- card-scoped ALSA Card Profile and WirePlumber configuration;
- native JSON profiles and a bounded Sound Blaster Command settings importer;
- Fedora/Nobara RPM and reversible per-user installation paths;
- an ordered Linux CA0132 patch queue;
- reproducible software, VM, VFIO, and physical-hardware test procedures.

The only audited hardware target is:

```text
PCI device:    1102:0012
Subsystem:     1102:0051
Codec:         Creative Sound BlasterX AE-5
```

Do not generalize a result to another Creative card or AE-5 revision without
separate discovery, control, routing, and safety evidence.

## Current maturity

This is a **guarded hardware-specific MVP on the development host**, not a
finished general-purpose Sound Blaster Command replacement. The immediate
sound-corruption cause is fixed in the current kernel queue and qualified on
the physical card through VFIO and a true motherboard power-removal boot.
Persistent S16 playback and fail-closed hardware output processing remain
defense in depth. The rebuilt kernel is installed side by side and passed its
bare-metal first-open, warm-reopen, idle, and rejected-OutFX matrix. Connected
analog-output and suspend/resume acceptance remain separate gates.

The user also observed the same fault after a Linux-to-Windows warm boot and
cleared it only by removing motherboard power. Source tracing found that
CA0132 driver removal resets the DSP while the generic HDA shutdown path leaves
it running. The current nine-patch source queue now ends with an AE-5-only
shutdown reset candidate, but the installed `7.1.4-ae5-stable` kernel still
contains the previously accepted eight-patch queue. Do not describe the
warm-handoff issue as fixed until the gates in
[`docs/WARM_REBOOT_DSP_RESET.md`](docs/WARM_REBOOT_DSP_RESET.md) pass.

The source ledger currently tracks 54 Command features:

| Classification | Count | Meaning |
|---|---:|---|
| Verified | 5 | Passed its current evidence gate |
| Intentionally substituted | 14 | Uses a documented Linux-native equivalent |
| Deferred | 25 | Implemented or exposed, but physical/parity acceptance is incomplete |
| Unsupported | 10 | No safe or legal Linux mechanism is currently available |

Run `ae5ctl features`, `ae5ctl features deferred`, and
`ae5ctl features unsupported` for the authoritative per-feature evidence and
remaining gate. The source of that report is
[`feature-parity.tsv`](feature-parity.tsv).

Working on the target host:

- exact AE-5 discovery and live ALSA control;
- native Wayland GTK application and CLI;
- card-specific route discovery and input routing;
- exact output-route transitions on the clean `7.1.4-ae5-stable` kernel, with
  dynamic PipeWire route lookup and fail-closed volume/mute preservation;
- PipeWire software volume and explicit route health/repair;
- native profiles, retained imported effect metadata, and a guarded PipeWire
  software-EQ path; unsafe hardware output effects are not applied;
- guarded in-place PipeWire software-EQ generation, response-aware automatic
  preamp, runtime graph-signature verification, and suspend/load/resume on the
  existing physical sink; no virtual sink or second volume stage is used;
- exact-target, fail-closed track-transition stress, a client-owned in-place
  PipeWire renegotiation probe, and HDA-position trace tooling, implemented
  and self-tested but not yet run on S32;
- 33 embedded Command factory profiles with speaker/headphone variants;
- import of the user's Command speaker/headphone profiles and custom EQ;
- What U Hear digital capture;
- five onboard RGB LEDs with the project kernel and scoped udev rule;
- rootless installation and RPM build/install/remove validation.

Important incomplete areas:

- S32 desktop playback is disabled after a loud real track-switch fault;
- the stable-playback patch passed warm, idle, cold-like, rate, channel, and
  50-cycle VFIO reopen matrices; the exact packaged kernel also passed a fresh
  passthrough boot and a true power-removal bare-metal boot, each with clean
  first-open, warm/idle, and rejected-OutFX matrices;
- `7.1.4-ae5-stable` is installed side by side and running for the accepted
  one-shot boot; stock remains the saved/default kernel;
- the ninth warm-shutdown DSP-reset patch is source-compatible, its exact
  affected objects pass the warnings-as-errors build, and its separate
  `7.1.4-ae5-shutdown` RPM passes non-installing verification, but it is not
  installed or bare-metal accepted yet;
- the installed `7.1.4-ae5-guarded` host kernel predates the final fix and must
  not be selected;
- the complete exact-target S32 transition and HDA-position campaign remains
  unrun; the single-client case passed both unlinked and linked virtual graph
  validation but has not run against the AE-5;
- matched Windows/Linux analog response, noise, and headphone-model tuning;
- a valid Windows post-graphic-EQ or analog capture; `What U Hear` proved the
  imported Acoustic Engine/OutFX boundary but did not contain the displayed
  graphic-EQ curve;
- bounded bare-metal suspend/resume with the rebuilt stable-playback kernel;
- connected physical speaker layouts, line-out, optical I/O, and analog inputs;
- Direct Mode remains removed from the production series pending its own
  physical transition acceptance on top of the stable-playback fix;
- external AE-5 LED strip support;
- broader software-EQ rate/preset coverage and a valid matched Windows post-EQ
  comparison; the latency/topology, CPU, and two-hour stability gates passed.

## Non-negotiable audio safety

The user requires all tests to stay at or below **20%**. Start at **5%**, use
**Low headphone gain**, and keep headphones off the user's head for first
playback after any routing, format, kernel, VM, or DSP-recovery change.

Before investigation or playback setup:

```sh
wpctl set-mute @DEFAULT_AUDIO_SINK@ 1
wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%
ae5ctl set-playback-switch Master off
ae5ctl set-playback-switch Front off
```

Inspect the AE-5 playback PCM without forcing it closed:

```sh
for status in /proc/asound/card*/pcm*p/sub*/status; do
    printf '%s: ' "$status"
    sed -n '1p' "$status"
done
```

The emergency hardware mute is:

```sh
ae5ctl set-playback-switch Master off
```

Additional rules:

1. Never raise the PipeWire sink above 20% during testing.
2. Never use High headphone gain for an acoustic test.
3. Use only fixtures generated by `scripts/audio-parity.sh`; it rejects peaks
   above the project ceiling.
4. Run the relevant `playback-preflight` immediately before a fixture.
5. Re-check PipeWire mute and both hardware switches after every route,
   profile, format, service, kernel, or VM transition.
6. `route-repair` may unmute Master and Front. It intentionally preserves
   user-selected hardware levels.
7. Windows Command route changes may unmute the Windows render endpoint.
8. Do not restore `S32LE` in the managed WirePlumber rule until the loud
   track-switch fault has a physical, fail-closed acceptance result.
9. The rebuilt stable-playback kernel passed its isolated physical-host reopen
   matrix. Retain `session.suspend-timeout-seconds = 0` as defense in depth,
   and use only the bounded suspend campaign for the still-unqualified system
   suspend/resume path.
10. Never write hardware OutFX, its child output effects, hardware EQ, or
    Direct Mode. Use the app guard and guarded kernel.

## Latest incident and recovery state

On 2026-07-26, S32 desktop playback worked continuously, but a real music
track change produced loud buzzing that desktop mute did not stop. It ended
around the time the playback PCM suspended. There was no matching kernel,
ALSA, PipeWire, or XRUN diagnostic.

A later waveform-qualified VFIO matrix found the repeatable trigger: closing
and reopening normal analog playback can alternate between clean output and
approximately 26.4% THD, even at S16 with OutFX off and no mixer write. The
distorted waveform has discontinuities every 16 frames.

The underlying operations are now identified. Generic CA0132 cleanup cleared
the AE-5 playback converter on close, and HDA runtime autosuspend later cleared
the retained assignment after idle. The final AE-5-only patch retains the
converter across PCM close and takes a balanced codec runtime-PM reference.
Controller DMA still stops normally, global HDA `power_save=10` remains
enabled, and system suspend retains normal all-stream cleanup.

The exact functional module passed 50/50 clean reopens after a fresh
host-driver-to-VFIO cycle, plus warm, repeated-idle, 48/96 kHz, and
2/6-channel matrices. A hardware-OutFX enable request was rejected and eight
subsequent captures remained clean. All measurements were bounded internal
What U Hear captures with AE-5 analog outputs unplugged.

The exact packaged kernel then passed the same fail-closed harness after a
true motherboard power-removal boot. All 22 bare-metal captures were clean at
0.002720620–0.002724749% THD with an identical 3.130447865% peak: first open,
12 immediate reopens, one after 20 seconds idle, and eight after exact OutFX
enable was rejected with `EOPNOTSUPP`. Cleanup restored 5% muted, Master and
Front off, Low gain, OutFX off, and both playback PCMs closed.

During the preceding incident, the user also booted Windows without removing
power and heard the same fault there; only a full cold reboot cleared it. This
is user-observed rather than instrumented evidence, but it puts that incident
below the Linux/PipeWire-only boundary and is consistent with AE-5 DSP or PCI
power state surviving a warm OS switch.

Production still uses the conservative desktop baseline:

```text
access:       RW_INTERLEAVED
format:       S16_LE
rate:         48000
period_size:  6016
buffer_size:  24064
suspend timeout: 0 (keep playback PCM open)
```

The application retains output-effect settings in profiles but skips them
during hardware apply. Windows Command OutFX is a software APO master and is
not equivalent to Linux's rejected CA0132 hardware control.

A guarded Windows VM comparison has now confirmed that `What U Hear` is
downstream of the imported Acoustic Engine/OutFX profile. Neutral captures
repeated within 0.00 dB and processed captures within 0.03 dB from 250 Hz
through 16 kHz. The displayed graphic-EQ curve was absent, however, with up to
13.08 dB disagreement from the Linux model, so that endpoint is not accepted
as an EQ-parity reference. See
[`docs/windows-capture/VM-OUTFX-RESULTS.md`](docs/windows-capture/VM-OUTFX-RESULTS.md).

If effects appear inactive, do not toggle hardware OutFX or reapply hardware
effect controls. Preserve logs and mixer readback, keep the physical output
muted, verify the persistent-playback rule, and inspect the PipeWire software
effects graph. Software-EQ activation must leave the physical sink as the
desktop default and keep its existing volume/mute state. A missing
`audioconvert.filter-graph.N` capability, stale target, active OutFX, or
runtime-signature mismatch must block or roll back activation.

## Development-host snapshot

At this handover snapshot, no AE-5 analog output is physically connected. The
headphones are connected to a non-AE-5 output, so no acoustic AE-5 test is
possible without a separate physical reconnection:

```text
Kernel:            7.1.4-ae5-stable
Kernel taint:      0
Saved/default:     7.1.4-200.nobara.fc44.x86_64
ALSA card:         0, HDA Creative
Output:            Headphone, 2.0
Input:             Microphone
AE-5 sink:         not default, 5%, muted
Listening output:  non-AE-5 output
Hardware stages:   Master 99/on, Front 90/on, PCM 255 (audited 0 dB points)
Headphone gain:    Low
Direct Mode:       unavailable
OutFX:             off
Software EQ:       runtime unloaded; managed state absent after full-graph probe
Playback PCMs:     closed
Audio services:    PipeWire, PipeWire Pulse, and WirePlumber active
Session Windows VM: running, logged in; no physical AE-5 hostdev
System VMs:        both powered off
GUI test:          current debug build opened natively on Wayland
```

The latest silent route qualification ended on the exact Headphone route with
the audited soft-mixer hardware stages above, OutFX off, the AE-5 sink at 5%
and muted, Low gain selected, and both playback PCMs closed. The managed EQ
state and runtime marker were absent. Re-read live state before relying on this
snapshot.

The installed GUI and CLI are from the reversible per-user installation. The
WirePlumber configuration is linked to
[`packaging/wireplumber/90-ae5-control.conf`](packaging/wireplumber/90-ae5-control.conf).

Four prepared guests exist. At this snapshot, only the session Windows
comparison guest is running:

- session: `ae5-kernel-test-f44` — shut off;
- system: `ae5-kernel-test-f44-system` — shut off;
- session: `ae5-windows-compare` — running, logged in, no physical AE-5
  hostdev;
- system: `ae5-windows-compare-system` — shut off.

These names and the host PCI address are machine-local facts, not portable
configuration.

## Build and first read-only run

Fedora/Nobara development dependencies:

```sh
sudo dnf install cargo rust alsa-lib-devel gtk4-devel pipewire-devel \
    pipewire-utils pulseaudio-utils sox
```

Build and run the non-mutating checks:

```sh
cargo build --locked --release --all-features
cargo test --all-features
bash scripts/build-transition-helper.sh
cargo run -- status
cargo run -- route-status
cargo run -- features deferred
```

Run the native Wayland GUI:

```sh
GDK_BACKEND=wayland cargo run --features gui --bin ae5-control
```

Install for the current user without root:

```sh
bash scripts/install-user.sh
```

The full RPM prerequisites and lifecycle are in
[`packaging/README.md`](packaging/README.md).

## Validation before publishing a change

Minimum local gate:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
bash scripts/check-ae5-acp-profile.sh
bash scripts/check-feature-parity.sh
bash scripts/audio-parity.sh --self-test
bash scripts/check-ae5-playback-stability.sh --self-test
bash scripts/check-software-eq-performance.sh --self-test
bash scripts/check-user-install.sh
```

For packaging changes:

```sh
bash scripts/build-rpm.sh
```

Run `scripts/check-rpm-lifecycle.sh /path/to/binary.rpm` only as root inside a
disposable container; the script deliberately refuses to run on the host.

For kernel changes, follow [`kernel/README.md`](kernel/README.md) and
[`docs/KERNEL_MAINTENANCE.md`](docs/KERNEL_MAINTENANCE.md). Do not treat a
module build, QEMU boot, or VFIO warm reset as a substitute for the remaining
connected-output and suspend/resume gates.

The latest checkpoint passed:

- Rust formatting, strict Clippy, release build, and 132 GUI-enabled tests;
- ACP and 54-row feature-ledger validators;
- audio-parity self-test;
- fail-closed stable-playback instrument self-test and a 22/22 clean
  physical-card passthrough matrix at 0.003304–0.003352% THD;
- a true power-removal bare-metal 22/22 playback matrix at
  0.002720620–0.002724749% THD;
- repeated bare-metal in-place EQ captures matching the requested 48 kHz
  ten-band response within 0.34 dB;
- an exact-sink software-EQ benchmark with zero added PipeWire buffer frames,
  +0.3990 percentage points of CPU, +178.564 µs work per quantum, and a
  zero-error 7200-second nonzero qualification with 7197 samples and exact
  recovery;
- a guarded Windows `What U Hear` comparison proving the imported
  Acoustic Engine/OutFX boundary while rejecting that endpoint for graphic-EQ
  parity;
- silent physical Headphones → Speakers → Headphones transitions that retained
  exactly 5% muted, kept OutFX off, and left both playback PCMs closed;
- transactional rootless install lifecycle;
- hosted Rust, RPM lifecycle, and ALSA `for-next` compatibility jobs.

## Repository map

| Path | Purpose |
|---|---|
| `src/device.rs` | Exact PCI/subsystem and ALSA-card discovery |
| `src/controls.rs` | Typed ALSA controls, guards, route repair |
| `src/pipewire.rs` | PipeWire discovery, profiles, routes, suspension, direct graph load/unload |
| `src/eq_chain.rs` | Managed ten-band software EQ and automatic-headroom response model |
| `src/profile*.rs` | Native profiles and profile library |
| `src/sbcommand.rs` | Bounded Windows settings interoperability |
| `src/bin/ae5-control.rs` | GTK 4 application |
| `feature-parity.tsv` | Authoritative feature/evidence ledger |
| `packaging/` | RPM, ACP, WirePlumber, desktop and udev payload |
| `kernel/series` | Ordered functional kernel patch queue |
| `scripts/` | Build, validation, diagnostics, audio and VFIO gates |
| `scripts/track-transition-stress.sh` | Exact-target client-transition and fail-closed evidence harness |
| `scripts/check-software-eq-performance.sh` | Exact-sink EQ latency/topology, CPU, and unattended soak gate |
| `scripts/build-transition-helper.sh` | Builds the development-only native PipeWire renegotiation client |
| `tools/pipewire-format-renegotiate.c` | Silent client-owned in-place format/rate probe |
| `scripts/hda-position-trace.sh` | Root-only consumer for upstream HDA lifecycle/position tracepoints |
| `scripts/check-ae5-playback-stability.sh` | Hard-muted first-open, warm, idle, and rejected-OutFX physical acceptance |
| `tools/tone-thd.py` | Internal steady-tone THD analyzer used by the playback gate |
| `PORT_PLAN.md` | Original scope, architecture, phases and acceptance plan |
| `docs/` | Detailed investigation and validation records |

Read these next according to the task:

- routing or the S16/S32 issue:
  [`docs/DRIVER_ROUTING_INVESTIGATION.md`](docs/DRIVER_ROUTING_INVESTIGATION.md)
  [`docs/PCM_REOPEN_EVIDENCE.md`](docs/PCM_REOPEN_EVIDENCE.md), and
  [`docs/TRACK_TRANSITION_INVESTIGATION.md`](docs/TRACK_TRANSITION_INVESTIGATION.md);
- effects or EQ:
  [`docs/DSP_EFFECT_MEASUREMENT.md`](docs/DSP_EFFECT_MEASUREMENT.md) and
  [`docs/SOFTWARE_EFFECTS_PLAN.md`](docs/SOFTWARE_EFFECTS_PLAN.md);
- Windows comparison:
  [`docs/WINDOWS_MIGRATION_VALIDATION.md`](docs/WINDOWS_MIGRATION_VALIDATION.md)
  [`docs/VFIO_TEST_PLAN.md`](docs/VFIO_TEST_PLAN.md), and
  [`docs/windows-capture/VM-OUTFX-RESULTS.md`](docs/windows-capture/VM-OUTFX-RESULTS.md);
- kernel work:
  [`kernel/README.md`](kernel/README.md),
  [`docs/WARM_REBOOT_DSP_RESET.md`](docs/WARM_REBOOT_DSP_RESET.md),
  [`docs/SOURCE_INVENTORY.md`](docs/SOURCE_INVENTORY.md), and
  [`docs/KERNEL_MAINTENANCE.md`](docs/KERNEL_MAINTENANCE.md);
- packaging:
  [`docs/PACKAGING_VALIDATION.md`](docs/PACKAGING_VALIDATION.md).

## What is deliberately not in Git

The repository does not contain:

- Creative binaries, firmware copied from Windows, or proprietary source;
- the Windows VM or Windows installation media;
- VM credentials;
- the user's raw Sound Blaster Command settings or sensitive identifiers;
- raw physical captures;
- saved Windows or local GUI reference screenshots;
- machine-local bootloader, libvirt, or PCI-address configuration.

The committed factory-profile catalog contains only independently representable
interoperability data. Keep that legal and privacy boundary intact.

## Recommended next work

Priority order:

1. Keep S16 and `session.suspend-timeout-seconds = 0` as the managed defaults.
2. Install the verified shutdown-reset candidate side by side for its guarded
   Linux warm-reboot acceptance.
3. Run the bounded bare-metal suspend/resume campaign with connected-headphone
   routing preflight and preserve the kernel journal and route evidence.
4. Broaden software-EQ rate/preset coverage and obtain a valid Windows post-EQ
   comparison in
   [`docs/SOFTWARE_EFFECTS_PLAN.md`](docs/SOFTWARE_EFFECTS_PLAN.md).
5. When an AE-5 output is physically available again, run matched, safely
   attenuated Windows/Linux analog measurements.
6. Finish physical speaker, line-out, optical, and analog-input acceptance.

Do not spend the next session redesigning the GUI or adding another abstraction
before the loud-buzz path is understood. Safety and reproducibility are the
release blockers.
