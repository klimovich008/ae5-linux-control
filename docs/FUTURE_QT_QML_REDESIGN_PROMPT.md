# AE5 Control future Qt/QML redesign prompt

Status: **design exploration only**

Do not implement or scaffold this redesign until:

1. the user confirms that the core audio functions work as intended; and
2. the user selects one of the three Product Design directions.

Until then, GUI work is limited to functional and safety checks: the current
GUI must open, expose the supported operations, report failures, preserve
confirmed hardware state, and keep the Rust backend reusable. GUI performance
optimization and visual restructuring are explicitly deferred.

The prompt below is the authoritative brief for the future redesign.

---

Design a Linux desktop control application for the Creative Sound BlasterX
AE-5 PCIe sound card. The working product name is “AE5 Control.”

This application is an open Linux replacement for the useful parts of Creative
Sound Blaster Command. It is a dedicated audio-hardware control center, not a
generic Linux sound settings panel, a DAW, a music player, a mobile interface
or a browser dashboard.

The application should make the AE-5 genuinely comfortable to configure on
Linux while improving the hierarchy, clarity and reliability of the original
Windows application.

## Production implementation

The final application will use:

- Qt 6
- Qt Quick and QML
- Qt Quick Controls 2
- QtQuick.Layouts
- QtQuick.Shapes for the equalizer curve, level meters and speaker diagrams
- A custom QML theme based on Qt Quick Controls Basic
- Rust for all hardware, profile and state-management logic
- CXX-Qt for narrowly scoped Rust-to-QML view models
- A separate Rust user daemon called `ae5d`
- A typed D-Bus interface between the UI and daemon
- ALSA controls as the primary AE-5 hardware interface
- PipeWire only for stream routing, software processing and session-level
  audio information
- udev for device discovery
- systemd user services for background state restoration

Do not design around HTML, CSS, JavaScript, React, Svelte, Electron, Tauri,
WebKit, GTK, libadwaita or browser-specific behavior.

Do not require Qt Charts or Qt Graphs. Equalizer and meter graphics must be
realistically implementable using QtQuick.Shapes or a focused custom
QQuickItem.

The design must remain practical to implement as reusable QML components.
Avoid visuals that would require a custom game engine, 3D renderer, heavy
shaders or hundreds of bespoke drawing primitives.

## Target window

Design a desktop application, not a mobile application.

Use:

- Default canvas: 1280 × 800
- Minimum supported window: approximately 1024 × 680
- Comfortable wide layout: up to approximately 1600 × 1000
- Support Linux display scaling from 100% through 200%
- No browser chrome
- No monitor or laptop mockup
- No decorative operating-system frame
- Use the application content as the complete image

The application must work under both Wayland and X11 and should feel
appropriate on GNOME, KDE Plasma, Cinnamon and other Linux desktops without
copying one desktop environment’s visual language.

## Core user goals

The user should be able to:

1. Confirm that the AE-5, kernel driver and required firmware are available.
2. See the currently active output: speakers, headphones or digital output.
3. Switch outputs without searching through multiple pages.
4. Select, modify, save and restore audio profiles.
5. Configure supported enhancement parameters.
6. Adjust a precise ten-band equalizer.
7. Configure speaker layout and channel calibration.
8. Control playback and recording levels.
9. Understand when Direct Mode disables or bypasses enhancements.
10. Configure supported lighting without letting RGB dominate the audio
    interface.
11. Diagnose unsupported controls, permission problems and driver limitations.
12. Recover from a failed operation without restarting the application.

## Information architecture

Use a persistent left navigation sidebar on normal desktop widths.

Suggested top-level destinations:

- Overview
- Sound
- Equalizer
- Playback
- Recording
- Mixer
- Lighting
- Device
- Settings

Group related destinations visually. Do not treat every item as equally
important.

“Device” should contain driver, firmware, ALSA control availability, hardware
identifiers, diagnostic information and troubleshooting actions. It must not
look like a developer-only debug console.

At very narrow window widths, the sidebar may collapse to an icon rail with
tooltips, but the application should remain desktop-first.

## Persistent control area

Create a compact control area that remains available while navigating between
pages.

It should contain:

- Current output: Speakers, Headphones or Digital
- Master volume
- Mute
- Active profile
- Device-connected state
- A subtle Applying, Saved, Modified or Error state

This area can be a restrained footer, header section or compact control strip.
It must not consume excessive vertical space.

Switching output should be deliberate enough to avoid mistakes but should not
require a confirmation dialog under normal conditions.

## Primary screen for this ideation round

Design the main “Sound” or “Overview” screen only.

The visible screen should include:

- Sound BlasterX AE-5 name and connected state
- Current output
- Current audio profile
- Master volume and mute
- A useful equalizer response preview
- A compact profile selector
- Supported sound-enhancement controls
- Save, reset or revert behavior for a modified profile
- A clear indication when Direct Mode conflicts with enhancements
- One small device or driver status area
- Navigation toward the rest of the application

Possible enhancement controls include:

- Surround
- Crystalizer
- Bass
- Smart Volume
- Dialog enhancement

Treat these names as capability-dependent hardware features, not as controls
that are guaranteed to exist.

Do not show every future feature on the main page. Recording, detailed speaker
calibration, per-channel mixer controls, complete lighting configuration and
full diagnostics belong on their own pages.

## Equalizer interaction

The equalizer is a central interaction, not decoration.

Design a ten-band equalizer with frequencies such as:

31 Hz, 62 Hz, 125 Hz, 250 Hz, 500 Hz, 1 kHz, 2 kHz, 4 kHz, 8 kHz and 16 kHz.

Requirements:

- Clearly show the 0 dB reference.
- Show gain values and units.
- Support direct dragging of points.
- Support keyboard adjustment.
- Provide an accessible numeric alternative.
- Make reset and flat-state actions obvious.
- Show the resulting curve clearly.
- Avoid fake spectrum data or invented real-time analysis.
- Do not use a generic business chart appearance.
- Do not put the graph inside several nested cards.
- Make it implementable using QtQuick.Shapes.

## Control design

Prefer precise horizontal sliders, segmented selectors, switches and numeric
fields.

Do not turn every parameter into a circular dial. Dials may be used only where
they improve comprehension and still provide a precise numeric value.

Every control must have:

- A visible label
- A current value
- A meaningful unit where applicable
- A keyboard interaction
- A disabled state
- An explanation when unavailable
- A reliable reset path

Changes may be applied immediately, but the interface must show modified state
and provide Revert or Reset.

Conflicting controls must be disabled with an explanation. For example,
enabling Direct Mode may disable equalizer and enhancement controls.

## Capability-driven design

The Linux implementation may expose fewer controls than the Windows
application at first.

The UI must handle:

- Fully supported feature
- Partially supported feature
- Unsupported by this card revision
- Unsupported by the current kernel driver
- Firmware unavailable
- Permission denied
- Device disconnected
- Device busy
- Value being applied
- Write failed and previous value restored
- Hardware value changed outside the application

Do not present unsupported functions as ordinary inactive controls with no
explanation.

A user should be able to understand whether a missing feature is caused by:

- The physical AE-5 model
- The Linux kernel driver
- Missing firmware
- Permissions
- The current output mode
- A conflict such as Direct Mode
- An implementation that has not yet been completed

## Required system states

Establish visual and interaction patterns for:

- AE-5 connected and fully operational
- No compatible card detected
- Driver loaded with partial capabilities
- Required firmware missing
- Permission failure
- Speakers active
- Headphones active
- Digital output active
- Direct Mode active
- Profile modified
- Profile saved
- Applying changes
- Hardware write failed
- Microphone disconnected
- Lighting unsupported
- Daemon unavailable

Do not display all states simultaneously. The selected screen should
demonstrate one healthy state and, optionally, one restrained capability
notice.

## Visual direction

The application should feel like a precise, modern Linux audio instrument.

It should be:

- Dark-first, but fully compatible with a light theme
- Technical without becoming intimidating
- Premium without looking proprietary or bloated
- Calm during normal use
- Precise enough for audio configuration
- Consistent across Linux desktop environments
- Keyboard-accessible
- High contrast
- Clear at fractional scaling
- Suitable for long-term daily use

Use the system UI font or a realistic system-sans equivalent. Do not make a
bundled custom font essential to the identity.

A restrained cyan or teal accent is appropriate. A secondary violet accent may
be used sparingly. Warning and destructive colors must remain semantic rather
than decorative.

Use subtle technical geometry only where it improves grouping or orientation.

## Avoid

Do not clone the current Sound Blaster Command interface.

Avoid:

- Overcrowded profile carousels
- Rows of oversized circular gauges
- Gamer HUD styling
- Constant RGB gradients
- Cyberpunk visual noise
- Glassmorphism
- Excessive transparency
- Nested cards
- A dashboard made entirely from disconnected cards
- Tiny labels
- Fake audio visualizers
- Decorative waveform backgrounds
- Huge product logos
- Diagonal panels that reduce usable space
- Unlabeled icons
- Color as the only indication of state
- A raw terminal
- Generic shell-command controls
- A Windows Settings clone
- An Android or mobile layout
- Excessive animation

Lighting configuration may use richer color, but the rest of the application
should not visually imitate an RGB effect.

## Safety and reliability

The GUI communicates with a background Rust service. Reflect that architecture
in the interaction design.

- Never imply that the application must run as root.
- Do not expose arbitrary command execution.
- Show hardware-write failures clearly.
- Preserve the previously confirmed value after a failed write.
- Distinguish Reset profile, Reset page and Reset device settings.
- Confirm only genuinely destructive reset operations.
- Do not interrupt audio merely because the user navigates between pages.
- Show when an operation may temporarily mute or restart the device.
- Keep diagnostic details available without placing them on the main dashboard.

## QML component thinking

Ensure the design can be decomposed into components resembling:

- AppShell
- NavigationSidebar
- DeviceHeader
- PersistentOutputBar
- OutputSelector
- ProfileSelector
- ProfileModifiedIndicator
- EnhancementControl
- EqualizerGraph
- EqualizerBandControl
- MasterVolumeControl
- DeviceStatusBadge
- CapabilityNotice
- InlineError
- ApplyingIndicator
- EmptyDeviceState

Use standard Qt Quick layouts and controls wherever possible. Custom drawing
should be concentrated in EqualizerGraph, compact meters and the
speaker-calibration diagram.

## Ideation deliverable

Generate exactly three independent high-fidelity visual directions for the
primary 1280 × 800 desktop screen.

Each direction must be a separate image rather than three variations placed
into one image.

Make the directions meaningfully different:

1. Precision-native direction: calm, restrained and highly legible, with a
   strong Linux desktop-tool character.
2. Studio-console direction: more focused on equalization and audio
   adjustment, borrowing hierarchy from professional audio tools without
   becoming a DAW.
3. Restrained Sound Blaster direction: a subtle technical and
   gaming-influenced identity using controlled cyan or violet accents, without
   RGB clutter or gamer-HUD styling.

Each image must show:

- Connected Sound BlasterX AE-5
- Headphones or speakers selected
- One active profile
- Master volume
- Equalizer curve
- Several supported enhancement controls
- Clear navigation
- A saved or modified profile state
- One small driver or capability status
- No device frame or browser chrome

Under each direction, provide:

- A short rationale
- The principal hierarchy
- The expected reusable QML components
- Any implementation risk
- How it adapts down to 1024 × 680

Do not implement or scaffold the QML application yet.

After producing all three directions, stop and ask the user to select one
before proceeding to component specification or image-to-QML implementation.
