//! Shared visual system for the AE-5 GTK 4 application.
//!
//! The stylesheet is deliberately one place: the pages compose widgets and
//! attach CSS classes, and every colour, size and state lives here so the
//! look can be changed without touching a control path.

use gtk::gdk::Display;

/// Install the application stylesheet on the default display.
pub fn install_css() {
    let Some(display) = Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "
        window {
            background: #111827;
            color: #edf2f7;
        }
        .application-shell {
            background: #1d1c2e;
        }
        .sidebar-panel {
            min-width: 232px;
            background: #162040;
            border-right: 1px solid alpha(#ffffff, 0.08);
        }
        .sidebar-brand {
            padding: 20px 18px 16px 18px;
            background: #0d1828;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        .sidebar-title {
            color: #21c6d4;
            font-size: 19px;
            font-weight: 700;
            letter-spacing: 0.6px;
        }
        .sidebar-device {
            color: #8ca0b4;
            font-size: 11px;
        }
        .sidebar-footer {
            padding: 14px 16px;
            color: #7890a5;
            background: #101a2d;
            border-top: 1px solid alpha(#ffffff, 0.08);
            font-family: monospace;
            font-size: 10px;
            font-weight: 700;
        }
        .main-panel {
            background: linear-gradient(160deg, #241f3d 0%, #1d1c2e 46%, #191a2a 100%);
        }
        .hero {
            min-height: 52px;
            padding: 12px 26px;
            background: #0d1828;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        .hero-kicker, .error-kicker {
            color: #22c7d4;
            font-family: monospace;
            font-size: 11px;
            font-weight: 700;
        }
        .hero-title {
            font-size: 16px;
            font-weight: 600;
            letter-spacing: 0.2px;
        }
        .dim-label { color: #98a7b7; }
        .status-pill {
            background: alpha(#22c7d4, 0.10);
            color: #57dce5;
            border: 1px solid alpha(#22c7d4, 0.38);
            border-radius: 3px;
            padding: 6px 10px;
            font-family: monospace;
            font-size: 10px;
            font-weight: 700;
        }
        .operation-status {
            color: #9fb0c0;
            font-size: 11px;
            padding: 0;
        }
        .status-rail {
            min-height: 34px;
            padding: 5px 14px;
            background: #0d1828;
            border-top: 1px solid alpha(#ffffff, 0.08);
        }
        /* The signal path. Colour here means one thing only: whether signal
           passes. Nothing else in the interface may use these three hues. */
        .signal-path {
            padding: 7px 14px;
            background: #14161a;
            border-top: 1px solid alpha(#ffffff, 0.07);
        }
        .signal-path-blocked {
            background: #23161a;
            border-top: 1px solid alpha(#e2564f, 0.55);
        }
        .path-stage { padding: 0 16px 0 0; }
        .path-stage-label {
            color: #7c8590;
            font-size: 10px;
            letter-spacing: 0.7px;
            text-transform: uppercase;
        }
        .path-stage-reading {
            font-family: monospace;
            font-size: 12px;
            font-weight: 600;
        }
        .path-stage-mark { font-size: 9px; }
        .stage-passing .path-stage-reading,
        .stage-passing .path-stage-mark { color: #6fd08c; }
        .stage-attention .path-stage-reading,
        .stage-attention .path-stage-mark { color: #e8b064; }
        .stage-blocked .path-stage-reading,
        .stage-blocked .path-stage-mark { color: #f0736b; }
        .stage-unknown .path-stage-reading,
        .stage-unknown .path-stage-mark { color: #8b949e; }
        .path-link {
            padding: 0 14px 0 0;
            color: #4a525c;
            font-size: 12px;
        }
        .status-mark {
            color: #eef3f7;
            font-size: 14px;
            font-weight: 800;
        }
        .output-state {
            color: #49d5df;
            font-family: monospace;
            font-size: 10px;
            font-weight: 700;
        }
        .footer-output-selector {
            border: 1px solid alpha(#ffffff, 0.14);
            border-radius: 2px;
        }
        .footer-route-label {
            padding: 0 8px;
            color: #8da0b2;
            background: #162238;
            font-family: monospace;
            font-size: 9px;
            font-weight: 700;
        }
        .footer-output-choice {
            min-width: 72px;
            min-height: 26px;
            padding: 2px 8px;
            color: #aab7c4;
            background: #252a38;
            border: 0;
            border-left: 1px solid alpha(#ffffff, 0.10);
            border-radius: 0;
            font-size: 10px;
        }
        .footer-output-choice:checked {
            color: #f8fbfc;
            background: #147e88;
            box-shadow: inset 0 -2px #35d3de;
        }
        .operation-ok { color: #72d9c0; }
        .operation-error, .warning-label, .warning-value { color: #ffb4a9; }
        .unavailable-pill {
            padding: 6px 10px;
            color: #ffd19a;
            background: alpha(#ffad42, 0.08);
            border: 1px solid alpha(#ffbd66, 0.30);
            border-radius: 3px;
            font-family: monospace;
            font-size: 10px;
            font-weight: 700;
        }
        stacksidebar.navigation-sidebar,
        stacksidebar.navigation-sidebar scrolledwindow,
        stacksidebar.navigation-sidebar viewport,
        stacksidebar.navigation-sidebar list,
        stacksidebar.navigation-sidebar .view {
            background: #162040;
        }
        .navigation-sidebar {
            padding: 12px 0;
        }
        .navigation-sidebar row {
            min-height: 46px;
            margin: 0;
            padding: 0 14px;
            background: #162040;
            border-radius: 0;
            border-left: 3px solid transparent;
            font-size: 13px;
        }
        .navigation-sidebar row:hover { background: alpha(#ffffff, 0.05); }
        .navigation-sidebar row:focus-visible {
            box-shadow: inset 0 0 0 2px #57dce5;
        }
        .navigation-sidebar row:selected {
            background: alpha(#22c7d4, 0.13);
            color: #4fdbe5;
            border-left: 3px solid #22c7d4;
            font-weight: 600;
        }
        .settings-header {
            padding: 18px 24px 0 24px;
            background: #25213c;
            border-bottom: 1px solid alpha(#ffffff, 0.07);
        }
        .page-tabs button {
            min-height: 36px;
            padding: 6px 18px;
            color: #98a7b7;
            background: transparent;
            border: 0;
            border-radius: 0;
            border-bottom: 2px solid transparent;
            font-size: 13px;
        }
        .page-tabs button:hover { color: #c6d3de; }
        .page-tabs button:checked {
            color: #35d3de;
            border-bottom: 2px solid #22c7d4;
            font-weight: 600;
        }
        .profile-page, .control-page { padding: 24px 30px 28px 30px; }
        .page-title {
            margin-bottom: 2px;
            font-size: 26px;
            font-weight: 450;
            letter-spacing: 0.2px;
        }
        .mixer-section {
            margin-top: 4px;
            color: #eef3f7;
            font-size: 14px;
            font-weight: 700;
        }
        .equalizer-bands {
            min-height: 230px;
            padding: 16px 18px 12px 18px;
            background: #242238;
            border: 1px solid alpha(#c1c7d0, 0.10);
            border-radius: 2px;
        }
        .equalizer-frequency {
            color: #9fb0c0;
            font-family: monospace;
            font-size: 11px;
            font-weight: 700;
        }
        .profile-carousel {
            padding-bottom: 4px;
            background: transparent;
        }
        .sound-profile-card {
            padding: 12px;
            background-image: linear-gradient(145deg, #292747, #1b2e45);
            border: 1px solid alpha(#aebbd0, 0.16);
            border-radius: 3px;
        }
        .sound-profile-card:hover {
            background-image: linear-gradient(145deg, #302e54, #1f384f);
            border-color: alpha(#4dd8e1, 0.42);
        }
        .sound-profile-card-active {
            border: 2px solid #21c6d4;
            background-image: linear-gradient(145deg, #32305a, #184253);
        }
        .profile-card-kicker {
            color: #52d8e1;
            font-family: monospace;
            font-size: 9px;
            font-weight: 800;
        }
        .profile-card-title {
            color: #f4f7fa;
            font-size: 15px;
            font-weight: 750;
        }
        .profile-card-action {
            min-height: 24px;
            padding: 3px 8px;
            font-size: 10px;
        }
        .profile-card-active-label {
            color: #55dce5;
            font-family: monospace;
            font-size: 9px;
            font-weight: 800;
        }
        .effect-card {
            padding: 12px;
            background: #242238;
            border: 1px solid alpha(#c1c7d0, 0.12);
            border-top: 2px solid alpha(#22c7d4, 0.50);
            border-radius: 3px;
        }
        .effect-card:hover {
            background: #292640;
            border-top-color: #2fd0dc;
        }
        .effect-card scale.horizontal {
            min-width: 116px;
        }
        .effect-scale-note {
            color: #98a7b7;
            font-size: 10px;
        }
        .effect-card-title {
            color: #edf2f7;
            font-size: 13px;
            font-weight: 700;
        }
        .effect-dial-value {
            min-width: 52px;
            min-height: 52px;
            padding: 7px;
            color: #f7fbfd;
            background: #1b2940;
            border: 5px solid #22c7d4;
            border-radius: 999px;
            font-size: 18px;
            font-weight: 800;
        }
        .playback-route-note {
            padding: 9px 12px;
            color: #b8c5d0;
            background: alpha(#22c7d4, 0.06);
            border: 1px solid alpha(#22c7d4, 0.18);
            border-left: 3px solid #22c7d4;
            border-radius: 2px;
        }
        .playback-setting-tile {
            min-height: 92px;
            padding: 12px 14px;
            background: #242238;
            border: 1px solid alpha(#c1c7d0, 0.10);
            border-radius: 3px;
        }
        .recording-source-panel {
            padding: 14px 16px;
            background: #242238;
            border: 1px solid alpha(#22c7d4, 0.26);
            border-left: 3px solid #22c7d4;
            border-radius: 3px;
        }
        .profile-card {
            background: #242238;
            border: 1px solid alpha(#c1c7d0, 0.10);
            border-radius: 2px;
            padding: 14px 16px;
        }
        .profile-library-row {
            padding: 8px 0;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        .feature-entry {
            padding: 7px 0;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        .section-index {
            background: alpha(#22c7d4, 0.12);
            color: #4ed6df;
            border: 1px solid alpha(#22c7d4, 0.30);
            border-radius: 2px;
            padding: 3px 7px;
            font-family: monospace;
            font-weight: 700;
        }
        .section-title { font-size: 15px; font-weight: 700; }
        .control-list {
            background: #242238;
            border: 1px solid alpha(#c1c7d0, 0.10);
            border-radius: 2px;
        }
        .gain-stage-notice {
            color: #ffd19a;
            background: alpha(#ffad42, 0.08);
            border: 1px solid alpha(#ffbd66, 0.25);
            border-left: 3px solid #ffad42;
            border-radius: 2px;
            padding: 10px 12px;
        }
        .control-row {
            min-height: 42px;
            padding: 9px 12px;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        .control-row:hover { background: alpha(#ffffff, 0.035); }
        button {
            min-height: 29px;
            padding: 5px 10px;
            border-radius: 2px;
        }
        button.suggested-action {
            background: #147e88;
            color: #ffffff;
            border-color: #22b8c5;
        }
        button.destructive-action {
            color: #ffc3bd;
            background: alpha(#d95c5c, 0.12);
            border-color: alpha(#ff8d83, 0.35);
        }
        button.destructive-action:hover {
            color: #ffffff;
            background: alpha(#d95c5c, 0.28);
        }
        button:focus-visible,
        switch:focus-visible,
        dropdown:focus-visible,
        entry:focus-visible,
        scale:focus-visible {
            outline: 2px solid #57dce5;
            outline-offset: 2px;
        }
        switch {
            min-width: 34px;
            min-height: 18px;
        }
        switch:checked {
            background: #17b9c6;
        }
        scale trough {
            min-height: 4px;
            background: #3d4052;
            border-radius: 0;
        }
        scale highlight {
            background: #20c7d4;
        }
        scale slider {
            min-width: 18px;
            min-height: 18px;
            background: #dbe6ed;
            border: 0;
            border-radius: 999px;
        }
        dropdown, entry {
            background: #343544;
            border-color: alpha(#ffffff, 0.12);
            border-radius: 2px;
        }
        .error-view { padding: 32px; }
        .unavailable-card {
            background: #242238;
            border: 1px solid alpha(#ffbd66, 0.28);
            border-left: 4px solid #ffad42;
            border-radius: 2px;
            padding: 28px;
        }
        .offline-icon {
            color: #ffc06d;
            background: alpha(#ffad42, 0.10);
            border: 1px solid alpha(#ffbd66, 0.24);
            border-radius: 2px;
            padding: 10px;
        }
        .error-kicker { color: #ffc06d; }
        .error-hint {
            color: #c0cdd5;
            margin-top: 4px;
        }
        .error-action {
            margin-top: 8px;
            background: #d98a27;
            color: #111820;
            border-color: #f2ad54;
            font-weight: 700;
        }
        .error-action:hover { background: #ed9f3c; }
        scale.horizontal { min-width: 190px; }
        scale.vertical {
            min-width: 28px;
            min-height: 178px;
        }
        scrollbar slider { min-width: 8px; }
        scrollbar.horizontal slider { min-height: 8px; }
        ",
    );
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
