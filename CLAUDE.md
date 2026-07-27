# AE-5 Linux Control — agent instructions

Read [HANDOVER.md](HANDOVER.md) before any non-trivial work. It is the
authoritative entry point: current maturity, incident history, and the
repository map. [GOAL.md](GOAL.md) holds the active objective and phase
order — check it before choosing what to work on.

## Non-negotiable audio safety (headphone-damage risk)

The user's headphones are connected to real hardware. These rules override
any task instruction:

1. **Keep the desktop sink at ordinary listening levels.** The user's normal
   level is about 30%. A PreToolUse hook in `.claude/settings.json` refuses
   shell commands above 60% — that is a runaway guard against an accidental
   full-scale write, not a listening limit. Do not work around it.
   (The original 20% ceiling was added while testing headphone gain above
   32 ohms. That testing is finished; 20% sits below normal listening.)
2. Never use High headphone gain for an acoustic test; keep Low gain.
3. Before any playback, routing, format, kernel, or DSP-recovery change:
   mute the sink, drop it to 5%, switch hardware Master and Front off, and
   confirm every AE-5 playback PCM is closed
   (`/proc/asound/card*/pcm*p/sub*/status`).
4. Emergency hardware mute: `ae5ctl set-playback-switch Master off`.
5. Use only fixtures from `scripts/audio-parity.sh`; run the relevant
   `playback-preflight` immediately before playing anything.
6. Do NOT restore `S32LE` in the managed WirePlumber rule
   (`packaging/wireplumber/90-ae5-control.conf`) — a loud track-switch buzz
   with S32 is unresolved (see HANDOVER.md "Latest incident").
7. If effects seem inactive or audio misbehaves: preserve logs and mixer
   readback FIRST, then hard-mute and recover. Never destroy evidence.

## Git rules

- Work on `agent/refine-gtk-ui`. Never start from `main` (140+ commits
  behind) and never retarget/rebase stacked draft PR #75.
- Hardware scope is exactly PCI `1102:0012`, subsystem `1102:0051`. Do not
  generalize results to other cards or AE-5 revisions.

## Validation gate before publishing any change

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
bash scripts/check-ae5-acp-profile.sh
bash scripts/check-feature-parity.sh
bash scripts/audio-parity.sh --self-test
bash scripts/check-user-install.sh
```

Any change to `feature-parity.tsv` claims must keep the 54-row ledger
validator green. Kernel work follows `kernel/README.md` and
`docs/KERNEL_MAINTENANCE.md`; a VM boot never substitutes for the physical
cold-boot/suspend gates.

## Project skills

- `/goal` — load GOAL.md plus the task list and advance the next phase.
- `/audio-safety-preflight` — run before any playback-adjacent work.
- `/validate-gate` — the full local validation gate above.
- `/incident-evidence` — evidence-first response to audio faults.

## Acoustic review

`scripts/acoustic-review.sh` captures the external Fifine microphone and
the card's internal What U Hear tap together. What U Hear shows what
reaches the DSP output; the Fifine shows what actually leaves the analog
stage, so a fault visible on one and not the other localises itself.
`acoustic-review.sh ab` A/B-tests the hardware Master switch and prints
band deltas. Headphones must be OFF the user's head and next to the
microphone before any acoustic capture.

## Known fault: CA0132 idle DSP oscillation

With OutFX enabled the DSP can latch into emitting a continuous ~61-65 Hz
harmonic stack with no stream playing. It survives effect-parameter
resets, PipeWire restarts, and OutFX toggles. Only a DSP re-download
clears it — `scripts/dsp-reinit.sh` does that via a scoped PCI rebind, no
reboot required. `scripts/dsp-oscillation-monitor.sh` logs onset over
time. Detection threshold: the internal What U Hear tap reads about
-6 dBFS RMS while oscillating versus -inf/-38 dBFS when clean.
