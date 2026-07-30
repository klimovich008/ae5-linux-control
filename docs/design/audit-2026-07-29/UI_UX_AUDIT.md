# AE5 Control UI/UX audit

Date: 2026-07-29  
Scope: current Qt 6/QML Sound screen only  
Audio safety: no hardware or audio state was changed during capture or review

## Evidence

| Capture | Purpose |
|---|---|
| [1280×800 dark](./01-sound-1280x800-dark.png) | Default desktop target |
| [1024×680 dark](./02-sound-1024x680-dark.png) | Minimum supported window |
| [1600×1000 dark](./03-sound-1600x1000-dark.png) | Comfortable wide layout |
| [1280×800 light](./04-sound-1280x800-light.png) | Light-theme behavior |
| [Selected direction](../ae5-control-sound-selected-v2.png) | Intended visual hierarchy |

The current screenshots were captured from the running Wayland build. The QML
accessibility tree was also inspected. This is not a claim of WCAG compliance:
fractional scaling, screen-reader workflows, color-management output and every
failure state still require dedicated validation.

## Independent review

Claude was used twice as a read-only second brain:

- Model returned by the CLI: `claude-opus-5` (`canonicalModel:
  claude-opus-5`).
- It inspected the four current captures, selected reference, implementation
  spec and nine QML files.
- It made no edits, performed no web searches and had no permission denials.
- Pass 1 challenged the diagnosis; pass 2 supplied a concrete token,
  component and file-level remediation map.

The conclusions below combine direct screenshot/source inspection with the
Opus 5 review. They are not copied blindly: disagreements are resolved in
[Design decisions](#design-decisions).

## Overall diagnosis

The product structure is sound, but its presentation layer is not yet a
coherent design system.

The shell, section-owned Effects/EQ profiles, real ten-band EQ controls and
fixed hardware faceplate are worth preserving. The problems come from three
implementation gaps:

1. `Theme.qml` is a short color list rather than a complete set of semantic
   color, spacing, typography, control, icon and breakpoint tokens.
2. Raw Qt Quick Controls Basic widgets are mixed with individually styled
   controls, producing inconsistent selected, hover and light-theme states.
3. Layout metrics are local literals rather than a shared grid, so headers,
   rows and faceplate groups drift apart at different widths.

This is why isolated color fixes would not be enough.

## Experience health

1. **Launch and confirm the AE-5 — Good.** Device identity, connection state,
   output format and current route are visible. The healthy notice is too
   prominent, but the information is present.
2. **Choose an EQ preset or Effects profile — Mixed.** The two objects are
   correctly independent, but their selectors and state labels do not share a
   stable grid. “Preview” looks like an active cyan control rather than a state.
3. **Inspect and edit EQ — Mixed.** The graph is a real accessible interaction
   with keyboard handling, but it has almost no hover/drag affordance, tiny
   labels and no visible numeric alternative on this screen.
4. **Adjust Effects — Needs work.** Switches and sliders are visually
   inconsistent, Smart Volume breaks the row grid, and lower rows are clipped
   at the default and minimum sizes.
5. **Use output, gain, mute and volume — At risk.** The fixed faceplate is the
   right location, but it is overcrowded. Output and gain use different styles;
   the selected Medium gain reads as disabled. Gain is currently read-only and
   must not be made writable by a visual refactor.
6. **Use the app at 1024×680 — Poor.** The rail icons become the only labels,
   but several are ambiguous or unresolved. Digital output and the gain group
   disappear, and content is cut at the faceplate boundary.
7. **Use the light theme — Poor.** Surface hierarchy is weak and raw Basic
   switches/sliders produce heavy black or grey tracks that conflict with the
   intended palette.

## What should be preserved

- The faceplate spans the complete bottom edge, including below the sidebar.
- Hardware output remains separate from Effects profiles and EQ presets.
- Effects and EQ save independently; there is no global Save button.
- Direct Mode keeps a dedicated row because it bypasses more than one section.
- Equalizer handles remain real QML sliders with keyboard support.
- The graph remains restrained: no fake spectrum or decorative analyzer.
- The sidebar keeps one meaningful group separator rather than a caption above
  every single item.
- Existing Rust, D-Bus, daemon and audio behavior remain outside this visual
  remediation.

## P0 — blocking visual/usability issues

### P0.1 Content is visibly cut by the faceplate

At 1280×800 the Crystalizer row is bisected. At 1024×680 the Direct Mode
description and following controls are cut. A user sees a broken render before
they see a scrollable page.

Cause: `SoundPage.qml` clips the `ScrollView`, has no bottom breathing room and
does not provide a persistent scroll cue. The fixed faceplate is correct; the
content budget above it is not.

Fix:

- add bottom padding and reserve the vertical scrollbar gutter;
- prevent a row from landing half-visible at the faceplate boundary;
- demote the healthy notice to one line;
- use a two-column Effects layout at the default width if screenshot validation
  confirms it is needed to fit all five controls;
- keep a visible thin scrollbar at the minimum size.

### P0.2 Pointer and hover language is missing

There is no `cursorShape` declaration anywhere in the QML. A few controls react
to hover, but many have no visual response, and EQ points only react after
focus.

Fix:

- pointing-hand cursor for buttons, navigation, switches, segments, combo boxes
  and icon actions;
- context-appropriate drag feedback for sliders and EQ points rather than
  pretending they are navigation links;
- visible hover/pressed states from shared tokens;
- value bubble and larger handle on EQ hover;
- keep disabled controls on the normal arrow cursor and explain them with a
  blocked reason.

### P0.3 Adjacent controls use incompatible styles

Output is custom styled while Gain uses raw Basic buttons. In dark mode the
selected Medium gain is a large grey rectangle that reads as disabled; in light
mode it becomes even more conspicuous. Raw switches/sliders also render with
heavy black tracks.

Fix: introduce a minimal themed control layer and use the same
`SegmentedControl`, `AppSwitch` and `AppSlider` in both themes.

Safety boundary: the current QML gain segments have no click handler, and
`headphoneGainWriteEnabled` is false in the device state. The redesign must
show the confirmed gain as read-only with an explanation. It must not silently
enable gain writes.

### P0.4 Icons are environment-dependent and semantically inconsistent

The app relies on freedesktop host-theme icon names. Equalizer and Lighting
degrade to a dot/square in the current captures; Sound and Mixer reuse speaker
metaphors; Device uses a disk icon for a PCIe sound card. The 72 px compact rail
makes these glyphs the only navigation labels.

Fix: bundle one permissively licensed monochrome icon set in the Qt resource
collection, use explicit 16/18/20/24 px size tokens, and map icons by semantic
role. Lucide is the strongest current candidate.

Minimum visible mapping:

| Destination/action | Icon concept |
|---|---|
| Overview | dashboard |
| Sound | volume/output |
| Equalizer | vertical sliders |
| Playback | play circle |
| Recording | microphone |
| Mixer | horizontal sliders |
| Lighting | lightbulb |
| Device | PCIe device/chip |
| Settings | settings |
| More actions | vertical ellipsis |
| Information | info |
| Mute states | volume, volume-1, volume-2, volume-x |
| Save/Revert | save, rotate counter-clockwise |
| Status | check, warning, error, applying |

### P0.5 Compact layout removes important controls

At 1024×680 the output model drops Digital and the complete Gain group becomes
invisible. Compact output labels become unexplained `SPK` and `HP`.

Fix: preserve every route. If all three segments cannot fit, Digital belongs in
an explicit overflow selector with a visible current-state summary. Collapse
audio-format detail before route selection. Read-only gain may collapse only
after its confirmed value remains visible elsewhere.

## P1 — required design-system work

### Color and state semantics

- Cyan currently means selected navigation, editable graph, selected output
  and neutral “Preview” state. Reserve cyan for interaction and current
  selection.
- Green should mean connected/saved/success, not dominate the healthy screen.
- Violet is a keyboard-focus token; it must not tint Direct Mode state.
- Modified, applying, failed and bypassed need explicit state roles and
  text/icons, not color alone.
- Light surfaces need a clearer background/sidebar/surface/sunken ramp.
- Inline alpha blends should be replaced by explicit semantic fills and borders
  so light and dark themes render predictably.

Proposed starting roles:

| Role | Light | Dark |
|---|---:|---:|
| Background | `#F4F8FA` | `#071725` |
| Sidebar | `#E8F0F4` | `#081B2A` |
| Surface | `#FFFFFF` | `#0C2131` |
| Raised | `#EDF3F7` | `#102A3C` |
| Sunken/track | `#DDE7EE` | `#051019` |
| Separator | `#C6D5DE` | `#294354` |
| Strong outline | `#9FB5C2` | `#3C5C72` |
| Primary text | `#10212C` | `#F1F6F9` |
| Secondary text | `#405967` | `#9FB1BC` |
| Accent | `#006F86` | `#00C7E6` |
| Accent hover | `#00596C` | `#4FDCF2` |
| Accent pressed | `#004353` | `#00A2BB` |
| Focus | `#6941C6` | `#9B73F4` |
| Success | `#187A32` | `#55C96A` |
| Modified | `#8A5200` | `#F3A72A` |
| Error | `#B42335` | `#F05F6D` |

These are implementation candidates, not certified contrast results. Rendered
text, status tints, disabled labels and 100–200% scaling must be sampled after
the themed controls exist.

### Alignment and responsive grid

- Add one 4 px-based spacing scale: 4, 8, 12, 16, 24, 32.
- Stop using local widths for every header and row.
- Align both object headers to the same columns: title/detail, selector,
  state, actions.
- Reserve the Save/Revert action slot so a modified state does not shift the
  header.
- Use one Effects row grid: name, info, switch, slider, value.
- Put Smart Volume’s Night/Loud poles below its slider so its track aligns with
  every other effect.
- Give faceplate captions a fixed baseline and the same control height.
- Cap wide content near 1280 logical px; use the remaining width as a calm
  gutter rather than stretching the EQ curve indefinitely.
- Derive all breakpoints from the window/page contract in one place.

Recommended behavior:

| Window | Navigation | Content | Faceplate |
|---|---|---|---|
| 1024×680 | 72 px rail with deterministic icons/tooltips | one-column Effects, visible scrollbar, 180 px graph floor | 76 px; keep route, mute, volume and connected state |
| 1280×800 | 208 px sidebar | validate two-column Effects, 200 px graph, all controls uncut | 88 px; full route labels and read-only gain |
| 1600×1000 | 224 px sidebar | max-width cap, taller graph, no uncontrolled horizontal stretching | 88 px; stable shared baselines |

### Typography

- Reduce the current seven-size mixture to a small ramp: page title 28,
  section title 18, body 14, label 13, caption 12.
- Avoid 10–11 px text for status-bearing information.
- Replace literal `font.family: "monospace"` with tabular figures when the
  available Qt version supports them; keep values aligned without making the
  UI look like a terminal.
- Sentence-case faceplate captions are easier to read than 10 px all-caps.

### Discoverability and explanations

- Enabled icon-only actions always receive a tooltip.
- Disabled controls receive their concrete blocked reason.
- Visible text buttons do not need duplicate tooltips.
- An elided label receives its full text on hover/focus.
- The disabled profile selector must show why it is locked while a draft is
  modified; this rule should not live only in accessibility text.
- “Preview” must become a defined `StateBadge` state or be renamed to the
  product vocabulary (`Not applied`, `Saved`, `Modified`, `Applying`,
  `Bypassed`, `Error`).

## P2 — polish after the structural pass

- Clip the sidebar selection bar to the rounded selected background.
- Draw keyboard focus outside controls so borders do not move content.
- Show the EQ value bubble on hover and keyboard focus.
- Put `dB` next to the value axis and include `Hz` in the frequency axis.
- Reflect 0/low/high/muted in the faceplate volume icon.
- Increase tiny status dots to an even optical size and accompany important
  states with an icon/text.
- Give long translated labels elision or wrapping rules.
- Add deterministic motion tokens around 120–220 ms and delay applying
  spinners so fast writes do not flash.

## Minimal QML component layer

The review recommends a deliberately small layer:

| Component | Responsibility |
|---|---|
| `FocusRing` | shared external keyboard-focus outline |
| `AppButton` | primary, secondary, ghost and danger variants |
| `IconButton` | deterministic icon, tooltip, blocked reason |
| `AppSwitch` | explicit track/thumb colors and user-only toggle signal |
| `AppSlider` | explicit track/handle, safe wheel policy, optional debounce |
| `AppComboBox` | deterministic field, chevron, popup and value selection |
| `SegmentedControl` | confirmed-value selection with joined borders |
| `StateBadge` | saved/modified/applying/error/bypassed/preview states |

Do not add generic Card/Form/Section abstractions yet. Keep native Qt popup,
menu, dialog, keyboard and close-policy behavior.

## File-level remediation map

| File | Main change |
|---|---|
| `qml/Theme.qml` | Add semantic colors, spacing, typography, dimensions, icon, motion and breakpoint tokens |
| `qml/Main.qml` | Complete the palette for Basic popups/dialogs/tooltips; consume breakpoint and sidebar tokens |
| `qml/pages/SoundPage.qml` | Fix content budget, width cap, scrollbar, Effects columns and shared gutters |
| `qml/components/NavigationSidebar.qml` | Deterministic icons, semantic keys, consistent unavailable state, pointer/hover/focus |
| `qml/components/ObjectHeader.qml` | Shared selector/state/action grid, `StateBadge`, reserved action slot |
| `qml/components/HardwareFaceplate.qml` | Shared segmented controls, aligned captions, read-only gain truth, stable compact content |
| `qml/components/EnhancementRow.qml` | Shared row grid, themed controls, poles below track, external state re-sync |
| `qml/components/EqualizerGraph.qml` | Hover/drag feedback, scalable insets, value/axis clarity, keyboard-focus ring |
| `qml/components/CapabilityNotice.qml` | Quiet healthy variant; semantic warning/error variants |

The long inline Live EQ and Direct Mode blocks may become focused components
after the control layer lands. That extraction should follow visible fixes, not
precede them.

## Design decisions

The two reviews disagreed in useful ways. The final recommendations are:

1. **Keep the fixed faceplate.** Fix the page budget above it.
2. **Keep the healthy status in the accessibility tree, but visually demote
   it.** Do not remove useful device state.
3. **Use context-appropriate cursors.** Pointing hand for button-like controls;
   visible drag feedback for sliders/EQ; no forbidden cursor.
4. **Do not add tooltips everywhere.** Use them for icon-only, blocked and
   elided cases.
5. **Preserve native Qt menu/dialog behavior.** Style the small control set
   instead of forking every Basic control.
6. **Defer sticky object headers.** First fix clipping and height budgeting;
   revisit sticky behavior at 1024 after that baseline is stable.
7. **Do not enable gain writes in the visual pass.** Show confirmed read-only
   state honestly.
8. **Treat two-column Effects as a measured adaptation, not doctrine.** It is
   likely necessary at 1280×800 to avoid scrolling, but must be validated
   against row readability after the grid is fixed.

## Incremental patch sequence

1. Add tokens and complete the Qt palette; verify dark/light healthy state.
2. Add `FocusRing`, `AppButton`, `IconButton` and `StateBadge`; fix the healthy
   notice and object headers.
3. Fix page height/width budgeting and Effects layout; re-capture all three
   window sizes.
4. Bundle and replace icons; verify the compact rail on KDE and GNOME.
5. Add `AppSwitch`, `AppSlider`, `AppComboBox` and `SegmentedControl`; replace
   raw Basic visuals without changing write behavior.
6. Rebuild the faceplate grid and keep gain explicitly read-only.
7. Improve EQ hover/value/axis behavior and enhancement-row alignment.
8. Validate keyboard focus, hover, disabled explanations, light/dark themes and
   100/125/150/175/200% scaling.

After steps 3, 5 and 8, run the existing feature-parity checks to guard against
accidental GUI-to-backend behavior changes.
