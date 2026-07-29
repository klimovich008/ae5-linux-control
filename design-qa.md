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

## Phase 4 profile-object addendum

The mock selector arrays have been replaced by a typed `ae5d` sound-object
catalog. Software tests verify that every one of the 33 embedded Command
profiles produces an independent Effects object and EQ object, combined
Windows imports split without cross-modification, route-specific user profiles
do not leak to another output, and incomplete EQ data cannot suppress a valid
Effects object.

An isolated D-Bus integration smoke returned the personal `My profile` and
`SHP Last` objects plus representative factory entries such as `Gaming`, then
ran the Qt application without a QML type, binding, or load error. Native
Wayland accessibility inspection confirmed that the selected personal objects
populate the real enhancement values and ten-band curve.

The completed persistence pass verified independent Rust-owned Effects and EQ
drafts, object-scoped Modified state, selector locking while modified, Revert,
Save, Save as, and independent unsaved counts. A native Wayland Save as from
the combined `My profile` created an Effects-only JSON file through typed D-Bus,
preserved the hidden X-Bass crossover value, omitted every EQ control, changed
the catalog from 35 to 36 Effects profiles, and remained discoverable after
restarting the QML process. The daemon logged a verified
`effects-save-as` completion. Tests also cover atomic replacement, strict value
validation, duplicate-name refusal, route isolation, and preservation of
unrelated files.

This check performed no hardware write. The final physical state remained
Headphones, Medium gain, S16LE at 96 kHz, 20%, and unmuted.

## Phase 5 checked-EQ addendum

The QML screen now separates a preset's Saved/Modified state from the live
software-EQ graph state. Apply and Disable are explicit typed daemon
transactions rather than side effects of selecting or saving a preset.
Applying is unavailable with a precise explanation while OutFX, Direct Mode,
the selected preset, daemon state, or exact PipeWire target makes the
transaction unsafe.

The guarded physical OutFX-on test confirmed the failure path: the request was
blocked before a write, the managed PipeWire configuration remained absent,
no owned runtime graph appeared, and the exact 20% unmuted sink state remained
unchanged. The user-observed OutFX-off apply/change/disable/restart cycle is
still required before Phase 5 is complete.

## Phase 7 accessibility and scaling addendum

The production QML path now selects Qt Quick Controls Basic before constructing
the application and maps all custom surfaces and states through semantic dark
or light tokens. Contrast calculations passed for normal primary/secondary
text and cyan, violet, green, amber, and red semantic status colors. Disabled
controls remain visibly distinct and expose the reason they are unavailable.

Native Wayland inspection verified:

- 1024 × 680: 72 px icon rail, 908 px workspace, no horizontal overflow,
  vertical scrolling, and the compact persistent hardware faceplate;
- 1280 × 800: the selected default composition with full sidebar and one
  object-owned console column;
- 1600 × 1000: full sidebar, expanded graph, two-column enhancement use, and
  the same singular hardware faceplate;
- dark and explicit `--light` launches without QML type, binding, or load
  errors;
- AT-SPI status, grouping, chart, slider, checkbox, button, and list-item
  semantics with meaningful names, descriptions, current values, and disabled
  reasons;
- visible keyboard-only focus styling on custom navigation, save, output, and
  EQ controls, plus object-scoped Save/Save as and faceplate Review paths;
- an X11 startup smoke that remained alive for the complete six-second window
  with no diagnostic output.

Qt's AT-SPI adapter emits benign `GetNselections`/`GetText` warnings when the
inspection tool queries interfaces Qt does not implement. They do not indicate
a QML load error or an application failure. Automated focus-order assertions,
close-with-unsaved handling, 125%–200% scale checks, and complete injected
failure-state coverage remain open Phase 7 work.

This pass invoked no audio action. It did not change volume, mute, route, gain,
OutFX, Direct Mode, ALSA controls, software-EQ configuration, or the runtime
PipeWire graph.
