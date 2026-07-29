# AE5 Control design QA

Date: 2026-07-30
Scope: Qt 6/QML Sound screen, persistent hardware faceplate, deterministic
failure states, and keyboard/accessibility behavior
Result scope: visual and deterministic state acceptance; physical hardware
write acceptance remains governed by the roadmap

## Visual truth and implementation evidence

- Source visual truth:
  `docs/design/ae5-control-sound-selected-v2.png`
- Final healthy QA state, 1280×800:
  `docs/design/qa-2026-07-30/06-final-ready-1280x800.png`
- Final minimum layout, 1024×680:
  `docs/design/qa-2026-07-30/02-ready-1024x680.png`
- Final wide layout, 1600×1000:
  `docs/design/qa-2026-07-30/03-ready-1600x1000.png`
- Final permission failure, 1280×800:
  `docs/design/qa-2026-07-30/07-final-permission-denied-1280x800.png`
- Final independently modified Effects and EQ state, 1280×800:
  `docs/design/qa-2026-07-30/10-final-both-modified-1280x800.png`
- Same-canvas source/implementation comparison:
  `docs/design/qa-2026-07-30/08-reference-vs-final-ready.png`

All captures are decoration-free Wayland application canvases. The comparison
normalizes the selected 1586×992 visual to the implementation's 1280×800
canvas before placing the two images side by side.

## Review method

The selected image and final 1280×800 implementation were inspected in one
combined comparison input. The 1024×680 and 1600×1000 layouts were then
inspected independently. Permission-denied, write-failed, and both-modified
states were rendered from deterministic fixtures that cannot access ALSA,
PipeWire, D-Bus, or the AE-5.

Claude Opus 5 (`claude-opus-5`) was used as a strict second reviewer. Its
first review found actionable state and interaction issues. The accepted
findings were fixed before final QA:

- unsaved counts are derived from actual draft-versus-saved content;
- modified drafts survive catalog refreshes and output changes;
- Review lands on a safe enabled editor/menu control, not a disabled selector
  or an immediate Save action;
- a failed volume write restores the last confirmed slider value without a
  polling tick cancelling a pending user edit;
- write errors remain visible but cannot hide a later no-device, permission,
  firmware, busy, or daemon failure;
- live EQ apply and disable run off the Qt event thread and reject overlapping
  operations in Rust;
- EQ keyboard shortcuts no longer replace the slider's value binding;
- display-state tokens use explicit normalized meanings instead of substring
  color matching;
- the modified-state focus chain and visible Save/Revert/Review actions are
  covered by the automated focus audit.

## Visual findings

### Hierarchy and alignment

- Device output appears only in the persistent faceplate.
- Effects profiles and EQ presets retain separate selectors, state badges,
  Save/Revert actions, and drafts.
- Equalizer and Effects headers share a stable alignment grid at 1280 and
  1600 logical pixels.
- The 1280 modified-state footer uses a compact `2 unsaved` action, preventing
  Current setup or device identity from being pushed beyond the window.
- At 1024×680 the navigation becomes an icon rail, the footer compresses to
  SPK/HP/DIG, and the workspace scrolls vertically with no horizontal
  overflow.

### Color and icon language

- Cyan is reserved for selected navigation, active controls, and primary
  action emphasis.
- Violet is reserved for keyboard focus.
- Green, amber, and red are semantic confirmed, modified, and error colors;
  every state also has text.
- Navigation and control icons are deterministic bundled Phosphor assets.
- Disabled checked controls remain visibly selected while using disabled
  contrast, avoiding false live affordances.
- Healthy state stays visually quiet; permission and write failures add one
  restrained notice and one recovery action.

### Pointer, keyboard, and accessibility behavior

- Buttons and selectors use pointing-hand cursors.
- sliders and EQ nodes use open/closed-hand drag cursors;
- disabled controls retain the normal cursor and expose a reason by tooltip;
- the healthy focus chain has 31 named stops;
- the both-modified focus chain additionally covers independent Revert, Save,
  and Review actions;
- decorative help icons are no longer tab stops, while their explanation is
  carried by the accessible enhancement group;
- AT-SPI exposes page, object, graph, enhancement, output, mute, volume, and
  status semantics.

## Capability and failure-state findings

The same components now render ten deterministic scenarios:

1. healthy;
2. no compatible card;
3. partial driver capabilities;
4. firmware missing;
5. permission denied;
6. device busy;
7. write failed with prior value authoritative;
8. daemon unavailable;
9. Direct Mode bypass;
10. both sound objects modified.

Unsupported and failed states explain whether the cause is the card, driver,
firmware, permission, daemon, output mode, or a write failure. Retry wording is
specific to the cause. Hardware controls are disabled when the state cannot
support a safe write.

## Verification

- `cargo fmt --check`
- `git diff --check`
- strict `cargo clippy` with `-D warnings`
- 155 Rust tests passed
- release Qt/QML build passed
- exact focus-order audits passed for healthy and both-modified states
- offscreen render smoke passed for all ten deterministic scenarios
- native Wayland visual inspection passed at 1024×680, 1280×800, and
  1600×1000
- source and final implementation were reviewed together at the same canvas
- no audio output, gain, mute, volume, EQ, OutFX, or Direct Mode write was
  issued during this QA pass

The remaining release work is hardware acceptance, remaining pages, and
package refresh. Those items do not block the Sound-screen visual/state QA but
still block the project-wide definition of done.

final result: passed
