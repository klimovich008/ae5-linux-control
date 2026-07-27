# AE-5 Linux Control maintainer handover

This is the shortest authoritative entry point for a new maintainer and
supersedes older current-state claims elsewhere as of the snapshot date. The
main README contains useful cumulative evidence, but some passages describe
earlier milestones rather than the current development host.

Snapshot date: **2026-07-27**

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
the physical card through VFIO. Persistent S16 playback and fail-closed
hardware output processing remain defense in depth. The rebuilt kernel passed
packaged-kernel VFIO acceptance, is installed side by side, and awaits its
scheduled physical-host boot and analog-output acceptance.

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
- card-specific route discovery and input routing; output/profile transitions
  are currently blocked because they reopen playback;
- PipeWire software volume and explicit route health/repair;
- native profiles, retained imported effect metadata, and a guarded PipeWire
  software-EQ path; unsafe hardware output effects are not applied;
- guarded PipeWire software-EQ generation, graph-signature verification, and
  explicit default-sink activation with fail-closed volume/mute transfer in
  both directions;
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
  passthrough boot, first-open capture, warm/idle, and rejected-OutFX matrices,
  but still needs physical-host boot acceptance;
- `7.1.4-ae5-stable` is installed side by side and selected for the next boot
  only; stock remains the running and saved/default kernel;
- the installed `7.1.4-ae5-guarded` host kernel predates the final fix and must
  not be selected;
- the complete exact-target S32 transition and HDA-position campaign remains
  unrun; the single-client case passed both unlinked and linked virtual graph
  validation but has not run against the AE-5;
- matched Windows/Linux analog response, noise, and headphone-model tuning;
- a physical power-removal cold boot and bounded bare-metal suspend/resume
  with the rebuilt stable-playback kernel;
- connected physical speaker layouts, line-out, optical I/O, and analog inputs;
- Direct Mode remains removed from the production series pending its own
  physical transition acceptance on top of the stable-playback fix;
- external AE-5 LED strip support;
- physical response, latency, CPU, and long-duration stability acceptance for
  the new software-EQ path.

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
9. Until the rebuilt stable-playback kernel passes physical-host acceptance,
   do not suspend, close, or deliberately reopen the normal AE-5 playback PCM
   outside the isolated diagnostic harness. Retain
   `session.suspend-timeout-seconds = 0` as defense in depth.
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

If effects appear inactive, do not toggle hardware OutFX or reapply hardware
effect controls. Preserve logs and mixer readback, keep the physical output
muted, verify the persistent-playback rule, and inspect the PipeWire software
effects graph. Software-EQ activation copies and verifies the current
PipeWire volume and mute state before changing the default sink; an unknown or
mismatched readback must block activation.

## Development-host snapshot

At this handover snapshot, no AE-5 output is connected to the user's
headphones. The headphones are connected directly to the motherboard line
out, so no acoustic AE-5 test is possible without a separate physical
reconnection:

```text
Kernel:            7.1.4-200.nobara.fc44.x86_64
Kernel taint:      0
Next boot once:    7.1.4-ae5-stable
Saved/default:     7.1.4-200.nobara.fc44.x86_64
ALSA card:         0, HDA Creative
Output:            Headphone, 2.0
Input:             Microphone
Desktop sink:      AE-5 default, 5%, muted
Listening output:  motherboard line out
Headphone gain:    Low
Direct Mode:       unavailable on the stock kernel
OutFX:             off
Software EQ:       not installed in the real per-user PipeWire configuration
Playback PCMs:     closed
Audio services:    PipeWire, PipeWire Pulse, and WirePlumber active
System VMs:        both powered off; Windows domain has no hostdev
GUI test:          current debug build opened natively on Wayland
```

At the end of the packaged-kernel and Windows-readiness cycles, Master and
Front were off, OutFX was off, the AE-5 desktop sink was 5% and muted, Low
gain was selected, the exact-card no-suspend property was live, and both
system VMs were shut off. Playback had not yet opened since the final card
rebind, so the PCM was closed; after the first managed playback it should stay
open. Re-read live state before relying on this snapshot.

The installed GUI and CLI are from the reversible per-user installation. The
WirePlumber configuration is linked to
[`packaging/wireplumber/90-ae5-control.conf`](packaging/wireplumber/90-ae5-control.conf).

Four prepared guests exist and were powered off at the snapshot:

- session: `ae5-kernel-test-f44`;
- system: `ae5-kernel-test-f44-system`;
- session: `ae5-windows-compare`;
- system: `ae5-windows-compare-system`.

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
module build, QEMU boot, or VFIO warm reset as a substitute for the stated
physical cold-boot/suspend gate.

The latest checkpoint passed:

- Rust formatting, strict Clippy, and all tests;
- ACP and 54-row feature-ledger validators;
- audio-parity self-test;
- fail-closed stable-playback instrument self-test and a 22/22 clean
  physical-card passthrough matrix at 0.003304–0.003352% THD;
- transactional rootless install lifecycle;
- hosted Rust, RPM lifecycle, and ALSA `for-next` compatibility jobs.

## Repository map

| Path | Purpose |
|---|---|
| `src/device.rs` | Exact PCI/subsystem and ALSA-card discovery |
| `src/controls.rs` | Typed ALSA controls, guards, route repair |
| `src/pipewire.rs` | PipeWire discovery, profiles, routes, suspension |
| `src/eq_chain.rs` | Managed ten-band PipeWire software-EQ graph |
| `src/profile*.rs` | Native profiles and profile library |
| `src/sbcommand.rs` | Bounded Windows settings interoperability |
| `src/bin/ae5-control.rs` | GTK 4 application |
| `feature-parity.tsv` | Authoritative feature/evidence ledger |
| `packaging/` | RPM, ACP, WirePlumber, desktop and udev payload |
| `kernel/series` | Ordered functional kernel patch queue |
| `scripts/` | Build, validation, diagnostics, audio and VFIO gates |
| `scripts/track-transition-stress.sh` | Exact-target client-transition and fail-closed evidence harness |
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
  [`docs/windows-capture/VM-OUTFX-A-B.md`](docs/windows-capture/VM-OUTFX-A-B.md);
- kernel work:
  [`kernel/README.md`](kernel/README.md),
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
   Do not force a production suspend or PCM closure until the new host kernel
   passes acceptance.
2. Complete the already-scheduled one-shot physical boot of
   `7.1.4-ae5-stable`, then run the fail-closed runtime gate before changing
   any control. Do not boot the older `7.1.4-ae5-guarded` artifact.
3. Run a true power-removal cold start and bounded bare-metal suspend/resume,
   then preserve the kernel journal and internal capture evidence.
4. When an AE-5 output is physically available again, complete the guarded
   software-EQ response, latency, CPU, disable/restore, and stability gates in
   [`docs/SOFTWARE_EFFECTS_PLAN.md`](docs/SOFTWARE_EFFECTS_PLAN.md).
5. Run matched, safely attenuated Windows/Linux analog measurements.
6. Finish physical speaker, line-out, optical, and analog-input acceptance.

Do not spend the next session redesigning the GUI or adding another abstraction
before the loud-buzz path is understood. Safety and reproducibility are the
release blockers.
