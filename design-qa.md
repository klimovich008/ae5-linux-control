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

## Installed-host acceptance

Commit `b068ff9` was rebuilt and upgraded through the transactional rootless
installer on the physical Fedora/KDE Wayland host. The installed
`ae5-control-qml` and `ae5d` executables byte-match that release build. The
desktop entry and systemd user unit validate, and all seven existing
configuration/profile files retained the same aggregate digest across the
upgrade.

The installed executable then passed:

- all ten deterministic states under native Wayland and X11/XWayland;
- the healthy focus audit at 1024×680, 1280×800, and 1600×1000;
- the independently modified focus audit at the minimum X11/XWayland size;
- a 200% Wayland scaling focus audit;
- a live, read-only Wayland launch against the physical AE-5 and `ae5d`;
- AT-SPI inspection of the page, graph, output, gain, mute, volume, profile,
  enhancement, and status semantics.

The live screen reported Headphones, Medium gain, S16LE/96 kHz, 20% master
volume, 35 Effects profiles, and 37 EQ presets. `ae5d` logged no write event.
The PipeWire volume and all profile files were unchanged. The only full ALSA
snapshot delta was the read-only volatile `Capture Channel Map` reporting
`FL,FR` instead of unassigned values; no writable mixer control changed.

The exact Fedora 44 RPM is
`dist/qt-qml-b068ff9/ae5-control-0.1.0-1.fc44.x86_64.rpm` with SHA-256
`2e4c768ffe51b5a0a09e70c4a29e9cdc0c8da00fcb78b71dfdd4687f9b597b8d`.
Its clean install/remove and runtime checks passed in CI. The host system-RPM
transaction remains an authenticated packaging gate because the current
session has neither passwordless sudo nor noninteractive polkit; the rootless
package is the installed testable delivery.

The remaining project-wide work is physical hardware acceptance, remaining
pages, and the authenticated host system-RPM lifecycle. Those items do not
block the Sound-screen visual/state goal.

## Multi-page shell addendum

The remaining navigation destinations were implemented after the Sound-screen
acceptance. Evidence and findings are recorded in
`docs/design/audit-2026-07-30-all-tabs/README.md`.

### Source and implementation comparison

- selected source:
  `docs/design/ae5-control-sound-selected-v2.png`;
- implementation fixture:
  `--qa-state=both-modified --qa-page=sound --qa-window=1280x800`;
- same-canvas comparison:
  `docs/design/audit-2026-07-30-all-tabs/design-qa-sound-full.png`;
- focused sidebar comparison:
  `docs/design/audit-2026-07-30-all-tabs/design-qa-sidebar-focus.png`;
- all nine 1280×800 destinations:
  `docs/design/audit-2026-07-30-all-tabs/page-*-1280x800.png`;
- representative minimum-window captures:
  `docs/design/audit-2026-07-30-all-tabs/compact-*-1024x680.png`;
- semantic light-theme capture:
  `docs/design/audit-2026-07-30-all-tabs/page-sound-light-1280x800.png`.

The pre-fix screenshot showed each sidebar icon/label pair shifted below the
center of its 44 px row. The cause was inherited `ItemDelegate` top and bottom
padding. Both navigation delegate groups now set zero vertical padding; AT-SPI
reports matching row and label bounds, and the focused comparison confirms
optical centering and consistent icon sizing.

Overview, Equalizer, Playback, Recording, Mixer, Lighting, Device, and
Settings now use functional keyboard/pointer navigation and the same semantic
components. The footer remains the single owner of output, gain, mute, and
master volume. Equalizer edits the same Rust-owned preset as Sound.
Unavailable typed writes remain visibly read-only, deferred, or guarded.

The multi-page pass added nine destination smokes to the existing ten-state
and focus-order test. A native Wayland release launch emitted no QML error.
Representative 1024×680 pages use the compact icon rail and vertical scrolling
without overlap or horizontal overflow.

The release build, strict Qt/QML-target Clippy check, ShellCheck, formatting,
and diff checks passed. The workspace test run passed 147 tests. All visual
captures used deterministic no-write fixtures; the separate live startup smoke
performed no control action.

The addendum does not replace the earlier installed-host acceptance. It closes
the navigation and visual shell slice; deferred backend integration and
physical hardware acceptance remain roadmap work.

final result: passed
