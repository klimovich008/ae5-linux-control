# Opus 5 polish audit — 2026-07-30

This audit polishes the selected AE5 Control direction without changing the
core backend/UI contract. The review focused on alignment, color hierarchy,
disabled and modified states, responsive footer behavior, and the navigation
sidebar.

## Review provenance

The second-opinion review was run with the local Claude CLI using
`--model claude-opus-5 --effort max`. The returned metadata identified both
`canonicalModel` and `modelUsage` as `claude-opus-5`; no fallback model or
permission denial was reported. Claude was given read-only access to the
current screenshots and relevant QML/theme files.

The review was treated as design input rather than applied mechanically. In
particular, the proposed disabled-text token was raised to `#879CA9` after a
contrast check showed that the lower value would not reach 4.5:1 against the
raised dark surface.

## Accepted findings

- Retune the dark accent from high-saturation cyan to a calmer audio-tool cyan.
- Increase separation between the page, normal surfaces, and raised surfaces.
- Stop rendering disabled controls on a brighter surface than enabled ones.
- Make checked-but-disabled controls visually distinct from checked controls.
- Give modified-state review actions an explicit amber treatment.
- Align Smart Volume's slider with the other enhancement sliders while keeping
  its Night/Loud endpoint labels.
- Strengthen the selected sidebar item and the sidebar/page boundary.
- Normalize sidebar glyphs by their visible SVG bounds, then center every icon
  and label in a fixed-height navigation row.
- Reserve equal horizontal padding at both ends of the equalizer graph so the
  `16k Hz` label does not touch the card edge.
- Keep non-alert state text neutral while retaining the semantic state dot.
- Reduce footer pressure at 1280 px and keep the compact 1024 px footer usable.

## Evidence

- [`01-current-ready-window.png`](01-current-ready-window.png) — pre-polish
  ready state with window decoration.
- [`02-current-ready-1280x800.png`](02-current-ready-1280x800.png) — pre-polish
  ready state at the design viewport.
- [`03-reference-1280x800.png`](03-reference-1280x800.png) — selected visual
  reference.
- [`04-reference-vs-current-ready.png`](04-reference-vs-current-ready.png) —
  pre-polish comparison.
- [`05-current-both-modified-window.png`](05-current-both-modified-window.png)
  and [`06-current-both-modified-1280x800.png`](06-current-both-modified-1280x800.png)
  — pre-polish modified state.
- [`07-polished-ready-1280x800.png`](07-polished-ready-1280x800.png) — final
  dark ready state.
- [`08-polished-both-modified-1024x680.png`](08-polished-both-modified-1024x680.png)
  — final minimum-size compact sidebar and modified-state footer.
- [`09-polished-ready-1600x1000.png`](09-polished-ready-1600x1000.png) — final
  comfortable wide layout.
- [`10-polished-light-ready-1280x800.png`](10-polished-light-ready-1280x800.png)
  — final light-theme check.
- [`11-reference-vs-polished-ready.png`](11-reference-vs-polished-ready.png) —
  matched-viewport reference comparison.
- [`12-before-vs-polished-ready.png`](12-before-vs-polished-ready.png) —
  matched-viewport before/after comparison.
- [`13-polished-direct-mode-1280x800.png`](13-polished-direct-mode-1280x800.png)
  — active Direct Mode with enhancement controls visibly bypassed.
- [`14-polished-write-failed-1280x800.png`](14-polished-write-failed-1280x800.png)
  — hardware-write failure and retained-value recovery state.

## Acceptance checks

- Release Rust/Qt/QML build.
- QML keyboard focus-order audit in ready and both-modified states.
- QML accessibility/state smoke across all ten QA scenarios.
- Dark theme at 1024 × 680, 1280 × 800, and 1600 × 1000.
- Light theme at 1280 × 800.
- Ready and both-modified profile states.
- Direct Mode and hardware-write-failed states.
- Sidebar icon optical-size and vertical-center checks in compact and expanded
  modes.

The screen remains a QA-mode view in these images: preview data is local and
hardware writes are disabled.
