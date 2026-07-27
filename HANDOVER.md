# AE-5 Linux Control maintainer handover

This is the shortest authoritative entry point for a new maintainer and
supersedes older current-state claims elsewhere as of the snapshot date. The
main README contains useful cumulative evidence, but some passages describe
earlier milestones rather than the current development host.

Snapshot date: **2026-07-27**

## Start from the correct revision

- Public repository: <https://github.com/klimovich008/ae5-linux-control>
- Active integration branch: `agent/refine-gtk-ui`
- Minimum analysis checkpoint: `dc3a36c` (`Correct OutFX bypass semantics`)
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

This is a **usable hardware-specific MVP on the development host**, not a
finished general-purpose Sound Blaster Command replacement.

The source ledger currently tracks 54 Command features:

| Classification | Count | Meaning |
|---|---:|---|
| Verified | 13 | Passed its current evidence gate |
| Intentionally substituted | 13 | Uses a documented Linux-native equivalent |
| Deferred | 18 | Implemented or exposed, but physical/parity acceptance is incomplete |
| Unsupported | 10 | No safe or legal Linux mechanism is currently available |

Run `ae5ctl features`, `ae5ctl features deferred`, and
`ae5ctl features unsupported` for the authoritative per-feature evidence and
remaining gate. The source of that report is
[`feature-parity.tsv`](feature-parity.tsv).

Working on the target host:

- exact AE-5 discovery and live ALSA control;
- native Wayland GTK application and CLI;
- card-specific headphone, speaker-layout, and input routing;
- PipeWire software volume and explicit route health/repair;
- output effects, ten-band EQ, factory EQ presets, and native profiles;
- guarded PipeWire software-EQ generation, graph-signature verification, and
  explicit default-sink activation;
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
- the complete exact-target S32 transition and HDA-position campaign remains
  unrun; the single-client case passed both unlinked and linked virtual graph
  validation but has not run against the AE-5;
- matched Windows/Linux analog response, noise, and headphone-model tuning;
- required cold-boot and bare-metal suspend/resume counts;
- connected physical speaker layouts, line-out, optical I/O, and analog inputs;
- final host acceptance for Direct Mode and several kernel patches;
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

Confirm both AE-5 playback PCMs are closed:

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

## Latest incident and recovery state

On 2026-07-26, S32 desktop playback worked continuously, but a real music
track change produced loud buzzing that desktop mute did not stop. It ended
around the time the playback PCM suspended. There was no matching kernel,
ALSA, PipeWire, or XRUN diagnostic.

A guarded recreation switched 60 short streams across 44.1, 48, and 96 kHz.
Both S32 and S16 runs ended in exact digital silence and logged no error, so
the precise trigger remains unknown. Production configuration was therefore
rolled back to the previously stable:

```text
access:       RW_INTERLEAVED
format:       S16_LE
rate:         48000
period_size:  6016
buffer_size:  24064
```

The same incident left live effect controls different from the selected
personal profile. With the physical output hard-muted, global OutFX was
disabled and the complete `My profile · Headphones` profile was reapplied.
All 21 controls then matched. What U Hear measured a real recovery: +8.95 dB
at 1 kHz and up to 11.40 dB of response-shape change compared with the prior
mismatched state.

If effects appear inactive again, preserve logs and mixer readback first.
Only then hard-mute the physical output, toggle global OutFX off, and reapply
the intended profile. Do not hide a reproducible stale-DSP failure before
collecting evidence.

## Development-host snapshot

At this handover snapshot, no AE-5 output is connected to the user's
headphones. The headphones are connected directly to the motherboard line
out, so no acoustic AE-5 test is possible without a separate physical
reconnection:

```text
Kernel:            7.1.4-200.nobara.fc44.x86_64
Kernel taint:      0
ALSA card:         0, HDA Creative
Output:            Headphone, 2.0
Input:             Microphone
Desktop sink:      AE-5 default, 30%
Listening output:  motherboard line out
Headphone gain:    Medium
Direct Mode:       unavailable on the stock kernel
OutFX:             off
Software EQ:       not installed in the real per-user PipeWire configuration
Playback PCMs:     closed
GUI test:          current debug build opened natively on Wayland
```

The current 30% desktop volume is above the project's 20% test ceiling. It is
a user state, not an approved playback-test state; lower it and rerun the
relevant preflight before any audio-producing test.

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
| `PORT_PLAN.md` | Original scope, architecture, phases and acceptance plan |
| `docs/` | Detailed investigation and validation records |

Read these next according to the task:

- routing or the S16/S32 issue:
  [`docs/DRIVER_ROUTING_INVESTIGATION.md`](docs/DRIVER_ROUTING_INVESTIGATION.md)
  and
  [`docs/TRACK_TRANSITION_INVESTIGATION.md`](docs/TRACK_TRANSITION_INVESTIGATION.md);
- effects or EQ:
  [`docs/DSP_EFFECT_MEASUREMENT.md`](docs/DSP_EFFECT_MEASUREMENT.md) and
  [`docs/SOFTWARE_EFFECTS_PLAN.md`](docs/SOFTWARE_EFFECTS_PLAN.md);
- Windows comparison:
  [`docs/WINDOWS_MIGRATION_VALIDATION.md`](docs/WINDOWS_MIGRATION_VALIDATION.md)
  and [`docs/VFIO_TEST_PLAN.md`](docs/VFIO_TEST_PLAN.md);
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

1. Keep S16 as the managed default. Do not run the new transition harness
   until its documented physical, volume, format, and trace preconditions are
   met.
2. Obtain a root-capable HDA trace session, then run the n>=5 S32 campaign
   with the exact target, validated in-place client, and fail-closed watchdog.
3. When an AE-5 output is physically available again, complete the guarded
   software-EQ response, latency, CPU, disable/restore, and stability gates in
   [`docs/SOFTWARE_EFFECTS_PLAN.md`](docs/SOFTWARE_EFFECTS_PLAN.md).
4. Complete the required cold-boot and bare-metal suspend/resume matrices.
5. Run matched, safely attenuated Windows/Linux analog measurements.
6. Finish physical speaker, line-out, optical, and analog-input acceptance.

Do not spend the next session redesigning the GUI or adding another abstraction
before the loud-buzz path is understood. Safety and reproducibility are the
release blockers.
