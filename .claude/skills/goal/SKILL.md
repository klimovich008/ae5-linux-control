---
name: goal
description: Resume and advance the active AE-5 project goal. Use when asked to continue the goal, work on the next phase, check goal progress, or when picking up the project after a break.
---

# Advance the AE-5 goal

This project runs against a written objective. Follow it rather than
inventing new work.

## 1. Load the current position

- Read [GOAL.md](../../../GOAL.md) — mission, invariants, phase order, and
  the acceptance criteria for each phase.
- Read [HANDOVER.md](../../../HANDOVER.md) if the session is fresh — it
  records current state and incident history.
- Run `TaskList` to see phase status and dependencies. Tasks mirror the
  GOAL.md phases.

## 2. Choose what to work on

Work phases **in order**. Pick the lowest-numbered task that is `pending`
and not blocked. Two standing exceptions from GOAL.md:

- Phase 2 (loud-buzz root cause) outranks Phase 4 (GUI). Do not redesign
  the GUI while the buzz path is unexplained.
- Phase 4a (GUI module split) may proceed in parallel **only** when
  Phase 2 is blocked awaiting hardware.

If the next task is one that needs Maks physically present, do not stall
silently: batch the required steps into one clear request and say so.

## 3. Work the phase

- Mark the task `in_progress` before starting; keep it `in_progress` if
  you finish only part of it.
- Anything playback-adjacent: run `/audio-safety-preflight` first.
- Any audio fault: `/incident-evidence` before recovering.
- Acoustic checks: `scripts/acoustic-review.sh` (Fifine + internal tap).
  Headphones must be off Maks's head and next to the microphone.
- Before any commit: `/validate-gate`.

## 4. Respect the invariants

Never trade these for progress, whatever the task seems to need:

1. Ordinary listening levels (~30%), Low headphone gain. A hook refuses
   runaway writes above 60%.
2. Evidence before recovery.
3. No fake controls in the UI.
4. No `S32LE` in the managed WirePlumber rule until Phase 2 closes.
5. Hardware claims need physical evidence — a VM boot or a
   zero-amplitude stream never satisfies a physical gate.

## 5. Close the loop

- Update the task status honestly. A partially passing gate is a failing
  gate; say so with the output.
- Record durable findings in `docs/` and, when the phase picture changes,
  update `GOAL.md`.
- End by stating: what moved, what is now blocked, and the single next
  action.
