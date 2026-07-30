# Hover, OutFX/EQ, and sample-rate audit

This pass covers the concrete interaction defects reported against the Qt/QML
frontend on 2026-07-30. It was exercised with the deterministic Wayland QA
fixture at 1280 × 800, so the screenshots do not perform hardware writes.

## Results

| Step | Result | Evidence |
| --- | --- | --- |
| Playback sample-rate policy | Healthy. The picker exposes Automatic, 48 kHz, and 96 kHz. Selecting 48 kHz updates both the policy and transport-format preview. | `01-playback-rate-picker-after.png` |
| Sidebar hover | Healthy. Hovered inactive destinations retain high-contrast text and tinted icons, with a pointing-hand cursor. The active destination remains distinct. | `02-sidebar-hover-after.png` |
| Combo-box hover | Healthy. Popup rows use explicit foreground/background tokens, and the hovered row keeps white text and a pointing-hand cursor. | `03-dropdown-hover-after.png` |
| OutFX with software EQ | Healthy in the state model and EQ activation path. OutFX no longer disables software EQ; Direct Mode and actual PipeWire graph conflicts still do. | Rust unit tests |
| Minimum desktop width | Healthy. At 1024 × 680 content remains scrollable, the sidebar collapses to an icon rail, and the sample-rate picker remains usable. | `04-playback-1024x680-after.png` |

## Safety boundary

The sample-rate selector uses the existing guarded Rust transition: mute the
AE-5, reopen only the AE-5 playback path, verify the negotiated S16 transport,
and restore the previous policy on failure. The physical transition was not
invoked during this visual audit.

The change does not relax the separate protection around raw hardware OutFX
transitions, which previously produced unstable card state. It only removes the
incorrect UI/backend assumption that an already active OutFX state and the
managed software EQ cannot coexist.
