# AE5 Control design QA

Date: 2026-07-29
Scope: Qt 6/QML Sound screen and persistent hardware faceplate
Result scope: healthy, connected AE-5 state only

## Visual truth and implementation evidence

- Source visual truth:
  `docs/design/ae5-control-sound-selected-v2.png`
- Default dark implementation, 1280×800 logical:
  `docs/design/qa-2026-07-29/13-final-1280x800-dark.png`
- Minimum dark implementation, 1024×680 logical:
  `docs/design/qa-2026-07-29/06-final-1024x680-dark.png`
- Wide dark implementation, 1600×1000 logical:
  `docs/design/qa-2026-07-29/09-final-1600x1000-dark-settled.png`
- Default light implementation, 1280×800 logical:
  `docs/design/qa-2026-07-29/10-final-1280x800-light.png`
- Full source/implementation comparison:
  `docs/design/qa-2026-07-29/14-comparison-final-full.png`
- Main-workspace comparison:
  `docs/design/qa-2026-07-29/15-comparison-final-main.png`
- Hardware-faceplate comparison:
  `docs/design/qa-2026-07-29/16-comparison-final-faceplate.png`

The implementation captures include the 28 px KDE window decoration. Comparison
images remove that decoration and normalize both designs to a 1280×800
application canvas.

## Captured state

- AE-5 connected through `ae5d`
- Headphones selected
- Medium headphone gain confirmed and read-only
- Master volume at 20%, unmuted
- Effects profile: `My profile`
- EQ preset: `SHP Last`
- Effects and EQ shown as independent Preview objects
- Software EQ inactive because the current OutFX state blocks a second
  processing path
- No GUI audio control was activated while capturing or reviewing

## Full-view comparison

The implementation preserves the selected direction's hierarchy: persistent
navigation, section-owned EQ and Effects objects, a central ten-band EQ, a
dedicated Direct Mode row, five enhancement controls, and a full-width hardware
faceplate.

Intentional departures from the concept image:

1. The implementation uses the real healthy Preview state instead of inventing
   Modified or unsaved state for the screenshot.
2. A compact Live EQ row exposes the real software-processing state and blocked
   apply reason.
3. Effects use two columns at 1280×800 so all five controls remain visible
   without reducing target sizes or placing content behind the faceplate.
4. The footer omits a decorative product thumbnail and shows confirmed device
   text instead.
5. Unimplemented destinations are visibly unavailable while retaining
   deterministic icons and accessible names.

## Focused-region findings

### Navigation and shell

- Selected Sound state has a clipped accent rail, semantic Phosphor icon,
  readable label, and stable 44 px target.
- Future destinations use deterministic bundled icons rather than host-theme
  fallbacks.
- The 1024 px rail retains tooltips and accessible destination names.
- No P0, P1, or P2 visual issue remains in the captured shell.

### EQ and Effects workspace

- EQ and Effects headers share one stable title/detail, selector, state, and
  action grid.
- Selector widths and action reservations no longer shift between sections.
- Neutral Preview uses grey text/dot; cyan remains reserved for interaction and
  current selection.
- The EQ curve exposes the 0 dB reference, dB axis, ten frequencies, real
  controls, hover/focus value bubbles, and drag cursors without fake spectrum
  data.
- All five enhancement controls are visible at 1280×800 and 1600×1000.
- At 1024×680 the page becomes one column and shows a persistent scrollbar
  instead of clipping a row at the faceplate.
- No P0, P1, or P2 visual issue remains in the captured workspace.

### Hardware faceplate

- Speakers, Headphones, and Digital remain represented at every width.
- Compact SPK/HP/DIG labels have full-name tooltips and accessible names.
- Medium gain remains selected but read-only; the UI does not add a write
  handler or enable the backend capability.
- Master volume, mute, connected state, format, active objects, and confirmed
  gain remain available without duplicate controls elsewhere.
- Full output and gain labels fit at 1280×800 and above.
- No P0, P1, or P2 visual issue remains in the captured faceplate.

### Theme and interaction language

- Dark and light themes use semantic surface, text, selection, success,
  modified, error, disabled, and focus tokens.
- Buttons, icon buttons, combo boxes, switches, sliders, EQ points, and
  navigation items use shared hover/pressed/focus styling.
- Button-like controls use pointing-hand cursors; sliders and EQ points use
  open/closed-hand drag cursors; disabled controls retain the normal cursor and
  a blocked-reason tooltip.
- The AT-SPI tree exposes the page, object selectors, EQ sliders, effect
  controls, output routes, mute, volume, and status text. Synthetic keyboard
  input was unavailable on this host, so focus-ring behavior was source-checked
  rather than captured with a Tab-driven screenshot.
- No P0, P1, or P2 visual issue remains in the captured theme states.

## Responsive and platform checks

- Wayland captures passed at 1024×680, 1280×800, and 1600×1000.
- Light and dark startup/rendering passed.
- Wayland startup smoke passed with Qt scale factors 1.25, 1.5, and 2.0.
- X11/XWayland startup remained healthy for the six-second smoke window.

## Non-visual regression checks

- `cargo fmt --check`
- `cargo test --features qml-gui`: 147 tests passed
- `scripts/check-feature-parity.sh`: 54 rows validated
- `scripts/audio-parity.sh --self-test`
- `git diff --check`
- Qt QML lint for the final edited page/header passed

final result: passed
