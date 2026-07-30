# Qt/QML multi-page design audit — 2026-07-30

## Scope

This audit covers the native Qt 6/QML shell after the remaining navigation
destinations were implemented. It is a deterministic UI review: every capture
uses `--qa-state` fixtures, so no ALSA, PipeWire, OutFX, route, gain, or volume
write can occur.

The selected Sound-screen source remains
[`../ae5-control-sound-selected-v2.png`](../ae5-control-sound-selected-v2.png).
The new pages extend its semantic theme, persistent hardware faceplate, control
ownership, and capability-driven status model; they do not invent additional
backend support.

## Captured journey

1. `01-sound-before-1280x800.png` — accepted pre-fix Sound screen.
2. `02-sound-sidebar-before.png` — focused evidence of the navigation row
   alignment defect.
3. `page-overview-1280x800.png` — device, output, saved-object, EQ, and service
   summary.
4. `page-sound-modified-1280x800.png` — both sound objects modified, matching
   the selected reference state.
5. `page-equalizer-1280x800.png` — detailed ten-band preset editing.
6. `page-playback-1280x800.png` — authoritative playback path and guarded
   speaker capabilities.
7. `page-recording-1280x800.png` — capture capabilities with unavailable
   writes explained.
8. `page-mixer-1280x800.png` — read-only channel inventory without duplicating
   master-volume ownership.
9. `page-lighting-1280x800.png` — restrained five-LED preview with writes
   explicitly pending.
10. `page-device-1280x800.png` — exact card identity, daemon state, format, and
    driver safety boundaries.
11. `page-settings-1280x800.png` — working session theme control and guarded
    service/maintenance settings.
12. `compact-*-1024x680.png` — minimum-window checks for the densest and most
    representative destinations.
13. `page-sound-light-1280x800.png` — semantic light-theme check.
14. `design-qa-sound-full.png` — selected source and implemented Sound screen
    at the same normalized 1280 × 800 viewport and modified state.
15. `design-qa-sidebar-focus.png` — focused source/implementation sidebar
    comparison.

## Findings and resolutions

### P1 — navigation rows did not own their requested height

`ItemDelegate` retained its default top and bottom padding while the icon and
label requested the complete 44 px control height. The content pair aligned to
itself but overflowed below the row, making Sound and the other labels appear
low.

Resolution: both primary and utility delegates now explicitly use zero
vertical padding. AT-SPI reports identical 44 px bounds for each delegate and
its label, and the post-fix screenshots show the icon/label pair centered.

### P1 — non-Sound destinations looked interactive but did not navigate

Resolution: all nine destinations now emit a typed page key into one
`StackLayout`. Keyboard focus, pointer cursor, selected state, compact tooltip,
and accessible descriptions are consistent for every row.

### P1 — pages could imply unsupported hardware writes

Resolution: Overview and Device use real `AppState` values. Equalizer edits the
same Rust-owned preset as Sound. Playback keeps output, gain, and volume in the
single persistent faceplate. Recording, extra mixer channels, lighting writes,
speaker configuration, and reset actions remain visibly read-only, deferred,
or guarded with a reason until their typed `ae5d` contracts exist.

### P2 — Overview cards compressed their detail text

Resolution: the cards now reserve 170 px and a two-line detail region. The
device summary uses the shorter canonical product name.

### P2 — minimum-width density

Resolution: at 1024 × 680 the sidebar becomes an icon rail, the faceplate
shortens output labels, and page content scrolls vertically. No horizontal
overflow or overlapping interactive control was found.

## Final audit result

Passed for the multi-page UI slice. Remaining work is backend integration and
physical acceptance for controls currently labelled read-only or guarded; it
is not a visual-navigation blocker.

Verification: release build, strict Qt/QML-target Clippy, ShellCheck, 147
workspace tests, ten deterministic state smokes, nine destination smokes,
focus-order audit, native Wayland startup, and dark/light visual inspection.
