# AE-5 Linux Control roadmap

This is the authoritative execution order and completion ledger. It turns the
original scope in [PORT_PLAN.md](PORT_PLAN.md), the incident history in
[GOAL.md](GOAL.md), and the current-state snapshot in
[HANDOVER.md](HANDOVER.md) into one sequence with explicit exit criteria.

Last audited: **2026-07-30**

## Current state

The project already provides a guarded, hardware-specific MVP:

- a Rust CLI, the temporary native GTK 4 fallback, and the selected Qt 6/QML
  Sound screen backed by the `ae5d` user service;
- exact AE-5 discovery and checked ALSA/PipeWire controls;
- native profiles, 33 embedded Command profile pairs, and personal Windows
  settings migration;
- a guarded in-place software equalizer;
- Fedora/Nobara RPM and reversible user installation;
- an ordered CA0132 patch queue with physical-card, VM, package, and upstream
  source validation.

It is not a finished release. The current compatibility ledger has 54 rows:
5 verified, 14 intentionally substituted, 25 deferred pending implementation
or physical acceptance, and 10 unsupported. Unsupported proprietary or absent
features do not block Version 1 when the UI identifies them honestly.

The repository history was consolidated on 2026-07-28. Pull request #75 merged
the 188-commit integration history into `main`; the 74 superseded stacked
drafts were closed with their branches and review history retained. CI now
runs once per PR update and once after a merge to `main`.

## Definition of done

### Daily-use release candidate

Maks can use the packaged app for ordinary headphone playback when all of
these are true:

1. The current integration history is on `main`, old stacked PRs are closed,
   and at most one incremental PR remains open.
2. The application records bounded, privacy-conscious startup, route, profile,
   EQ, mixer-write/readback, recovery, and error diagnostics by default.
3. The installed stable-playback kernel has no accepted first-open, reopen, or
   idle corruption regression, and the warm-shutdown candidate passes its
   Linux-to-Windows handoff gate.
4. Headphone routing works after cold boot and resume without an `alsamixer`
   toggle; every repair remains explicit and fail-closed.
5. Personal and representative factory profiles select the requested variant,
   apply with verified readback, survive restart, and visibly report whether
   software EQ is active.
6. Software EQ passes representative 44.1, 48, and 96 kHz response checks,
   the completed two-hour stability gate, and exact cleanup.
7. A fresh RPM install/upgrade launches from the desktop, the focused
   software/hardware gates pass, and the user completes the release checklist.

### Full Version 1 goal

The full goal is complete when, in addition to the release candidate:

- every Version 1 row in `feature-parity.tsv` is verified, intentionally
  substituted with accepted evidence, or explicitly excluded as unsupported;
- every connected Version 1 output and input has a real-signal acceptance
  result;
- Windows/Linux response differences are measured and either meet the target
  or map to a named unsupported mechanism;
- required CA0132 fixes are reviewable against current upstream source, pass
  style/build/runtime gates, and have an upstream submission-ready history;
- installation, rollback, diagnostics, CI, handover, and release artifacts are
  reproducible from the public repository.

No completion claim may silently relabel a deferred Version 1 requirement as
optional. Hardware that is not connected or available remains an explicit
external dependency.

## Ordered milestones

The user selected volume/loudness parity as the active prerequisite before
software OutFX. M2's accepted shutdown-reset work moves to normal release
packaging; M3 is active.

### M0 — Consolidate repository and evidence

Status: **complete**

- Create this roadmap and correct stale current-state claims.
- Retarget the integration PR to `main`.
- Close superseded stacked PRs with a pointer to the integration PR.
- Merge the validated integration history, then use one short-lived branch and
  one PR per milestone.
- Stop duplicate push/PR CI runs and skip the full build/RPM matrix for
  Markdown-only changes.

Exit: `main` contains the current implementation, GitHub has no historical
stack left open, and the next change starts from `main`.

Evidence: PR #75 merged as `f08b3536dc5a0860d93349ffa197d334fab5d9ed`;
all Rust, RPM, and current ALSA `for-next` checks passed; open PR count became
zero before the next milestone branch was created.

### M1 — Persistent diagnostic trail

Status: **complete**

- Enable the existing structured GUI trace by default, with `AE5_TRACE=0` as
  an opt-out.
- Cover startup identity, route/profile requests, EQ activation/deactivation,
  checked mixer writes, recovery, and terminal errors.
- Include relevant current-boot application trace lines in the private
  diagnostics report.
- Never log audio, credentials, unrelated devices, profile contents, user
  names, or arbitrary local paths.

Use the user journal instead of inventing a daemon or unbounded log format.
The journal supplies timestamps, process identity, storage limits, and
rotation. Default-on tracing may return to opt-in after three clean daily-use
sessions covering a cold boot, resume, profile switch, and EQ switch.

Exit: one reproduction report reconstructs the operation sequence without
asking the user to remember it.

Evidence: the rootless installed GUI is byte-identical to the release build;
a native Wayland launch recorded application start, exact-card discovery,
window presentation, and refresh events in the user journal; the installed
diagnostics command included those bounded trace lines. The launch preserved
the matched route, 5% muted sink, OutFX-off state, and closed playback PCMs.

### M2 — Warm-handoff kernel acceptance

Status: **complete; stable-package promotion moved to M5**

- Completed: install the package-verified `7.1.4-ae5-shutdown` kernel side by
  side for one boot while retaining the stock saved/default entry.
- Completed: the exact candidate passed its guarded bare-metal runtime and
  EFI-pstore preparation gates, then a no-power-removal warm reboot into
  `7.1.4-ae5-stable` proved exactly one successful shutdown reset, no reset
  failure, one DSP initialization in each boot, and zero kernel taint.
- Completed: a second exact-candidate boot passed the preparation gate and
  warm-booted into bare-metal Windows without motherboard power removal. The
  user confirmed normal Windows playback, then the acknowledged Linux return
  gate proved one successful candidate shutdown reset, no failure, one DSP
  initialization after return, and zero kernel taint.
- Keep stock and `7.1.4-ae5-stable` as recoverable boot choices until accepted.

Exit: the ninth patch passes the Linux and Windows warm-handoff gates or is
rejected with captured evidence and a narrower follow-up task.

### M3 — Representative cross-rate EQ acceptance

Status: **in progress**

- Completed prerequisite: matching Windows symbols and disassembly recovered
  the exact `-96..0 dB`, exponent-`1.75` endpoint taper. A tested PipeWire SPA
  overlay applies it only to the exact `11020051` AE-5 analog node while
  preserving ordinary desktop percentages; non-AE-5 nodes remain cubic.
- Run the guarded physical Windows/Linux loudness comparison at the same
  user-selected listening level and record whether the formula closes the
  reported level difference.
- Prove the active PipeWire graph and ALSA PCM rates at 44.1, 48, and 96 kHz.
- Measure neutral repeatability and three curves: the personal headphone
  profile plus two materially different factory profiles.
- Require at most 1 dB model error, zero relevant warnings, unchanged sink
  identity, byte-identical mixer recovery, and closed PCMs.
- Do not run all 33 presets on hardware. Their shared graph generator belongs
  in software tests; physical testing samples boundaries and distinct shapes.

Exit: one evidence matrix closes the rate/preset gate. Rerun it only after EQ,
PipeWire policy, kernel audio-path, or rate-negotiation changes.

### M4 — Profile and Qt/QML GUI daily-use acceptance

Status: **in progress — hardware Effects transaction complete**

- Reproduce and fix the reported profile-card fallback to Adventure and Action.
- Make profile application state explicit: selected profile, route variant,
  software-EQ runtime status, checked readback, and failure/rollback.
- Run the installed native Wayland UI through personal/factory profile
  switching, output selector, restart persistence, keyboard access, and the
  diagnostics action.
- Keep unsupported controls disabled and explained.
- The user has confirmed that the core audio MVP is sufficient to begin the
  Qt/QML redesign. Implement the selected **Section-Owned Profiles with
  Hardware Faceplate** direction from
  [`docs/QT_QML_SELECTED_DESIGN_SPEC.md`](docs/QT_QML_SELECTED_DESIGN_SPEC.md)
  incrementally while retaining the GTK application as a temporary fallback.
- Keep live device output separate from independently selectable and savable
  Effects profiles and EQ presets. There is no global profile Save action.
- Completed Phase 2: the optional `ae5-control-qml` target now provides the
  selected responsive shell, semantic theme, separate object ownership,
  Shape-based ten-band preview, persistent faceplate, and native
  Wayland/X11 smoke coverage. It is explicitly labelled as a preview and makes
  no ALSA or PipeWire writes.
- Completed Phase 3 read-only slice: the separate `ae5d` user service exposes
  a typed session D-Bus state contract, and the Qt faceplate now reads exact
  device, format, output, gain, volume, mute, capability, and write-block
  reasons. The UI enters a precise daemon-unavailable state and recovers on its
  five-second refresh without an application restart.
- Completed Phase 3 safe-write slice: volume and mute now use narrowly scoped
  typed D-Bus methods, exact AE-5 PipeWire targeting, readback, rollback, and
  structured daemon-journal events. Native Wayland tests passed 20% → 19% →
  20% and mute → unmute through the actual QML controls.
- Completed Phase 4 profile-object slice: `ae5d` exposes the 33 embedded Command
  profiles and route-compatible personal imports as separate typed Effects
  profiles and EQ presets. The QML selectors load the real independent
  objects, default to `My profile` and `SHP Last` when present, and update the
  Effects values or ten-band curve without changing live audio. Independent
  Rust-owned drafts now support editing, Revert, Save, and Save as through
  typed D-Bus methods. Combined imports and factory objects remain read-only
  and are copied into section-only files on Save as. Atomic persistence,
  strict validation, duplicate-name refusal, hidden section-control
  preservation, catalog refresh, restart discovery, and independent unsaved
  counts were verified against an isolated copy of the personal library.
- Completed Phase 5 checked-transaction slice: the ten-band QML draft now has
  typed Apply and Disable actions backed by `ae5d`. Saved preset state remains
  separate from live software-EQ state. The daemon validates the draft and
  exact output, blocks Direct Mode conflicts before writes, verifies graph
  ownership and PipeWire markers, omits the retired fixed EQ preamp, migrates
  managed v1 graphs, and restores the prior managed config and runtime graph
  on failure. OutFX and software EQ
  are now permitted together, matching the recovered Windows processing
  groups; native Wayland and X11 launches showed no QML errors.
- Completed the first Phase 7 acceptance slice: Qt Quick Controls Basic now
  uses dark and light semantic palettes; custom controls have keyboard-only
  focus rings; disabled controls explain their capability block; scoped
  `Ctrl+S`, `Ctrl+Shift+S`, and faceplate Review actions preserve independent
  Effects/EQ ownership; and AT-SPI exposes meaningful roles, names,
  descriptions, values, and live status. Native Wayland checks passed at
  1024 × 680, 1280 × 800, and 1600 × 1000 without horizontal overflow, and an
  X11 smoke launch produced no QML error. Object-scoped close-with-unsaved
  choices and 125%, 150%, and 200% Wayland scale smokes also pass. Automated
  focus-order assertions and complete injected failure-state tests remain
  open.
- Completed the first Phase 8 packaging slice: both the RPM and transactional
  rootless installer now ship `ae5-control-qml`, `ae5d`, D-Bus activation, and
  the systemd user unit while retaining GTK as a fallback. The installed
  desktop entry launches Qt. An already-running session bus reloads the new
  activation metadata immediately. The isolated rootless lifecycle, a clean
  Fedora 44 RPM install/remove transaction, native Wayland and X11 startup,
  and a live daemon stop/reactivation cycle passed without an audio write;
  the physical sink remained at 20%.
- Completed the desktop lifecycle slice: RPM and rootless packages autostart
  the Qt application hidden when a tray is available; closing hides the window,
  the tray provides Open/Hide and Quit, unsupported tray environments fall
  back to a visible window, and object-scoped unsaved handling remains active
  for both hide and quit. Automated QML and installer lifecycle checks cover
  the new paths.
- Completed the selected Sound-screen installed-host acceptance at commit
  `b068ff9`: the transactional user install upgraded without changing any of
  the seven existing configuration/profile files; installed binaries matched
  the rebuilt release; all ten deterministic states passed under native
  Wayland and X11/XWayland; focus audits passed at 1024×680, 1280×800,
  1600×1000, and 200% scaling; and the live physical-card Wayland screen
  exposed the expected AT-SPI semantics. The sink remained at 20%, `ae5d`
  logged no write, and no writable ALSA control changed. The exact green RPM
  is retained under `dist/qt-qml-b068ff9/`; authenticated host system-RPM
  installation remains a distinct M5 gate.
- Completed the Phase 8 multi-page shell slice: Overview, Equalizer, Playback,
  Recording, Mixer, Lighting, Device, and Settings now share functional
  navigation, semantic components, the Rust-owned EQ object, and the single
  persistent hardware faceplate. Unsupported typed writes are visible as
  read-only, deferred, or guarded. Native Wayland visual checks passed for all
  nine destinations at 1280 × 800, representative dense pages at 1024 × 680,
  and the Sound page in both dark and light themes. This closes navigation and
  layout only; typed backend integration and physical acceptance for deferred
  controls remain open.
- Completed the guarded hardware Effects slice on
  `7.1.4-ae5-outfx-lab`: `ae5d` applies the complete profile with active-stream
  parking, master-last ordering, exact ALSA readback, managed-state
  persistence, and rollback while paused. Software EQ remained active during
  a silent physical-card apply; the exact PipeWire stream survived, no
  transition sink leaked, disable/reapply verified, and kernel taint stayed
  zero. The UI now reports the confirmed hardware state and cannot stack the
  software-Effects fallback with OutFX.
- Active next step: run a user-controlled audible profile A/B, then verify the
  same profile and EQ state after daemon restart and a cold boot. Keep output,
  and Direct Mode writes unavailable until their own transactions are
  implemented. Headphone gain now has a route-matched checked transaction,
  exact readback/rollback, and explicit High-gain confirmation. Do not spend
  the core-validation milestone on GUI performance or visual redesign.
- Keep hardware, profiles, state transitions, readback, rollback, and
  diagnostics outside toolkit-specific UI code so the backend can be reused by
  the future `ae5d`/CXX-Qt/QML architecture documented in
  `docs/FUTURE_QT_QML_REDESIGN_PROMPT.md`.

Exit: the user can tell what applied without opening a terminal, and a failed
apply leaves the prior state intact.

### M5 — Release packaging

Status: **pending**

- Build and install the current RPM on the host.
- Promote the accepted ninth shutdown-reset patch into the daily stable
  package without the candidate-only EFI-pstore arguments.
- Verify upgrade, desktop launch, exact device detection, profile persistence,
  diagnostics, and removal/rollback behavior.
- Refresh README, handover, screenshots, known limitations, and recovery
  instructions.
- Tag a release candidate and publish its hashes.

Exit: the public repository alone is sufficient to install, test, recover,
and continue development.

### M6 — Remaining Version 1 hardware acceptance

Status: **pending; equipment-dependent**

Batch tests by physical setup rather than interrupting every software
milestone:

- cold-boot and suspend/resume lifecycle totals;
- connected speaker layouts and line-out;
- optical output with a receiver;
- rear/front microphone and line-in;
- attenuated Windows/Linux analog response and noise;
- visible onboard RGB confirmation.

Direct Mode, external-strip lighting, and open replacements for the remaining
Acoustic Engine effects stay separate after the daily-use release candidate
unless a missing one blocks the user's normal use.

## Testing policy

Testing must answer a decision; activity alone is not progress.

| Change scope | Required before commit | Physical rerun |
|---|---|---|
| Documentation only | link/claim review, `git diff --check` | none |
| Pure Rust logic | focused unit test, format, Clippy | none unless hardware semantics changed |
| GUI behavior | focused tests and native Wayland smoke | only the changed control path |
| Profile/EQ graph | focused software tests and self-test | representative response matrix at milestone exit |
| PipeWire routing | parser/unit checks and dry run | exact changed route/lifecycle gate |
| Kernel patch | apply/style/object build | exact affected hardware gate, once per candidate |
| Release | complete CI and package lifecycle | release checklist only |

A previously accepted gate remains valid when its implementation, dependency,
kernel path, and test assumptions are unchanged. A failure invalidates the
smallest related evidence set, not every result in the repository.

Every physical harness must:

- preserve the user's volume/mute state, hard-mute silent transactions, and
  avoid High gain for an acoustic test;
- fail closed when identity or state is ambiguous;
- capture before/after state and relevant journals;
- restore mixer, route, volume/mute, graph, and PCM state;
- write one machine-readable pass/fail summary with the tested commit and
  kernel.

## Pull-request policy

- No stacked PR chains.
- At most one integration PR is open.
- A PR represents one roadmap milestone or one root-cause fix.
- Merge when its declared exit gate and CI pass; do not keep completed work
  open to collect unrelated follow-ups.
- New discoveries become a roadmap item or issue, not an expansion of the
  current PR unless they invalidate its safety or correctness.

## Schedule estimate

The consolidation and diagnostic milestones are software-only and should fit
in one focused development session. A daily-use release candidate is
realistically **two to four focused sessions plus one user-assisted reboot and
warm Windows handoff**.

The full Version 1 date is dependency-bound rather than code-bound. With the
listed speakers, receiver, inputs, capture cabling, and reboot sessions
available, the remaining work is roughly **one to three weeks of focused
development and measurement**. Without that equipment, the project can ship a
guarded headphone-focused release candidate, but the full hardware claims
must remain open.
