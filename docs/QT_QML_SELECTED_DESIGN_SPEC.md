# AE5 Control Qt/QML selected design specification

Status: **selected; Sound-screen and multi-page shell visual/state QA accepted**

Selected direction: **Section-Owned Profiles with Hardware Faceplate**

Visual target:
[ae5-control-sound-selected-v2.png](design/ae5-control-sound-selected-v2.png)

This document turns the selected Sound-screen image into an implementation
contract. It supplements, rather than replaces:

- [FUTURE_QT_QML_REDESIGN_PROMPT.md](FUTURE_QT_QML_REDESIGN_PROMPT.md) for
  technology and product-wide constraints;
- [UI_REDESIGN.md](UI_REDESIGN.md) for the UX problems being corrected;
- [WINDOWS_MIGRATION_VALIDATION.md](WINDOWS_MIGRATION_VALIDATION.md) for the
  real Windows profile and EQ data model;
- [ROADMAP.md](../ROADMAP.md) for execution order and release gates.

## Outcome

The final application should feel like a precise Linux audio instrument:
quiet during normal operation, explicit when state changes, and impossible to
confuse about what is live on the card versus what is saved in a named object.

The selected screen is not a clone of Sound Blaster Command. It keeps the
useful mental model while replacing Command's profile carousel, duplicated
state, large dials, ambiguous saves, and hidden hardware failures.

## Non-negotiable state model

The Sound screen exposes three independent objects.

| Object | Owns | Persistence | Never owns |
|---|---|---|---|
| **Device output** | card connection, active output, headphone gain, master volume, mute, format | live hardware/session state | Effects or EQ save state |
| **Effects profile** | Surround, Crystalizer, Bass, Smart Volume, Dialog+, supported OutFX state | named Effects profile | output route, gain, volume, EQ bands |
| **EQ preset** | ten EQ bands and supported EQ preset metadata | named EQ preset | output route, gain, volume, Effects values |

Rules:

1. The UI never uses `Profile` without a qualifier. It says
   `Effects profile` or `EQ preset`.
2. There is no global `Save`, `Revert`, or `Modified` state.
3. Effects and EQ each own their selector, status, save action, and menu.
4. Hardware changes apply immediately and are verified by readback.
5. Saving persists the selected Effects profile or EQ preset; it does not
   apply values to the card.
6. Master volume, mute, output, and headphone gain never mark either sound
   object as modified.
7. `Current setup` is a read-only summary, not a third persisted file.
8. An Effects profile may suggest an associated EQ preset. The suggestion
   never changes EQ automatically unless the user explicitly opts into that
   behavior in Settings.

## Default 1280 × 800 composition

The root has three structural regions.

| Region | Default size | Behavior |
|---|---:|---|
| Navigation sidebar | 208 px wide × 712 px high | persistent above the faceplate |
| Page workspace | remaining width × 712 px high | vertically scrollable only when required |
| Hardware faceplate | full width × 88 px high | persistent across every page |

The faceplate spans underneath both sidebar and workspace. This makes it read
as the physical device layer rather than another page toolbar.

The Sound workspace is arranged vertically:

1. page heading and live-versus-saved explanation;
2. Equalizer object header and graph;
3. Effects object header and controls.

The selected screen demonstrates both objects as modified so that their
separate save targets are obvious. Healthy saved state should be visually
quieter in daily use.

## Navigation sidebar

### Structure

- Overview
- **Sound** — selected
- Equalizer
- Playback
- Recording
- Mixer
- Lighting
- Device
- Settings

Related destinations use section labels and separators. Playback is never
grouped under an Input heading.

### Appearance

- dark base surface with no floating card around the navigation;
- 44 px minimum item height;
- 3 px cyan selected indicator plus a restrained cyan surface tint;
- icon and text remain visible at normal widths;
- tooltips and accessible names are mandatory when the sidebar collapses;
- no Creative-owned icons or profile artwork.

## Page heading

The page starts with:

- title: `Sound`;
- helper: `Changes apply immediately. Save each profile or preset separately.`

The heading contains no device controls and no page-level overflow menu.
Object-specific actions belong to their object header.

## Reusable object header

Equalizer and Effects use the same `ObjectHeader` component so their separate
ownership is visually consistent.

### Fields

1. object title;
2. optional explanatory subtitle;
3. labelled searchable selector;
4. state indicator;
5. scoped `Save` or `Save as…` action when applicable;
6. scoped overflow menu.

### States

| State | Presentation | Behavior |
|---|---|---|
| Saved | muted dot and `Saved` | no primary Save action |
| Modified | amber dot and `Modified` | values are already live; Save persists them |
| Applying… | spinner after 250 ms | controls involved in the write remain guarded |
| Not applied | red icon and text | inline mismatch plus Retry; prior confirmed value remains authoritative |
| Bypassed | secondary `Bypassed by Direct Mode` chip | Saved/Modified state remains visible |

Status never relies on color alone.

### Menus

Effects menu:

- Save as new profile…
- Revert to saved
- Reset to factory values
- Rename
- Duplicate
- Delete

EQ menu:

- Save as new preset…
- Revert to saved
- Flatten bands
- Reset to factory values
- Rename
- Duplicate
- Delete

Factory objects cannot be overwritten or deleted. Editing one changes its
display name to `Custom (from NAME)` and promotes `Save as…` as the primary
action.

## Equalizer section

### Header

- title: `Equalizer`;
- selector label: `EQ preset`;
- example selection: `SHP Last`;
- status and actions scoped only to the preset;
- modified subtitle:
  `Applied to the card. Not yet saved to "SHP Last".`

If the Effects profile recommends another EQ:

`"Gaming" suggests EQ preset "Gaming".`

The notice provides `Use it` and `Dismiss`. It appears inside the Equalizer
section because that is where the change would land.

### Graph

- ten fixed frequencies: 31, 62, 125, 250, 500, 1k, 2k, 4k, 8k, 16k Hz;
- gain range: -12 dB to +12 dB;
- clearly stronger 0 dB reference;
- cyan response curve;
- violet focus ring only on the focused band;
- current focused value in a compact bubble;
- no fake spectrum, analyzer, waveform, or decorative gradient.

`EqualizerGraph` uses `QtQuick.Shapes`. Each band handle is a real accessible
slider, not decoration.

### Interaction

- drag a node for direct adjustment;
- Arrow keys adjust by 0.5 dB;
- Shift+Arrow adjusts by 0.1 dB;
- Page Up/Down adjusts by 1 dB;
- Home sets 0 dB;
- Enter turns the focused value bubble into an editable numeric field;
- `Edit values…` in the menu opens a compact numeric grid for all ten bands;
- mouse wheel never changes a band unless the control has explicit focus.

Every change is sent through checked backend write/readback logic and marks
only the selected EQ preset as modified.

## Effects section

### Header

- title: `Effects`;
- selector label: `Effects profile`;
- example selection: `My profile`;
- status and actions scoped only to Effects;
- modified subtitle:
  `Applied to the card. Not yet saved to "My profile".`

The selector popup is searchable and grouped into:

- My profiles
- Games
- Movies
- Music
- Communication

### Direct Mode

Direct Mode is an orthogonal device condition, not an Effects or EQ value.

When enabled:

- EQ and Effects controls remain visible but become unavailable;
- both object headers show `Bypassed by Direct Mode`;
- Saved/Modified state remains intact;
- an explanation says that Direct Mode bypasses EQ and enhancements;
- disabling Direct Mode restores the previously confirmed values.

### Enhancement rows

Each row contains:

1. visible feature name;
2. capability/help affordance;
3. switch;
4. horizontal slider;
5. numeric value and unit;
6. inline failure or unavailability explanation when needed.

Initial rows:

- Surround
- Crystalizer
- Bass
- Smart Volume, with `Night` and `Loud` poles
- Dialog+

Off and 0% cannot contradict one another. If a feature is off, its slider is
disabled but the last stored level may remain visible as secondary text.
Unsupported capability is labelled with its cause instead of looking like an
ordinary disabled control.

## Persistent hardware faceplate

The faceplate is the only global control strip.

### Groups

1. **Device status**
   - `Sound BlasterX AE-5`
   - `Connected` or a precise failure state
   - `S16LE · 96 kHz`
2. **Output**
   - Speakers
   - Headphones
   - Digital
3. **Headphone gain**
   - Low
   - High
4. **Master volume**
   - mute
   - horizontal slider
   - numeric percentage
5. **Current setup**
   - `Effects: My profile`
   - `EQ: SHP Last`
   - `Live on card; save in each section.`
6. **Unsaved review**
   - `2 unsaved changes`
   - `Review`

`Review` navigates to modified objects. It never saves them.

### Safety

- volume shows a number at all times;
- wheel-over-slider does not change volume;
- user dragging and hardware readback cannot create a binding loop;
- writes are debounced while dragging and verified after release;
- mute remains visible on every page;
- High gain requires a deliberate guarded action;
- switching output is direct but shows Applying/Error readback;
- device disconnection disables writes and labels the reason;
- failed writes restore the previous confirmed value.

## Save and close behavior

`Ctrl+S` saves the object that owns focus.

- If one object is modified and focus is elsewhere, save that object.
- If both objects are modified and focus is elsewhere, open Review.
- Never silently save both.

`Ctrl+Shift+S` invokes `Save as…` for the focused object.

Closing with both objects modified opens one dialog with independent choices:

- Save Effects profile `My profile`
- Save EQ preset `SHP Last`
- Discard
- Cancel

Saving one object does not clear the other's Modified state.

## Visual system

### Semantic dark tokens

| Token | Initial value | Purpose |
|---|---|---|
| `background` | `#071725` | app base |
| `surface` | `#0C2131` | section surface |
| `surfaceRaised` | `#102A3C` | focused or selected surface |
| `separator` | `#294354` | 1 px grouping lines |
| `textPrimary` | `#F1F6F9` | headings and values |
| `textSecondary` | `#9FB1BC` | helper text |
| `accent` | `#00C7E6` | active controls |
| `focus` | `#9B73F4` | keyboard focus only |
| `success` | `#55C96A` | confirmed connection/write |
| `modified` | `#F3A72A` | unsaved object changes |
| `error` | `#F05F6D` | failed writes and daemon errors |

These are starting tokens, not hard-coded colors inside components. A light
theme maps the same semantic tokens to light values.

### Type and spacing

- system UI font;
- 28 px page title;
- 18 px section title;
- 14–16 px control/body text;
- 12–13 px secondary labels only where contrast remains sufficient;
- tabular figures for dB, Hz, percentage, and rate values;
- base spacing unit: 4 px;
- normal gaps: 8, 12, 16, 24, and 32 px;
- corner radius: 4–6 px;
- shadows avoided except where a popup needs platform-appropriate separation.

### Motion

- hover/focus transitions: 100–140 ms;
- selector and notice transitions: 140–180 ms;
- Applying spinner appears only after 250 ms;
- no decorative waveform, background, or continuous animation;
- moving between pages never interrupts audio.

## Responsive behavior

### 1280 × 800

Full sidebar, complete faceplate metadata, graph and five Effects rows visible.

### 1024 × 680

- sidebar collapses to a 72 px icon rail with tooltips;
- faceplate height reduces to approximately 76 px;
- device format collapses before route, mute, volume, or save-state summary;
- `Current setup` becomes one compact line;
- object headers remain sticky inside a vertical `ScrollView`;
- graph height never falls below 180 px;
- section Save and overflow actions remain visible;
- no horizontal page scrolling.

### 1600 × 1000

- sidebar grows to at most 224 px;
- graph and sliders expand;
- content does not become a sparse collection of oversized cards;
- effect labels and values retain comfortable fixed columns.

The same layouts must render correctly at 100%, 125%, 150%, 175%, and 200%
display scaling.

## Required system states

The same components must represent:

- connected and healthy;
- no compatible AE-5;
- daemon unavailable;
- driver loaded with partial capabilities;
- firmware missing;
- permission failure;
- device busy;
- speakers, headphones, or digital active;
- Direct Mode active;
- one or both sound objects modified;
- applying a hardware write;
- write failed and previous value restored;
- hardware value changed outside the application;
- lighting or another feature unsupported.

Healthy state stays calm. Exceptional states add only the explanation and
recovery action needed for that condition.

## QML component map

```text
AppShell
├── NavigationSidebar
├── PageStack
│   └── SoundPage
│       ├── ObjectHeader (EQ preset)
│       ├── EqualizerGraph
│       │   └── EqualizerBandHandle × 10
│       ├── CapabilityNotice
│       ├── ObjectHeader (Effects profile)
│       └── EnhancementRow × 5
└── HardwareFaceplate
    ├── DeviceStatus
    ├── OutputSelector
    ├── HeadphoneGainSelector
    ├── MasterVolumeControl
    ├── CurrentSetupSummary
    └── UnsavedReviewLink
```

Supporting components:

- `StateBadge`
- `SearchableObjectPicker`
- `InlineError`
- `ApplyingIndicator`
- `EmptyDeviceState`
- `NumericBandEditor`
- `UnsavedReviewDialog`

Custom drawing stays limited to `EqualizerGraph`, compact meters, and future
speaker calibration.

## Rust and daemon boundaries

Toolkit-independent logic remains in Rust. QML never writes ALSA or PipeWire
directly.

Suggested view models:

- `DeviceOutputViewModel`
- `EffectsProfileViewModel`
- `EqPresetViewModel`
- `SoundPageViewModel`
- `CapabilityViewModel`
- `DiagnosticsViewModel`

Each write follows:

```text
QML intent
→ CXX-Qt view model
→ typed D-Bus request
→ ae5d checked ALSA/PipeWire operation
→ hardware/session readback
→ confirmed value or typed failure
→ QML state update
```

The existing Rust backend remains the source of hardware truth. The current
GTK application stays buildable as a temporary fallback until the Qt/QML
screen passes functional parity.

## Incremental implementation plan

### Phase 1 — freeze contracts and baseline

- preserve current audio tests and known-good hardware state;
- inventory reusable Rust modules versus GTK-only code;
- define typed device, profile, EQ, capability, and error states;
- add regression fixtures for independent Effects/EQ modifications.

Exit: GUI replacement can be developed without changing audio semantics.

### Phase 2 — Qt/QML shell and theme

Status: **complete**

- add the Qt 6/CXX-Qt build path beside the GTK fallback;
- implement semantic theme tokens;
- build AppShell, navigation, page stack, and breakpoints;
- render the selected screen with deterministic mock data.

Exit: static layout matches the selected image at all target sizes.

### Phase 3 — daemon and live faceplate

Status: **in progress — live state, recovery, volume, and mute complete**

- [x] introduce `ae5d` and the typed `Device1.GetDeviceState` session D-Bus
  contract;
- [x] connect read-only device discovery, connection, format, output, gain,
  volume, mute, capability, and control-count state;
- [x] preserve the prior confirmed display and disable hardware controls when
  the daemon disappears, then recover automatically when it returns;
- [x] add checked master-volume and mute methods with readback, rollback,
  structured daemon logging, D-Bus failure propagation, and accessible QML
  controls;
- [ ] add output, gain, and Direct Mode writes only after each operation has
  the same readback, rollback, logging, and capability guarantees;
- preserve verified write/readback, rollback, trace, and safety guards.

Exit: the faceplate shows one authoritative live hardware state.

### Phase 4 — independent profile objects

Status: **core complete — independent persistence verified**

- [x] expose separate Effects-profile and EQ-preset libraries through typed
  Rust and D-Bus models;
- [x] split combined factory/imported data into section-owned objects and
  filter personal profiles by the live output;
- [x] load real selectors, Effects values, and ten-band EQ curves in QML
  without applying audio;
- [x] implement independent Rust-owned drafts, Save, Save as, and Revert;
- [x] preserve hidden section controls, refuse duplicate Save as names, update
  the local catalog, and discover saved objects after restart;
- [x] keep Current setup and independent unsaved counts without a combined
  profile transaction;
- [ ] add search/grouping and optional Windows-import EQ suggestions during
  later GUI polish.

Exit: modifying or saving one object cannot alter the other's state.

### Phase 5 — interactive EQ

Status: **in progress — checked software-EQ transaction connected**

- [x] implement the Shape-based graph and ten keyboard-accessible band
  controls with visible numeric dB values;
- [x] load factory, imported, and custom preset curves into independent
  Rust-owned drafts;
- [x] expose saved/configured/current/different/unavailable runtime states
  separately from preset Saved/Modified state;
- [x] connect typed Apply and Disable operations through `ae5d`, with exact
  output targeting, OutFX and Direct Mode conflict checks, runtime ownership,
  marker readback, automatic preamp, config/runtime rollback, and structured
  logging;
- [x] verify the OutFX-on failure path on physical hardware: the daemon refused
  the request before writes, the managed config remained absent, no runtime
  graph appeared, and the 20% unmuted sink state was preserved;
- [ ] run one guarded OutFX-off apply/change/disable/restart cycle at no more
  than 20%, then record response and persistence evidence before closing the
  phase.

Exit: graph, numeric values, runtime EQ state, and persisted preset agree.

### Phase 6 — Effects and Direct Mode

- implement capability-driven enhancement rows;
- connect live effect writes and per-profile modification tracking;
- implement Direct Mode bypass and restoration behavior.

Exit: every visible control maps to a checked mechanism or explains why it
is unavailable.

### Phase 7 — failure states, accessibility, and scaling

Status: **in progress — deterministic failure rendering, responsive layout,
focus order, and close handling complete; physical failure injection and
screen-reader acceptance remain**

- [x] implement disconnected, partial, busy, permission, firmware, daemon,
  write-failure, Direct Mode, and independently modified states as reusable
  deterministic fixtures that cannot write audio hardware;
- [x] expose meaningful AT-SPI roles, names, descriptions, values, and
  applying/error status for navigation, notices, object headers, live EQ,
  equalizer bands, enhancement controls, and the hardware faceplate;
- [x] give custom controls keyboard-only focus rings, remove decorative icon
  buttons from the accessibility tree, explain disabled controls, and expose
  focused tooltips;
- [x] connect object-scoped `Ctrl+S`, `Ctrl+Shift+S`, and faceplate Review
  behavior without introducing a combined profile save;
- [x] use Qt Quick Controls Basic and semantic dark/light palettes whose
  normal text and status colors pass the intended contrast checks;
- [x] verify native Wayland layouts at 1024 × 680, 1280 × 800, and
  1600 × 1000 with no horizontal overflow, plus an X11 startup smoke test;
- [x] implement object-scoped close-with-unsaved choices without introducing a
  combined save;
- [x] add exact automated focus-order assertions for healthy and both-modified
  states, including object-scoped Save, Revert, and Review actions;
- [ ] complete physical failure injection and screen-reader reading-order
  acceptance;
- [x] verify 125%, 150%, and 200% display scaling. The explicit `--light`
  palette is available now; automatic system-theme following remains later
  polish.

Exit: no state requires a terminal or application restart to understand.

### Phase 8 — remaining pages and release

- [x] port Overview, Equalizer, Playback, Recording, Mixer, Lighting, Device,
  and Settings into the same shell and state model, with functional
  navigation, responsive scrolling, and honest capability states;
- [ ] connect the currently read-only Recording, extra Mixer, Lighting,
  speaker-configuration, and maintenance actions to narrowly scoped typed
  `ae5d` methods, then complete their physical acceptance;
- [x] package the Qt/QML app and user daemon in both the RPM and transactional
  rootless installer, with D-Bus activation and immediate session-bus reload;
- [ ] run the authenticated host RPM install/upgrade/rollback and remaining
  hardware acceptance;
- [ ] retire GTK only after accepted parity.

Exit: packaged daily-use candidate satisfies the roadmap and preserves audio
safety.

## Verification matrix

### Automated

- Rust unit tests for profile ownership and state transitions;
- D-Bus contract tests with fake daemon responses;
- QML component tests for Saved/Modified/Applying/Error;
- screenshot tests at 1024 × 680, 1280 × 800, and 1600 × 1000;
- keyboard navigation and accessible-name assertions;
- lint, format, release build, and package smoke tests.

### Hardware

- cold start, warm start, daemon restart, and application restart;
- output switching with verified readback;
- volume and mute without exceeding the repository's 20% test ceiling;
- Low/High gain guard;
- personal and representative factory Effects profiles;
- personal and representative factory EQ presets;
- one-object and two-object modification/save/revert;
- Direct Mode bypass and recovery;
- injected write failure and rollback;
- external ALSA change reflected in the UI;
- 48 and 96 kHz paths plus the existing supported-rate matrix;
- Wayland first, then X11 smoke validation.

## Definition of done for the selected design

The redesign is accepted when:

1. the packaged Qt/QML application launches natively under Wayland;
2. the selected Sound screen matches this specification at 1280 × 800 and
   remains usable at 1024 × 680;
3. device output, Effects profile, and EQ preset cannot be confused;
4. every visible write uses verified backend readback and safe rollback;
5. both sound objects can be modified, saved, reverted, and reviewed
   independently;
6. Direct Mode and unsupported capabilities are explained in context;
7. keyboard, scaling, contrast, and screen-reader labels pass;
8. existing audio functionality and safety tests do not regress;
9. RPM install, upgrade, launch, recovery, and removal are reproducible;
10. the user completes the daily-use acceptance checklist.

## Decisions not to reopen without new evidence

- Version 2 / hardware-faceplate direction is selected.
- Device output is not part of Effects profiles or EQ presets.
- Effects and EQ never share one Save action.
- Current setup remains read-only.
- suggested EQ changes require explicit consent.
- Qt 6/QML and Rust/CXX-Qt remain the production stack.
- backend safety and readback take priority over visual fidelity.
- no Creative-owned profile artwork is committed.
