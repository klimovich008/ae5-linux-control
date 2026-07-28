# AE-5 Linux Control — development history

This records the investigation and implementation history. The authoritative
forward execution order and completion criteria are now in
[ROADMAP.md](ROADMAP.md). [HANDOVER.md](HANDOVER.md) records current machine
state, while [PORT_PLAN.md](PORT_PLAN.md) preserves the original scope.

Created: 2026-07-27 · Owner: maks · Historical work branch:
`agent/refine-gtk-ui` · Integrated into `main` by PR #75

---

## Mission

Make the AE-5 fully usable on Linux — safe by construction, honest about
what it cannot do, and **better to operate than Sound Blaster Command**,
not merely a copy of it.

Success means Maks uses this daily without thinking about it, and a new
maintainer can pick it up from the repository alone.

## Invariants (never traded away for progress)

1. Every project-controlled playback test stays at or below 20%. Start at 5%,
   use Low headphone gain for acoustic tests, and keep the output physically
   away from the user's ears for the first playback after a transition.
2. Evidence before recovery. A reproducible fault is worth more than a
   fast fix — see `/incident-evidence`.
3. No fake controls. If a feature has no safe Linux mechanism, the UI says
   so plainly rather than presenting a dead widget.
4. `S32LE` stays out of the managed WirePlumber rule until Phase 2 closes.
5. Hardware claims require physical evidence. A VM boot, module compile, or
   zero-amplitude stream never satisfies a physical gate.

---

## Phase 0 — Safety net · **done**

Project-scoped `CLAUDE.md`, the volume-guard PreToolUse hook, and the
`/audio-safety-preflight`, `/validate-gate`, `/incident-evidence` skills
are installed. Future sessions inherit the rules automatically and cannot
write a runaway volume from a shell.

## Phase 1 — Restore working audio on this host · **done**

The card-specific ACP route and explicit route-repair transaction fixed the
hidden `Front` mute. Guarded physical tests confirmed audible desktop playback
on the stable S16 path and clean PCM closure. The current headphones are
connected to the motherboard line-out, so the present lack of an AE-5 listener
is a physical topology choice rather than a recurrence of that route fault.

## Phase 2 — Root-cause the loud-buzz fault · **resolved by later checkpoints**

The single most important item. A real music track change under S32
produced a loud buzz that desktop mute could not stop, ending only when
the PCM suspended. Because the card runs `api.alsa.soft-mixer=true`,
desktop mute only zeroes the *sample stream* — so output that survived it
was **not** coming from the samples. That points at DMA position/stream
teardown or stale CA0132 DSP state, not at bad audio data, and it is why
the 60-stream synthetic recreation missed it: short zero-amplitude streams
never reproduce a real client's transition.

- Build a transition-stress harness that reproduces *real* track-change
  semantics: client disconnect/reconnect, mid-stream rate and format
  renegotiation, gapless hand-off, and suspend races — not just sequential
  short streams.
- Instrument to distinguish the four candidate causes: stream teardown,
  HDA DMA/link position, PipeWire suspend timing, and stale DSP state.
  Log all four per transition with timestamps.
- Run entirely with `Master` and `Front` hard-muted; the harness must be
  inaudible by construction, never by intention.
- Add a fail-closed watchdog that hard-mutes on anomaly before any future
  audible test.

**Done when:** either a reproducible trigger with a fix, or bounded
evidence that a specific guard makes S32 safe. Only then does S32LE
return — worth pursuing because S32 matched direct DSP response within
0.01 dB at 20% where S16 was off by up to 5.81 dB.

### 2026-07-27 instrumentation checkpoint

[`scripts/track-transition-stress.sh`](scripts/track-transition-stress.sh)
now implements exact-target, five-trial-or-more close/reopen, abrupt
disconnect, client rate/format replacement, a client-owned in-place
renegotiation probe, gapless overlap, and suspend-boundary cases. It hard-mutes
`Master` and `Front`, continuously watches both switches, uses bounded
generated fixtures, records PCM, PipeWire, client, mixer, and journal
evidence, and never enables S32 itself.

The in-place helper uses PipeWire's native `pw_stream_update_params()` path
plus an explicit paused `Format` update and emits only digital silence. Its
`--target 0` graph validation observed S16/44.1, S32/48, S32/96, and S16/48 on
one node with zero links while both AE-5 playback PCMs remained closed.
Linked tests against S16/48 and S32/96 virtual null sinks each completed all
four updates with five negotiated callbacks. A sampled run retained one node
serial and the same two link serials throughout. The exact-target hardware
stress case remains unrun and this virtual result is not evidence that S32 is
safe on the AE-5.

[`scripts/hda-position-trace.sh`](scripts/hda-position-trace.sh) consumes the
upstream `hda_controller:azx_pcm_*` and `azx_get_position` tracepoints without
a kernel patch. Tracefs is root-only and this account has no authenticated
`sudo`, so the complete HDA-position capture is implemented but not yet run.

No transition playback was run at this checkpoint. The real sink remains S16,
the user's 30% state was not changed, both playback PCMs stayed closed, and
the headphones remain on the motherboard output. Full method and evidence
interpretation:
[`docs/TRACK_TRANSITION_INVESTIGATION.md`](docs/TRACK_TRANSITION_INVESTIGATION.md).

### 2026-07-27 breakthrough — idle DSP self-oscillation

A reproducible buzz was captured and characterised on the host. It is
**not** playback-derived:

- Present with the sink muted, every playback PCM closed, and no stream.
- Gated by two switches: hardware `Master`, and global `Enable OutFX`.
- A/B/A confirmed. Acoustic (Fifine): OutFX on -55.7 dB, off -70.0 dB,
  on -55.7 dB — the off case equals the room noise floor exactly.
  Internal What U Hear tap: on -2.80 dB RMS, off -38.33 dB, on -2.80 dB.
- Spectrum is a harmonic stack — fundamental ~61-65 Hz with harmonics at
  ~130, ~195, ~260 Hz — and **zero** increase above 3 kHz, which rules
  out broadband amplifier hiss.
- Scales with the analog `Front` control, so it originates upstream of
  the analog volume, in the DSP domain.
- Not attributable to any single effect: X-Bass, Crystalizer, Smart
  Volume, and Dialog Plus were each disabled individually without
  removing it. Only the global OutFX switch clears it.

This plausibly explains the original incident: a DSP-generated signal
independent of the sample stream is exactly why desktop (software) mute
could not stop the buzz, since `soft-mixer=true` only zeroes samples.

Reproduce with `scripts/acoustic-review.sh ab`.

### What the fault survives

Follow-up on the same day, after the user confirmed the buzz is
**Linux-only** and **appears only after some hours of uptime** rather
than immediately from boot:

| Attempted reset | Result |
|---|---|
| `Enable OutFX` off → on | returns instantly |
| `linux-defaults-apply` (29 DSP controls) | no change |
| `wireplumber` + `pipewire` restart | no change |
| Active stream instead of idle | weakens to ~-17 dB, never clears |
| Increasing idle gaps (0-60 s) | flat, no growth — state already latched |

So it is not an effect-parameter value, not a PipeWire-side condition,
and not a gradual drift. It behaves like corrupted DSP internal state
that only a DSP re-download clears.

Runtime power management is **excluded** as the trigger: although
`snd_hda_intel power_save=10` is set, the card's
`power/runtime_suspended_time` is `0`, so it has never runtime-suspended.
The kernel log shows `ca0132 DSP downloaded and running` once at boot
with no reinit or error since.

### Confirmed: corrupted DSP state, cleared by re-download

A PCI unbind/rebind of the audited card forces the driver to re-download
the DSP image. `modprobe -r snd_hda_codec_ca0132` is not usable because
the codec binding holds a reference while `snd_hda_intel` serves other
cards; rebinding only `0000:29:00.0` is both sufficient and narrower.

Measured across the reinit, with the configuration held identical
(OutFX on, Master on, idle, no stream):

| | What U Hear RMS | Fifine acoustic RMS |
|---|---|---|
| before reinit | -6 dB | -55.7 dB |
| after reinit | **-inf (exact digital silence)** | **-70.4 dB (room floor)** |

The user's full 21-control profile was then reapplied with effects
enabled and the output stayed at exact digital silence. That settles it:
the oscillation is corrupted CA0132 DSP internal state, not a
configuration value, and **effects do not have to be sacrificed**.

Recovery without a reboot is now `scripts/dsp-reinit.sh`, scoped to PCI
`1102:0012` / subsystem `1102:0051`, hard-muting throughout and refusing
any other device.

### Trigger: playback, probabilistically

Measured with a DSP reinit before every trial, so each run starts from
exact digital silence:

| Configuration | Trials | Oscillated after playing a tone fixture |
|---|---:|---:|
| user profile, effects on | 5 | **4** |
| all effects off, OutFX on | 1 | 0 |
| one effect on at driver defaults | 6 | 0 |
| silent (all-zero) stream, effects on | 1 | 0 |
| OutFX toggled, no playback at all | 1 | 0 |

So real audio through the DSP is what excites it, and it stays latched
after the stream closes. A digital-silence stream never triggers it,
which is the behaviour of an unstable filter that cannot be excited by a
zero input. The oscillation frequency also moves between episodes
(~41-47 Hz and ~61-65 Hz observed), consistent with filter state that
depends on what was last processed rather than a fixed tone.

The rate is roughly 80%, **not deterministic**. Any single-trial result
is therefore near-worthless: the seven clean single-trial runs above are
only weak evidence individually, though seven consecutive clean runs
would be very unlikely at an 80% rate, so effect *values* — not merely
having effects enabled — do appear to matter. Repeat every future
bisection at n>=5 before drawing a conclusion.

### Observability (added 2026-07-27, late session)

A code-and-log review found why every fault in this project has been hard
to catch, and instrumented all three layers:

- **The application logged nothing.** `AE5_TRACE=1` now emits a monotonic
  stderr trace of every mixer write with its readback, every ALSA event
  with its classification, and every window rebuild with its reason.
- **The GUI rebuilt itself for its own writes.** The mixer watch refreshed
  on every ALSA event, including echoes of the application's own writes —
  one full-window rebuild per click and per slider step (the reported
  "blink"), and a rebuild racing every route switch. Self-originated
  events (within 400 ms of our own write) are now suppressed and logged;
  the editors already display verified readback, so only external changes
  rebuild.
- **The driver had 121 debug sites, all off.** `scripts/ca0132-debug.sh`
  toggles `snd_hda_codec_ca0132` dynamic debug at runtime (dspio commands,
  DSP transfers), streams a filtered kernel log, and collects a full
  evidence snapshot (kernel tail, mixer readback, codec dump, PCM state,
  PipeWire graph). `dsp-oscillation-monitor.sh` now snapshots
  automatically at onset, so the trigger is caught while it is still in
  the kernel log's tail.

Two facts from the log review worth keeping: the card runs **non-snoop
DMA** (`Force to non-snoop mode` on every bind) — hardware does not keep
CPU caches coherent with DMA, which is a plausible mechanism for
*probabilistic* DSP-state corruption and fits the ~80% trigger rate; and
WirePlumber logs `wp_properties_get: assertion 'self != NULL'` during our
session teardowns, worth watching around route faults.

### Why it is Linux-only — answered 2026-07-27

Examining the vendor stack on this host's Windows partition settled the
question. **Windows does not run the SBX effects on the CA0132 DSP.** The
driver registers `CtxRFX64.dll`, "Creative Render Audio Effects", as a
software Audio Processing Object on every render endpoint, and its symbol
table names the effect implementations — Crystalizer, Smart Volume,
Surround, Bass Boost, bass management. Those sliders are APO parameters
computed on the CPU, not DSP register writes.

Linux does the opposite: `snd_hda_codec_ca0132` programs the card's DSP
through `dspio` commands. So the hardware effect path we drive is one the
vendor's own stack does not exercise for these effects, which is
consistent with meeting an instability no Windows user reports.

Full findings, method and limits: [`docs/WINDOWS_STACK_ARCHITECTURE.md`](docs/WINDOWS_STACK_ARCHITECTURE.md).

This opens a third option beyond "fix the DSP path" and "live with the
reinit": implement the effects as a PipeWire filter chain — the direct
analogue of an APO — and leave the unstable hardware path alone. It trades
hardware offload for stability, needs its own DSP design and measurement
work, and is the architecture the vendor themselves chose.

### Software-EQ performance checkpoint — 2026-07-28

The in-place ten-biquad graph passed its exact-sink two-hour qualification.
It retained the same PipeWire node and 2048-frame/48 kHz quantum, added no
PipeWire buffer frame, and added 0.3990 percentage points of process CPU.
Filter work increased by 178.564 µs, 0.4185% of the 42.667 ms quantum. The
7200-second nonzero soak recorded 7197 zero-error samples, 200.430 µs mean and
267.900 µs maximum sink work, 1.1060% PipeWire CPU, and no relevant journal
warning. Cleanup restored the byte-identical mixer, 5% muted sink and matched
routes, removed the graph/state file, kept OutFX off, and closed both PCMs.

### Kernel A/B — answered 2026-07-27, patches exonerated

Measured on the **stock** Nobara kernel `7.1.4-200.nobara.fc44.x86_64`
(taint 0), idle, all playback PCMs closed, no audio played since boot,
Low headphone gain:

| `Enable OutFX` | What U Hear RMS |
|---|---|
| on | -3.60 dB |
| off | **-36.31 dB** |
| on | -5.05 dB |

The same A/B/A on the patched kernel read -2.80 / -38.33 / -2.80 dB. The
behaviour is identical, so **the project's patch queue is not the cause**.
This is upstream `snd_hda_codec_ca0132` driving the hardware DSP, or the
silicon itself. It also removes the last reason to suspect
`ca0132-ae5-direct-mode.patch`, which was the only patch touching the
`PLAY_ENHANCEMENT` path.

Two things follow. The oscillation is a genuine upstream/hardware issue
worth reporting once characterised, not a local regression. And the
bypass premise the software-effects plan rests on holds on a stock
kernel: with OutFX off the tap sits at the floor.

### PCM-reopen corruption — fixed 2026-07-27

Waveform-qualified VFIO tests separated a second failure from the OutFX
oscillation. Generic CA0132 cleanup cleared the AE-5 HDA playback converter on
PCM close, and reassigning it could produce approximately 26.4% THD. HDA
runtime autosuspend also cleared a retained assignment after idle.

The eighth production patch retains the converter across AE-5 PCM close and
holds a balanced AE-5 codec runtime-PM reference. It passed 50/50 clean
reopens after a fresh host-driver-to-VFIO cycle, plus warm, repeated-idle,
48/96 kHz, 2/6-channel, and rejected-OutFX matrices. The exact packaged kernel
also passed a fresh passthrough boot, first-open capture, warm/idle reopen
matrix, and exact rejected-OutFX matrix. The scheduled physical-host
cold-start and suspend gates remain; S32 does not return merely because this
one failure is fixed.

### Still open

- Which parameter values push the chain unstable, at n>=5 per cell.
- The DSP initialisation sequence `CtxHda.sys` performs, which would
  require disassembly and is the natural next question.
- Whether the original loud track-switch fault is a louder instance of
  this: the mechanism matches, since a DSP-generated signal is exactly
  what survives a software mute under `soft-mixer`.
- Whether the original loud track-switch fault is this same oscillation
  reaching the analog stage during a gain or route transition. The
  signature fits: a DSP-generated signal is exactly what survives a
  software mute under `soft-mixer`.
- Whether the driver should re-download the DSP on a detected anomaly,
  and whether an upstream CA0132 fix is warranted once the trigger is
  known.

## Phase 3 — Stale-DSP detection

The same incident silently desynchronized live effect controls from the
selected profile, and nothing detected it.

- Surface the existing 21-control profile-vs-readback comparison as a
  continuous health check in both CLI and GUI.
- Warn with evidence; never silently "repair" — offer explicit recovery
  (hard-mute → OutFX cycle → reapply profile) as a user-confirmed action.

## Phase 4 — GUI and UX

Sound Blaster Command is the *reference*, not the ceiling. Match it where
it is genuinely good (recognizable information architecture, profile
cards, ten-band EQ) and beat it where it is weak. Command was designed for
a Windows desktop in 2018; it hides real state, has no concept of a safety
limit, and makes you visit four pages to answer "what is my card doing
right now?".

### Where this app should be *better* than Command

1. **Honest live state.** Every control already reflects checked ALSA
   readback. Make that visible and constant — the user should never have
   to guess whether a setting actually took effect. This is the single
   biggest advantage over Command and the direct answer to "I clicked the
   profile but I'm not sure if it applied".
2. **Safety as a first-class feature.** An optional, persistent output
   limiter and a clear gain-staging display (PipeWire % versus each
   hardware dB stage). Command offers nothing here, and it is the feature
   this user actually needs.
3. **A status home.** One view answering: active profile, route, gain
   stage, effects on/off, DSP health, kernel capability. Today that
   requires visiting several pages.
4. **Health surfaced, not buried.** Route health, hidden-mute split, and
   stale-DSP drift belong in the UI with one-click guarded repair.
5. **Task-shaped flows.** Optimize the common case ("switch to headphones
   for gaming") over feature-shaped navigation.
6. **Honest unsupported states.** Keep marking what Linux cannot do and
   why, with the substituted native mechanism named.

### Structural work this depends on

`src/bin/ae5-control.rs` was a single **5,398-line** file containing all
nine pages, both settings tabs, and the shared design system. It is being
split into modules under `src/gui/`, feature-gated behind `gui`.
Refactor-only — no behaviour change, gate green after every step.

Done so far (`dd3cd8d`, `b7db3f3`) — binary now **4,059 lines**, with
1,396 lines extracted:

| Module | Lines | Contents |
|---|---:|---|
| `gui::theme` | 453 | the entire stylesheet |
| `gui::editors` | 728 | generic ALSA control editors, `Category` |
| `gui::pages::compatibility` | 128 | + its ledger test |
| `gui::pages::scout` | 54 | |
| `gui::widgets` | 27 | shared card primitive |

The extraction recipe, in order: move the function range to the new
module; rewrite `ae5_control::` to `crate::` inside the library; in the
binary add `use ae5_control::gui::<module>::*;` rather than rewriting
every call site; move any test that covers the moved code. Three traps
worth knowing: `#[derive(..)]` attributes sit *above* the item, so slice
from the attribute; a constant used only by moved code must be deleted
from the binary or Clippy fails it as dead; and a private method on a
moved type needs `pub` wherever the shell still calls it.

Remaining pages, largest first: `sound_effects` (233), `device` (213),
`profile_page` (188) with `saved_profile_actions` (194) and
`builtin_profile_actions` (62), `lighting` (161), `analog_playback` (121)
with `playback_page`, `digital_playback` and `footer_output_selector`
(71), `recording` (117), `mixer` (102), `equalizer` (93), `settings` (41);
then the shell helpers (`content`, `populate_page`, `hero`, `status_rail`,
`sidebar_brand`, `error_view`, `routing_card`, `native_rates_card`,
`route_health_summary`, `effect_control_card`, `sound_profile_card`,
`kernel_readiness_summary`, `start_mixer_watch`).

### Windows reference material

The eight private Command page captures live at
`~/.local/share/ae5-control/windows-ui-reference/2026-07-26/`. They are
machine-local and stay out of Git. Use them for layout, proportion and
state, never for artwork: the profile-carousel illustrations are
Creative's copyrighted imagery, so the cards stay text-based.

A first visual pass against those captures landed in `dd3cd8d`: page
titles moved from 20px/weight-760 to 26px/weight-450, the sidebar widened
to 232px with 46px rows, the flat selection block became a translucent
cyan wash behind the existing stripe, the content area gained a subtle
violet gradient, and the tabs grew to 36px.

### Acceptance

- Native Wayland startup < 250 ms, refresh < 100 ms, idle CPU ~0%,
  peak idle RSS in the current ~75 MiB envelope.
- Every interactive control named in the accessibility tree; contrast
  ratios stay at or above the current 5.91:1 floor.
- No control present that does not map to a real, checked mechanism.
- Full Rust/GTK/CLI test suite and strict Clippy stay green.

## Phase 5 — Physical acceptance · **needs Maks**

These cannot progress without hardware participation. Each is a short,
scripted session at an ordinary listening level with Low gain:

- Install and boot the package-validated AE-5 warm-shutdown DSP-reset
  candidate, then prove the reset in the previous-boot journal and repeat a
  Linux-to-Windows warm handoff. See
  [`docs/WARM_REBOOT_DSP_RESET.md`](docs/WARM_REBOOT_DSP_RESET.md).
- Ten cold boots and twenty bare-metal suspend/resume cycles — this also
  validates the CA0132 resume patch now that the custom kernel runs on the
  host.
- Connected speaker layouts 2.0 → 5.1, line-out, optical/IEC958 with a
  receiver, and analog inputs.
- Direct Mode on physical line-out.
- Matched, safely attenuated Windows/Linux response and noise capture.
- Visible on-card RGB confirmation from a GUI-selected colour.

Closing these converts most of the 18 deferred ledger rows to verified.

## Phase 6 — Merge and release

Land the stack so the repository homepage tells the truth: land PR #75 and
its parent, bring `main` forward from 140+ commits behind, and confirm the
handover is sufficient without external context.

---

## Execution rules

- Work phases in order. Phase 2 outranks Phase 4 — do not redesign the GUI
  while the loud-buzz path is unexplained. Phase 4 structural refactor may
  proceed in parallel *only* when Phase 2 is blocked awaiting hardware.
- Run `/audio-safety-preflight` before anything playback-adjacent and
  `/validate-gate` before any commit.
- Anything needing Maks physically present gets batched into one clear
  request with exact steps, not dribbled out one test at a time.
- Report honestly: a partially passing gate is a failing gate.
