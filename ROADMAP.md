# AE-5 Linux Control roadmap

This is the authoritative execution order and completion ledger. It turns the
original scope in [PORT_PLAN.md](PORT_PLAN.md), the incident history in
[GOAL.md](GOAL.md), and the current-state snapshot in
[HANDOVER.md](HANDOVER.md) into one sequence with explicit exit criteria.

Last audited: **2026-07-29**

## Current state

The project already provides a guarded, hardware-specific MVP:

- a Rust CLI and native GTK 4/Wayland application;
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

Only the first unfinished milestone is active.

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

Status: **in progress**

- Completed: install the package-verified `7.1.4-ae5-shutdown` kernel side by
  side for one boot while retaining the stock saved/default entry.
- Completed: the exact candidate passed its guarded bare-metal runtime and
  EFI-pstore preparation gates, then a no-power-removal warm reboot into
  `7.1.4-ae5-stable` proved exactly one successful shutdown reset, no reset
  failure, one DSP initialization in each boot, and zero kernel taint.
- Warm-boot into Windows and compare against a full power-removal baseline.
- Before the handoff, require the candidate-only EFI pstore preparation gate.
  On return to Linux, run the explicitly acknowledged `--check` gate before
  any sound module reload so the shutdown reset and current DSP download
  remain unambiguous.
- Keep stock and `7.1.4-ae5-stable` as recoverable boot choices until accepted.

Exit: the ninth patch either passes and replaces the eight-patch build, or is
rejected with captured evidence and a narrower follow-up task.

### M3 — Representative cross-rate EQ acceptance

Status: **pending**

- Prove the active PipeWire graph and ALSA PCM rates at 44.1, 48, and 96 kHz.
- Measure neutral repeatability and three curves: the personal headphone
  profile plus two materially different factory profiles.
- Require at most 1 dB model error, zero relevant warnings, unchanged sink
  identity, byte-identical mixer recovery, and closed PCMs.
- Do not run all 33 presets on hardware. Their shared graph generator belongs
  in software tests; physical testing samples boundaries and distinct shapes.

Exit: one evidence matrix closes the rate/preset gate. Rerun it only after EQ,
PipeWire policy, kernel audio-path, or rate-negotiation changes.

### M4 — Profile and GUI daily-use acceptance

Status: **pending**

- Reproduce and fix the reported profile-card fallback to Adventure and Action.
- Make profile application state explicit: selected profile, route variant,
  software-EQ runtime status, checked readback, and failure/rollback.
- Run the installed native Wayland UI through personal/factory profile
  switching, output selector, restart persistence, keyboard access, and the
  diagnostics action.
- Keep unsupported controls disabled and explained.
- Limit current-GUI work to functional and safety acceptance. Do not optimize
  its performance or start the Qt/QML visual redesign before the user confirms
  that the core audio behavior works as intended.
- Keep hardware, profiles, state transitions, readback, rollback, and
  diagnostics outside toolkit-specific UI code so the backend can be reused by
  the future `ae5d`/CXX-Qt/QML architecture documented in
  `docs/FUTURE_QT_QML_REDESIGN_PROMPT.md`.

Exit: the user can tell what applied without opening a terminal, and a failed
apply leaves the prior state intact.

### M5 — Release packaging

Status: **pending**

- Build and install the current RPM on the host.
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

- enforce the 20% ceiling and Low gain;
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
