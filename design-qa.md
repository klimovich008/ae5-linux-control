# Design QA — AE5 Control selected Sound screen

Final result: **passed**

## Evidence

- Source:
  [`docs/design/ae5-control-sound-selected-v2.png`](docs/design/ae5-control-sound-selected-v2.png)
- Implementation:
  [`docs/design/ae5-control-qml-preview-phase2.png`](docs/design/ae5-control-qml-preview-phase2.png)
- Source capture: 1586 × 992, normalized to 1280 × 800 for comparison.
- Implementation capture: 1280 × 800 at 1× scale on native Wayland.
- Compared state: dark theme, Headphones output, Low gain, 20% master volume,
  `SHP Last` EQ preset modified, `My profile` Effects profile modified, two
  unsaved objects.
- The implementation is deliberately labelled `UI Preview` and performs no
  hardware writes. It therefore does not claim a live `Connected` state.

The complete source and implementation were placed side by side at the same
1280 × 800 viewport. Both full views remained readable, so no cropped
comparison was needed. Controls and accessibility were inspected separately in
the running native application.

## Iteration history

### Pass 1

- **P2 · hierarchy/state ownership:** Modified state was visually separated
  from its selector and scoped actions. The Effects area also used a two-column
  layout unlike the selected console hierarchy.
- **Fix:** `ObjectHeader` now groups selector, state, Save, and overflow in one
  object-owned row. The 1280-wide Effects section now uses one precise console
  column. The EQ curve and vertical rhythm were adjusted to the selected
  reference.
- **Result:** post-fix 1280 × 800 comparison preserved the selected hierarchy
  and made the two independent save targets immediately understandable.

### Pass 2

- **P2 · responsive footer:** at 1024 × 680 the compact faceplate hid setup and
  unsaved-state context.
- **Fix:** the compact footer now retains device, output, volume,
  `My profile · SHP Last`, and the unsaved count while dropping lower-priority
  format text.
- **Result:** no horizontal scrolling, overlap, or clipped primary action at
  1024 × 680. The icon rail, graph, object actions, and faceplate remain usable.

## Final rubric

### Fidelity

- **Layout and spacing:** sidebar, workspace order, two object-owned headers,
  graph, effect console, and full-width hardware faceplate match the selected
  composition. The flatter native section surfaces are a minor visual
  difference, not a hierarchy change.
- **Typography and content:** system sans, tabular audio values, page/section
  hierarchy, explicit object terminology, and modified-state explanations are
  coherent and readable.
- **Colors and tokens:** semantic dark tokens provide cyan active state, violet
  focus, amber modified state, green success, and red error without relying on
  decorative gradients.
- **Imagery and icons:** the target contains no required photographic asset.
  Native Qt-compatible icons replace concept icons without Creative-owned
  artwork. Their shape differs slightly from the visual concept.
- **Responsiveness:** verified at 1280 × 800 and 1024 × 680. X11 startup was
  smoke-tested in addition to native Wayland.
- **Interactions:** navigation, output selection, mute, master volume,
  Direct Mode preview, EQ/effect modification, and separately scoped Save
  actions work against deterministic preview state.

### Accessibility

- Interactive controls expose visible labels and accessible names.
- Keyboard-focus styling is separate from selection color.
- Sliders, switches, buttons, and tooltips were present in the accessibility
  tree.
- Primary controls remain available in the 1024 × 680 compact layout.
- High gain is disabled in the preview; volume starts at the project-safe 20%.

## Remaining P3 differences

1. Native Qt icon shapes and weights differ slightly from the generated
   concept. This preserves platform-native implementation and does not reduce
   comprehension.
2. Section surfaces are flatter and less card-like than the concept. The
   selected grouping and hierarchy remain intact.
3. `UI Preview` replaces `Connected`, and product imagery is omitted, because
   Phase 2 has no live daemon/device authority. Claiming otherwise would be
   misleading.

There are no remaining P0, P1, or P2 findings for the Phase 2 visual shell.

## Phase 3 live-state addendum

The selected shell now consumes typed state from the separate `ae5d` user
service. Native Wayland accessibility inspection confirmed the
physical AE-5 as connected with Headphones, Medium gain, 20% volume, unmuted,
and S16LE at 96 kHz. Output controls expose the qualified-kernel route block,
while gain and Direct Mode remain disabled with precise explanations.

Master volume and mute are now the only live writes. Both are typed D-Bus
methods backed by exact AE-5 PipeWire targeting, checked readback, rollback,
and structured daemon events. The native QML controls passed a 20% → 19% →
20% accessibility-value round trip and a mute → unmute button round trip. The
final state was 20% and unmuted; no ALSA, route, gain, Direct Mode, or OutFX
control changed.

The application was launched with `ae5d` absent, showing `Daemon unavailable`,
no format, an unavailable volume value, disabled controls, and retained
last-confirmed display state. Starting the matching daemon binary restored the
exact live values and re-enabled only volume and mute on the periodic refresh
without restarting the GUI.

## Phase 4 catalog addendum

The mock selector arrays have been replaced by a typed `ae5d` sound-object
catalog. Software tests verify that every one of the 33 embedded Command
profiles produces an independent Effects object and EQ object, combined
Windows imports split without cross-modification, route-specific user profiles
do not leak to another output, and incomplete EQ data cannot suppress a valid
Effects object.

An isolated D-Bus integration smoke returned the personal `My profile` and
`SHP Last` objects plus representative factory entries such as `Gaming`, then
ran the Qt application for eight seconds without a QML type, binding, or load
error. Native Wayland accessibility inspection confirmed that the selected
personal objects populate the real enhancement values and ten-band curve.
Selectors and values are labelled `Preview`; profile editors are disabled
with an explanation until checked apply/save transactions exist. The footer
separates `Device live` from `profiles preview`, and hidden Save/Review actions
are removed from keyboard interaction.

This check performed no hardware write. The final physical state remained
Headphones, Medium gain, S16LE at 96 kHz, 20%, and unmuted.
