#[cfg(test)]
use ae5_control::linux_driver_defaults_for;
use ae5_control::{
    Ae5Device, Ae5Lighting, Ae5Mixer, ChannelLevel, ControlError, ControlSnapshot,
    DIRECT_MODE_CONTROL, FeatureSupport, LINUX_DRIVER_DEFAULTS_PRESERVED, Level, NativeRatesConfig,
    ONBOARD_LED_COUNT, PipeWireNode, PipeWireRouteState, Profile, ProfileControl, RgbColor,
    SbCommandImport, SbCommandTarget, ae5_input, ae5_output, ae5_route_state,
    apply_linux_driver_defaults, capture_control_block_reason, direct_mode_block_reason,
    discover_sbcommand_installation, equalizer_band_block_reason, export_library_profile,
    feature_parity, import_discovered_sbcommand_profile_with_report,
    import_sbcommand_profile_with_report, library_profile, native_rates_config,
    playback_switch_block_reason, profile_library, profile_library_directory,
    rename_library_profile, set_ae5_default_input, set_ae5_default_output,
    set_native_rates_enabled, set_saved_led, set_saved_lighting, snapshot_controls,
    validate_linux_driver_defaults,
};
use gtk::prelude::*;
use gtk::{gdk::Display, gio};
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const APP_ID: &str = "io.github.klimovich008.ae5control";
const MAIN_STACK_NAME: &str = "main-navigation";
const PERFORMANCE_PROBE: &str = "AE5_CONTROL_PERFORMANCE_PROBE";
const DIRECT_MODE_DESCRIPTION: &str = "Bypasses CA0132 DSP processing for a stereo hardware path. \
    AE-5 Control briefly suspends PipeWire while switching; use stream or software volume because \
    the card's DSP effects and hardware playback levels are bypassed.";
const EQ_BAND_LABELS: [&str; 10] = [
    "31 Hz",
    "62 Hz (Bass in Command)",
    "125 Hz",
    "250 Hz",
    "500 Hz",
    "1 kHz",
    "2 kHz",
    "4 kHz",
    "8 kHz (Treble in Command)",
    "16 kHz",
];

fn main() -> gtk::glib::ExitCode {
    if std::env::var_os("GSK_RENDERER").is_none() {
        // SAFETY: GTK and worker threads have not started, so no other thread can read the
        // process environment while the static UI selects its lower-memory renderer.
        unsafe { std::env::set_var("GSK_RENDERER", "cairo") };
    }
    let started = Instant::now();
    let performance_probe = std::env::var_os(PERFORMANCE_PROBE).is_some();
    let mut builder = gtk::Application::builder().application_id(APP_ID);
    if performance_probe {
        builder = builder.flags(gio::ApplicationFlags::NON_UNIQUE);
    }
    let application = builder.build();
    application.connect_activate(move |application| {
        build_window(application, started, performance_probe);
    });
    application.run()
}

fn build_window(application: &gtk::Application, started: Instant, performance_probe: bool) {
    install_css();

    let window = gtk::ApplicationWindow::builder()
        .application(application)
        .title("AE-5 Control")
        .default_width(980)
        .default_height(680)
        .build();

    if let Some(card_index) = refresh_window(&window, None)
        && let Err(error) = start_mixer_watch(&window, card_index)
    {
        set_main_status(
            &window,
            false,
            &format!("Live synchronization failed: {error}"),
        );
    }
    window.present();
    if performance_probe {
        start_performance_probe(&window, started);
    }
}

fn start_performance_probe(window: &gtk::ApplicationWindow, started: Instant) {
    let window = window.clone();
    gtk::glib::idle_add_local_once(move || {
        let startup_ms = started.elapsed().as_millis();
        let refresh_started = Instant::now();
        if refresh_window(&window, None).is_none() {
            println!("probe_error=hardware refresh failed");
            let _ = std::io::stdout().flush();
            return;
        }
        gtk::glib::idle_add_local_once(move || {
            println!("startup_ms={startup_ms}");
            println!(
                "control_refresh_ms={}",
                refresh_started.elapsed().as_millis()
            );
            println!("probe_ready=1");
            let _ = std::io::stdout().flush();
        });
    });
}

fn refresh_window(window: &gtk::ApplicationWindow, message: Option<&str>) -> Option<i32> {
    let visible_page = main_stack(window).and_then(|stack| stack.visible_child_name());
    match load_hardware() {
        Ok((device, controls)) => {
            let card_index = device.card_index;
            window.set_child(Some(&content(
                window,
                &device,
                &controls,
                message,
                visible_page.as_deref(),
            )));
            Some(card_index)
        }
        Err(error) => {
            window.set_child(Some(&error_view(&error)));
            None
        }
    }
}

fn load_hardware() -> Result<(Ae5Device, Vec<ControlSnapshot>), String> {
    let device = Ae5Device::discover()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "Supported Sound BlasterX AE-5 was not found".to_owned())?;
    let controls = Ae5Mixer::open(device.card_index)
        .and_then(|mixer| mixer.snapshots())
        .map_err(|error| error.to_string())?;
    Ok((device, controls))
}

fn content(
    window: &gtk::ApplicationWindow,
    device: &Ae5Device,
    controls: &[ControlSnapshot],
    message: Option<&str>,
    visible_page: Option<&str>,
) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&hero(device, controls));
    let status = gtk::Label::new(Some(
        message.unwrap_or("Ready — every change is verified against the hardware."),
    ));
    status.set_xalign(0.0);
    status.set_wrap(true);
    status.add_css_class("operation-status");
    root.append(&status);

    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .hexpand(true)
        .vexpand(true)
        .build();
    stack.set_widget_name(MAIN_STACK_NAME);

    for (name, title) in [
        ("device", "Device"),
        ("compatibility", "Compatibility"),
        ("routing", "System audio"),
        ("lighting", "Lighting"),
        ("profiles", "Profiles"),
        ("playback", "Playback"),
        ("effects", "Sound effects"),
        ("equalizer", "Equalizer"),
        ("recording", "Recording"),
    ] {
        let holder = gtk::Box::new(gtk::Orientation::Vertical, 0);
        holder.set_hexpand(true);
        holder.set_vexpand(true);
        stack.add_titled(&holder, Some(name), title);
    }

    let initial_page = visible_page.unwrap_or("device");
    stack.set_visible_child_name(initial_page);
    populate_page(&stack, window, device, controls, &status, initial_page);
    {
        let window = window.clone();
        let device = device.clone();
        let controls = controls.to_vec();
        let status = status.clone();
        stack.connect_visible_child_name_notify(move |stack| {
            if let Some(name) = stack.visible_child_name() {
                populate_page(stack, &window, &device, &controls, &status, &name);
            }
        });
    }

    let sidebar = gtk::StackSidebar::builder()
        .stack(&stack)
        .width_request(190)
        .build();
    sidebar.add_css_class("navigation-sidebar");

    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    body.append(&sidebar);
    body.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    body.append(&stack);
    root.append(&body);
    root
}

fn populate_page(
    stack: &gtk::Stack,
    window: &gtk::ApplicationWindow,
    device: &Ae5Device,
    controls: &[ControlSnapshot],
    status: &gtk::Label,
    name: &str,
) {
    let Some(holder) = stack
        .child_by_name(name)
        .and_then(|child| child.downcast::<gtk::Box>().ok())
    else {
        return;
    };
    if holder.first_child().is_some() {
        return;
    }

    let page: gtk::Widget = match name {
        "device" => device_page(window, device, controls, status).upcast(),
        "compatibility" => compatibility_page().upcast(),
        "routing" => routing_page(device.card_index, status).upcast(),
        "lighting" => lighting_page(window, status).upcast(),
        "profiles" => profile_page(window, device.card_index, status).upcast(),
        _ => {
            let Some(category) = Category::ALL
                .into_iter()
                .find(|category| category.id() == name)
            else {
                return;
            };
            control_page(
                device.card_index,
                status,
                controls,
                controls
                    .iter()
                    .filter(|control| category.matches(&control.name)),
            )
            .upcast()
        }
    };
    page.set_hexpand(true);
    page.set_vexpand(true);
    holder.append(&page);
}

fn hero(device: &Ae5Device, controls: &[ControlSnapshot]) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 20);
    header.add_css_class("hero");

    let titles = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let title = gtk::Label::new(Some(
        device
            .codec_name
            .as_deref()
            .unwrap_or("Creative Sound BlasterX AE-5"),
    ));
    title.set_xalign(0.0);
    title.add_css_class("hero-title");
    let subtitle = gtk::Label::new(Some(&format!(
        "{} · PCI {} · subsystem {}",
        device.alsa_name,
        device.pci_id(),
        device.subsystem_id()
    )));
    subtitle.set_xalign(0.0);
    subtitle.add_css_class("dim-label");
    titles.append(&title);
    titles.append(&subtitle);
    header.append(&titles);

    let status = gtk::Label::new(Some(&format!("{} live controls", controls.len())));
    status.add_css_class("status-pill");
    status.set_halign(gtk::Align::End);
    status.set_hexpand(true);
    header.append(&status);
    header
}

fn device_page(
    window: &gtk::ApplicationWindow,
    device: &Ae5Device,
    controls: &[ControlSnapshot],
    status: &gtk::Label,
) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Label::new(Some("Device & diagnostics"));
    heading.set_xalign(0.0);
    heading.add_css_class("page-title");
    page.append(&heading);

    let identity = gtk::Box::new(gtk::Orientation::Vertical, 4);
    for detail in [
        format!("ALSA card {} · {}", device.card_index, device.alsa_name),
        device.alsa_long_name.clone(),
        format!(
            "PCI {} · subsystem {}",
            device.pci_id(),
            device.subsystem_id()
        ),
        format!(
            "Codec {}",
            device.codec_name.as_deref().unwrap_or("not reported")
        ),
    ] {
        let label = gtk::Label::new(Some(&detail));
        label.set_xalign(0.0);
        label.set_selectable(true);
        identity.append(&label);
    }
    page.append(&profile_card(
        "01",
        "Detected hardware",
        "AE-5 Control matches the PCI and subsystem IDs instead of relying on an unstable ALSA card number.",
        &identity,
    ));

    let warnings = driver_range_warnings(controls);
    let capability_summary = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let selectable = controls
        .iter()
        .filter(|control| control.selected.is_some())
        .count();
    let playback = controls
        .iter()
        .filter(|control| control.playback_switch.is_some() || control.playback_level.is_some())
        .count();
    let recording = controls
        .iter()
        .filter(|control| control.capture_switch.is_some() || control.capture_level.is_some())
        .count();
    let summary = gtk::Label::new(Some(&format!(
        "{} live controls · {} selectable · {} playback · {} recording",
        controls.len(),
        selectable,
        playback,
        recording
    )));
    summary.set_xalign(0.0);
    capability_summary.append(&summary);
    let health = gtk::Label::new(Some(if warnings.is_empty() {
        "Driver ranges are internally consistent."
    } else {
        "One or more driver values are outside their declared ranges."
    }));
    health.set_xalign(0.0);
    health.add_css_class(if warnings.is_empty() {
        "operation-ok"
    } else {
        "warning-value"
    });
    capability_summary.append(&health);
    for warning in &warnings {
        let label = gtk::Label::new(Some(warning));
        label.set_xalign(0.0);
        label.set_wrap(true);
        label.add_css_class("warning-value");
        capability_summary.append(&label);
    }
    page.append(&profile_card(
        "02",
        "Live capabilities",
        "Only controls exposed by the running ALSA driver appear in the application.",
        &capability_summary,
    ));

    let (route_summary, route_healthy) =
        route_health_summary(controls, ae5_route_state(device.card_index));
    let route_health = gtk::Label::new(Some(&route_summary));
    route_health.set_xalign(0.0);
    route_health.set_wrap(true);
    route_health.set_selectable(true);
    route_health.add_css_class(if route_healthy {
        "operation-ok"
    } else {
        "warning-value"
    });
    let route_actions = gtk::Box::new(gtk::Orientation::Vertical, 0);
    route_actions.append(&route_health);
    page.append(&profile_card(
        "03",
        "Desktop route health",
        "The ALSA output choice and PipeWire route must agree. This check is read-only.",
        &route_actions,
    ));

    let report_actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let save_report = gtk::Button::with_label("Save diagnostics report");
    report_actions.append(&save_report);
    page.append(&profile_card(
        "04",
        "Private diagnostics",
        "Create a local report without root. Hostname, user, storage, network data, and unrelated PipeWire devices are omitted by default. Review the file before sharing it.",
        &report_actions,
    ));

    {
        let window = window.clone();
        let status = status.clone();
        save_report.connect_clicked(move |button| {
            let window = window.clone();
            let status = status.clone();
            let button = button.clone();
            button.set_sensitive(false);
            gtk::glib::spawn_future_local(async move {
                match save_diagnostics_report(&window).await {
                    Ok(Some(message)) => set_status(&status, true, &message),
                    Ok(None) => {}
                    Err(error) => {
                        set_status(&status, false, &format!("Diagnostics failed: {error}"))
                    }
                }
                button.set_sensitive(true);
            });
        });
    }

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn compatibility_page() -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Label::new(Some("Sound Blaster Command compatibility"));
    heading.set_xalign(0.0);
    heading.add_css_class("page-title");
    page.append(&heading);

    let summary = gtk::Box::new(gtk::Orientation::Vertical, 4);
    for line in compatibility_summary().lines() {
        let label = gtk::Label::new(Some(line));
        label.set_xalign(0.0);
        summary.append(&label);
    }
    page.append(&profile_card(
        "01",
        "Tracked feature status",
        "This read-only view is built from the same evidence matrix used by the project. A Linux-native equivalent is labeled as a substitution instead of being presented as Creative's implementation.",
        &summary,
    ));

    for (index, support, title, description) in [
        (
            "02",
            FeatureSupport::Unsupported,
            "Unavailable features",
            "No verified safe Linux mechanism exists for these functions. They are listed explicitly instead of appearing as nonfunctional controls.",
        ),
        (
            "03",
            FeatureSupport::Deferred,
            "Pending acceptance",
            "These functions have a Linux control, candidate, or substitute, but still need the stated physical evidence before the project claims full support.",
        ),
    ] {
        let entries = gtk::Box::new(gtk::Orientation::Vertical, 8);
        for feature in feature_parity().filter(|feature| feature.support == support) {
            let expander =
                gtk::Expander::new(Some(&format!("{} · {}", feature.area, feature.feature)));
            expander.add_css_class("feature-entry");

            let details = gtk::Box::new(gtk::Orientation::Vertical, 4);
            for (label, value) in [
                ("Linux mechanism", feature.linux_mechanism),
                ("Current evidence", feature.current_evidence),
                ("Remaining gate", feature.remaining_gate),
                ("Source", feature.source),
            ] {
                let text = gtk::Label::new(Some(&format!("{label}: {value}")));
                text.set_xalign(0.0);
                text.set_wrap(true);
                text.set_selectable(true);
                text.add_css_class("dim-label");
                details.append(&text);
            }
            expander.set_child(Some(&details));
            entries.append(&expander);
        }
        page.append(&profile_card(index, title, description, &entries));
    }

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn compatibility_summary() -> String {
    let features = feature_parity().collect::<Vec<_>>();
    let counts = FeatureSupport::ALL.map(|support| {
        features
            .iter()
            .filter(|feature| feature.support == support)
            .count()
    });
    format!(
        "{} tracked features\nVerified: {}\nLinux-native equivalents: {}\nPending acceptance: {}\nUnavailable: {}",
        features.len(),
        counts[0],
        counts[1],
        counts[2],
        counts[3]
    )
}

fn routing_page(card_index: i32, status: &gtk::Label) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Label::new(Some("System audio routing"));
    heading.set_xalign(0.0);
    heading.add_css_class("page-title");
    page.append(&heading);

    let intro = gtk::Label::new(Some(
        "Choose whether desktop applications use the AE-5 by default. These \
         actions change WirePlumber routing only; they do not alter ALSA or DSP settings.",
    ));
    intro.set_xalign(0.0);
    intro.set_wrap(true);
    intro.add_css_class("dim-label");
    page.append(&intro);

    page.append(&routing_card(
        card_index,
        status,
        "01",
        "Playback output",
        ae5_output(card_index),
        set_ae5_default_output,
    ));
    page.append(&routing_card(
        card_index,
        status,
        "02",
        "Recording input",
        ae5_input(card_index),
        set_ae5_default_input,
    ));
    page.append(&native_rates_card(status, native_rates_config()));

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn lighting_page(window: &gtk::ApplicationWindow, status: &gtk::Label) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Label::new(Some("Onboard lighting"));
    heading.set_xalign(0.0);
    heading.add_css_class("page-title");
    page.append(&heading);

    let intro = gtk::Label::new(Some(
        "Set the five LEDs built into the AE-5. Colors are verified through the \
         Linux multicolor LED class and saved for the next desktop login.",
    ));
    intro.set_xalign(0.0);
    intro.set_wrap(true);
    intro.add_css_class("dim-label");
    page.append(&intro);

    match Ae5Lighting::discover().and_then(|lighting| lighting.colors()) {
        Ok(colors) => {
            let all = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let summary =
                gtk::Label::new(Some(&if colors.iter().all(|color| *color == colors[0]) {
                    format!("All five LEDs are {}", colors[0])
                } else {
                    "The five LEDs currently use different colors".to_owned()
                }));
            summary.set_xalign(0.0);
            summary.set_hexpand(true);
            let button = color_button("Choose one color for all onboard LEDs", colors[0]);
            {
                let window = window.clone();
                let status = status.clone();
                let updating = Rc::new(Cell::new(false));
                let updating_on_change = updating.clone();
                button.connect_rgba_notify(move |button| {
                    if updating_on_change.get() {
                        return;
                    }
                    let requested = rgb_from_rgba(&button.rgba());
                    match set_saved_lighting([requested; ONBOARD_LED_COUNT]) {
                        Ok(_) => {
                            let _ = refresh_window(
                                &window,
                                Some(&format!(
                                    "Applied, verified, and saved {requested} for all onboard LEDs."
                                )),
                            );
                        }
                        Err(error) => {
                            updating_on_change.set(true);
                            button.set_rgba(&rgba_from_rgb(colors[0]));
                            updating_on_change.set(false);
                            set_status(&status, false, &format!("Lighting change failed: {error}"));
                        }
                    }
                });
            }
            all.append(&summary);
            all.append(&button);
            page.append(&profile_card(
                "01",
                "Unified color",
                "Choose one color for the complete five-LED chain.",
                &all,
            ));

            let individual = gtk::Box::new(gtk::Orientation::Vertical, 10);
            for (index, color) in colors.into_iter().enumerate() {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                row.add_css_class("profile-library-row");
                let label = gtk::Label::new(Some(&format!("LED {} · {color}", index + 1)));
                label.set_xalign(0.0);
                label.set_hexpand(true);
                let button = color_button(
                    &format!("Choose a color for onboard LED {}", index + 1),
                    color,
                );
                {
                    let window = window.clone();
                    let status = status.clone();
                    let updating = Rc::new(Cell::new(false));
                    let updating_on_change = updating.clone();
                    button.connect_rgba_notify(move |button| {
                        if updating_on_change.get() {
                            return;
                        }
                        let requested = rgb_from_rgba(&button.rgba());
                        match set_saved_led(index + 1, requested) {
                            Ok(_) => {
                                let _ = refresh_window(
                                    &window,
                                    Some(&format!(
                                        "Applied, verified, and saved {requested} for onboard LED {}.",
                                        index + 1
                                    )),
                                );
                            }
                            Err(error) => {
                                updating_on_change.set(true);
                                button.set_rgba(&rgba_from_rgb(color));
                                updating_on_change.set(false);
                                set_status(
                                    &status,
                                    false,
                                    &format!("Lighting change failed: {error}"),
                                );
                            }
                        }
                    });
                }
                row.append(&label);
                row.append(&button);
                individual.append(&row);
            }
            page.append(&profile_card(
                "02",
                "Individual LEDs",
                "Each selection resends one coherent five-LED frame in the kernel.",
                &individual,
            ));
        }
        Err(error) => {
            let unavailable = gtk::Box::new(gtk::Orientation::Vertical, 6);
            let title = gtk::Label::new(Some("Lighting interface unavailable"));
            title.set_xalign(0.0);
            title.add_css_class("warning-label");
            let detail = gtk::Label::new(Some(&error.to_string()));
            detail.set_xalign(0.0);
            detail.set_wrap(true);
            unavailable.append(&title);
            unavailable.append(&detail);
            page.append(&profile_card(
                "01",
                "Kernel support required",
                "Install and boot a kernel containing the AE-5 onboard multicolor LED patch.",
                &unavailable,
            ));
        }
    }

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn color_button(label: &str, color: RgbColor) -> gtk::ColorDialogButton {
    let dialog = gtk::ColorDialog::builder()
        .title(label)
        .modal(true)
        .with_alpha(false)
        .build();
    let button = gtk::ColorDialogButton::new(Some(dialog));
    button.set_rgba(&rgba_from_rgb(color));
    button.set_tooltip_text(Some(label));
    button.update_property(&[gtk::accessible::Property::Label(label)]);
    button
}

fn rgba_from_rgb(color: RgbColor) -> gtk::gdk::RGBA {
    gtk::gdk::RGBA::new(
        f32::from(color.red) / 255.0,
        f32::from(color.green) / 255.0,
        f32::from(color.blue) / 255.0,
        1.0,
    )
}

fn rgb_from_rgba(color: &gtk::gdk::RGBA) -> RgbColor {
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    RgbColor::new(
        channel(color.red()),
        channel(color.green()),
        channel(color.blue()),
    )
}

fn native_rates_card(status: &gtk::Label, current: std::io::Result<NativeRatesConfig>) -> gtk::Box {
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let state = gtk::Label::new(None);
    state.set_xalign(0.0);
    state.set_wrap(true);
    state.set_hexpand(true);
    let button = gtk::Button::with_label("Enable after restart");
    button.add_css_class("suggested-action");
    let enabled = Rc::new(Cell::new(false));

    match current {
        Ok(config) => {
            enabled.set(config.enabled);
            state.set_text(&native_rates_summary(&config));
            if config.enabled {
                button.set_label("Disable after restart");
            }
        }
        Err(error) => {
            state.set_text(&format!("Configuration unavailable: {error}"));
            button.set_sensitive(false);
        }
    }
    actions.append(&state);
    actions.append(&button);

    let status = status.clone();
    let state_on_click = state.clone();
    let enabled_on_click = enabled.clone();
    button.connect_clicked(move |button| {
        let requested = !enabled_on_click.get();
        match set_native_rates_enabled(requested) {
            Ok(config) => {
                enabled_on_click.set(config.enabled);
                state_on_click.set_text(&native_rates_summary(&config));
                button.set_label(if config.enabled {
                    "Disable after restart"
                } else {
                    "Enable after restart"
                });
                set_status(
                    &status,
                    true,
                    "Native-rate configuration saved. Restart PipeWire or log in again to apply.",
                );
            }
            Err(error) => set_status(
                &status,
                false,
                &format!("Native-rate configuration failed: {error}"),
            ),
        }
    });

    profile_card(
        "03",
        "Native sample rates",
        "Experimental: allow 44.1, 48, and 96 kHz streams to avoid unnecessary \
         resampling. This changes the global PipeWire graph and may affect other devices.",
        &actions,
    )
}

fn native_rates_summary(config: &NativeRatesConfig) -> String {
    if config.enabled {
        "Enabled in PipeWire configuration\n44.1, 48, and 96 kHz".to_owned()
    } else {
        "Disabled\nUsing the distribution PipeWire defaults".to_owned()
    }
}

fn routing_card(
    card_index: i32,
    status: &gtk::Label,
    index: &str,
    title: &str,
    current: std::io::Result<Option<PipeWireNode>>,
    make_default: fn(i32) -> std::io::Result<PipeWireNode>,
) -> gtk::Box {
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let state = gtk::Label::new(None);
    state.set_xalign(0.0);
    state.set_wrap(true);
    state.set_hexpand(true);
    let button = gtk::Button::with_label("Make default");
    button.add_css_class("suggested-action");

    match current {
        Ok(Some(node)) => {
            state.set_text(&pipewire_node_summary(&node));
            button.set_sensitive(!node.is_default);
            if node.is_default {
                button.set_label("Default");
            }
        }
        Ok(None) => {
            state.set_text("AE-5 node unavailable in PipeWire.");
            button.set_sensitive(false);
        }
        Err(error) => {
            state.set_text(&format!("PipeWire status unavailable: {error}"));
            button.set_sensitive(false);
        }
    }
    actions.append(&state);
    actions.append(&button);

    let status = status.clone();
    let state_on_click = state.clone();
    button.connect_clicked(move |button| match make_default(card_index) {
        Ok(node) => {
            state_on_click.set_text(&pipewire_node_summary(&node));
            button.set_label("Default");
            button.set_sensitive(false);
            set_status(
                &status,
                true,
                &format!("AE-5 is now the default for {}.", node.description),
            );
        }
        Err(error) => set_status(
            &status,
            false,
            &format!("Default-device change failed: {error}"),
        ),
    });

    profile_card(
        index,
        title,
        "The selected default is stored by WirePlumber for desktop applications.",
        &actions,
    )
}

fn pipewire_node_summary(node: &PipeWireNode) -> String {
    format!(
        "{}\n{} — {}",
        node.description,
        node.node_name,
        if node.is_default {
            "currently default"
        } else {
            "not default"
        }
    )
}

fn route_health_summary(
    controls: &[ControlSnapshot],
    current: std::io::Result<PipeWireRouteState>,
) -> (String, bool) {
    let output_choice = controls
        .iter()
        .find(|control| control.name == "Output Select")
        .and_then(|control| control.selected.as_deref());
    let input_choice = controls
        .iter()
        .find(|control| control.name == "Input Source")
        .and_then(|control| control.selected.as_deref());
    let speaker_layout = controls
        .iter()
        .find(|control| control.name == "Surround Channel Config")
        .and_then(|control| control.selected.as_deref());
    match (current, output_choice, speaker_layout, input_choice) {
        (Ok(state), Some(output), Some(layout), Some(input)) => {
            let issues = [state.output_issue(output, layout), state.input_issue(input)]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>();
            (
                format!(
                    "{}\nALSA output: {output}\nPipeWire output: {}\nALSA input: {input}\nPipeWire input: {}\nProfile: {} ({}){}",
                    if issues.is_empty() {
                        "Matched"
                    } else {
                        "Needs attention"
                    },
                    state.output_route.as_deref().unwrap_or("unavailable"),
                    state.input_route.as_deref().unwrap_or("unavailable"),
                    state.active_profile.as_deref().unwrap_or("unavailable"),
                    state
                        .profile_set
                        .as_deref()
                        .unwrap_or("unknown profile set"),
                    if issues.is_empty() {
                        String::new()
                    } else {
                        format!("\n{}", issues.join("\n"))
                    }
                ),
                issues.is_empty(),
            )
        }
        (Ok(_), None, _, _) => ("Output Select is unavailable from ALSA.".to_owned(), false),
        (Ok(_), _, None, _) => (
            "Surround Channel Config is unavailable from ALSA.".to_owned(),
            false,
        ),
        (Ok(_), _, _, None) => ("Input Source is unavailable from ALSA.".to_owned(), false),
        (Err(error), _, _, _) => (
            format!("PipeWire route status is unavailable: {error}"),
            false,
        ),
    }
}

fn driver_range_warnings(controls: &[ControlSnapshot]) -> Vec<String> {
    controls
        .iter()
        .flat_map(|control| {
            [
                ("playback", control.playback_level.as_ref()),
                ("capture", control.capture_level.as_ref()),
            ]
            .into_iter()
            .filter_map(move |(direction, level)| {
                let level = level?;
                (!(level.min..=level.max).contains(&level.value)).then(|| {
                    format!(
                        "{} {direction} value {} is outside {}..{}",
                        control.name, level.value, level.min, level.max
                    )
                })
            })
        })
        .collect()
}

async fn save_diagnostics_report(
    window: &gtk::ApplicationWindow,
) -> Result<Option<String>, String> {
    let dialog = gtk::FileDialog::builder()
        .title("Save private AE-5 diagnostics")
        .modal(true)
        .initial_name("ae5-report.txt")
        .build();
    let file = match dialog.save_future(Some(window)).await {
        Ok(file) => file,
        Err(error) if is_cancelled(&error) => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    let path = file
        .path()
        .ok_or_else(|| "only local files are supported".to_owned())?;
    let argv = diagnostics_argv(&path);
    let argv = argv.iter().map(OsString::as_os_str).collect::<Vec<_>>();
    let process = gio::Subprocess::newv(
        &argv,
        gio::SubprocessFlags::STDOUT_PIPE | gio::SubprocessFlags::STDERR_PIPE,
    )
    .map_err(|error| {
        format!("unable to start ae5-collect-report; install the AE-5 Control package: {error}")
    })?;
    let (stdout, stderr) = process
        .communicate_utf8_future(None)
        .await
        .map_err(|error| error.to_string())?;
    if !process.is_successful() {
        let detail = stderr
            .as_deref()
            .or(stdout.as_deref())
            .map(str::trim)
            .filter(|message| !message.is_empty())
            .unwrap_or("the report command failed without an error message");
        return Err(format!(
            "ae5-collect-report exited with status {}: {detail}",
            process.exit_status()
        ));
    }

    Ok(Some(format!(
        "Diagnostics saved to {}. Review the file before sharing it.",
        path.display()
    )))
}

fn diagnostics_argv(path: &Path) -> [OsString; 2] {
    [
        OsString::from("ae5-collect-report"),
        path.as_os_str().to_owned(),
    ]
}

fn profile_page(
    window: &gtk::ApplicationWindow,
    card_index: i32,
    status: &gtk::Label,
) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Label::new(Some("Profiles & migration"));
    heading.set_xalign(0.0);
    heading.add_css_class("page-title");
    page.append(&heading);

    let intro = gtk::Label::new(Some(
        "Capture the live card, preview transactional changes, or convert your \
         Sound Blaster Command setup without altering the source files.",
    ));
    intro.set_xalign(0.0);
    intro.set_wrap(true);
    intro.add_css_class("dim-label");
    page.append(&intro);

    let saved_actions = saved_profile_actions(window, card_index, status);
    page.append(&profile_card(
        "01",
        "Saved profiles",
        "Profiles in the standard per-user library are available immediately after \
         an app restart. Every apply still uses preview, validation, readback, and rollback.",
        &saved_actions,
    ));

    let native_actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let save = gtk::Button::with_label("Save current state");
    save.add_css_class("suggested-action");
    let apply = gtk::Button::with_label("Preview & apply profile");
    native_actions.append(&save);
    native_actions.append(&apply);
    page.append(&profile_card(
        "02",
        "Profile files",
        "Portable JSON uses semantic ALSA names. Applying validates every value, \
         verifies readback, and rolls back the targeted controls on failure.",
        &native_actions,
    ));

    let reset = gtk::Button::with_label("Preview & reset processing");
    reset.add_css_class("destructive-action");
    reset.set_tooltip_text(Some(
        "Preserves routing, speaker layout, mixer volumes, mutes, and PipeWire settings.",
    ));
    let reset_actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    reset_actions.append(&reset);
    page.append(&profile_card(
        "03",
        "Linux driver defaults",
        "Restore the CA0132 processing values initialized by the Linux driver. \
         A native backup is saved before the first mixer write; this is not claimed \
         to reproduce Sound Blaster Command's undocumented factory reset.",
        &reset_actions,
    ));

    let import_active = gtk::Button::with_label("Import active Windows setup");
    import_active.add_css_class("suggested-action");
    let import = gtk::Button::with_label("Choose profile & EQ files");
    let import_actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    import_actions.append(&import_active);
    import_actions.append(&import);
    page.append(&profile_card(
        "04",
        "Sound Blaster Command",
        "Choose a mounted Windows user folder to discover and import its active setup, \
         or choose Creative profile and EQ files manually. Review every mapping before saving.",
        &import_actions,
    ));

    {
        let window = window.clone();
        let status = status.clone();
        save.connect_clicked(move |_| {
            let window = window.clone();
            let status = status.clone();
            gtk::glib::spawn_future_local(async move {
                match save_current_profile(&window, card_index).await {
                    Ok(Some(message)) => {
                        let _ = refresh_window(&window, Some(&message));
                    }
                    Ok(None) => {}
                    Err(error) => set_status(&status, false, &format!("Save failed: {error}")),
                }
            });
        });
    }
    {
        let window = window.clone();
        let status = status.clone();
        apply.connect_clicked(move |_| {
            let window = window.clone();
            let status = status.clone();
            gtk::glib::spawn_future_local(async move {
                match apply_native_profile(&window, card_index).await {
                    Ok(Some(message)) => {
                        let _ = refresh_window(&window, Some(&message));
                    }
                    Ok(None) => {}
                    Err(error) => set_status(&status, false, &format!("Apply failed: {error}")),
                }
            });
        });
    }
    {
        let window = window.clone();
        let status = status.clone();
        reset.connect_clicked(move |_| {
            let window = window.clone();
            let status = status.clone();
            gtk::glib::spawn_future_local(async move {
                match reset_linux_driver_defaults(&window, card_index).await {
                    Ok(Some(message)) => {
                        let _ = refresh_window(&window, Some(&message));
                    }
                    Ok(None) => {}
                    Err(error) => set_status(&status, false, &format!("Reset failed: {error}")),
                }
            });
        });
    }
    {
        let window = window.clone();
        let status = status.clone();
        import_active.connect_clicked(move |_| {
            let window = window.clone();
            let status = status.clone();
            gtk::glib::spawn_future_local(async move {
                match import_active_windows_profile(&window, card_index).await {
                    Ok(Some(message)) => {
                        let _ = refresh_window(&window, Some(&message));
                    }
                    Ok(None) => {}
                    Err(error) => set_status(&status, false, &format!("Import failed: {error}")),
                }
            });
        });
    }
    {
        let window = window.clone();
        let status = status.clone();
        import.connect_clicked(move |_| {
            let window = window.clone();
            let status = status.clone();
            gtk::glib::spawn_future_local(async move {
                match import_windows_profile(&window, card_index).await {
                    Ok(Some(message)) => {
                        let _ = refresh_window(&window, Some(&message));
                    }
                    Ok(None) => {}
                    Err(error) => set_status(&status, false, &format!("Import failed: {error}")),
                }
            });
        });
    }

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn saved_profile_actions(
    window: &gtk::ApplicationWindow,
    card_index: i32,
    status: &gtk::Label,
) -> gtk::Box {
    let actions = gtk::Box::new(gtk::Orientation::Vertical, 8);
    match profile_library() {
        Ok(library) => {
            if library.profiles.is_empty() {
                let empty = gtk::Label::new(Some(
                    "No saved profiles yet. New and converted profiles start in this library.",
                ));
                empty.set_xalign(0.0);
                empty.set_wrap(true);
                empty.add_css_class("dim-label");
                actions.append(&empty);
            }
            for entry in library.profiles {
                let row = gtk::Box::new(gtk::Orientation::Vertical, 8);
                row.add_css_class("profile-library-row");
                let details = gtk::Box::new(gtk::Orientation::Vertical, 4);
                details.set_hexpand(true);
                let name = gtk::Entry::builder()
                    .text(&entry.profile.name)
                    .max_length(80)
                    .hexpand(true)
                    .build();
                name.update_property(&[gtk::accessible::Property::Label(&format!(
                    "Name for saved profile {}",
                    entry.profile.name
                ))]);
                let file = gtk::Label::new(Some(&format!(
                    "{} controls · {}",
                    entry.profile.controls.len(),
                    entry
                        .path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("profile.json")
                )));
                file.set_xalign(0.0);
                file.set_wrap(true);
                file.add_css_class("dim-label");
                details.append(&name);
                details.append(&file);

                let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                buttons.set_halign(gtk::Align::End);
                let apply = gtk::Button::with_label("Preview & apply");
                let export = gtk::Button::with_label("Export copy");
                let rename = gtk::Button::with_label("Rename");
                let trash = gtk::Button::with_label("Move to Trash");
                export.set_tooltip_text(Some(
                    "Save a standalone copy without changing the library profile.",
                ));
                trash.add_css_class("destructive-action");
                trash.set_tooltip_text(Some("The profile can be restored from the desktop Trash."));

                {
                    let path = entry.path.clone();
                    let window = window.clone();
                    let status = status.clone();
                    apply.connect_clicked(move |_| {
                        let path = path.clone();
                        let window = window.clone();
                        let status = status.clone();
                        gtk::glib::spawn_future_local(async move {
                            match apply_profile_path(&window, card_index, &path).await {
                                Ok(Some(message)) => {
                                    let _ = refresh_window(&window, Some(&message));
                                }
                                Ok(None) => {}
                                Err(error) => {
                                    set_status(&status, false, &format!("Apply failed: {error}"))
                                }
                            }
                        });
                    });
                }

                {
                    let path = entry.path.clone();
                    let window = window.clone();
                    let status = status.clone();
                    export.connect_clicked(move |_| {
                        let path = path.clone();
                        let window = window.clone();
                        let status = status.clone();
                        gtk::glib::spawn_future_local(async move {
                            match export_saved_profile(&window, &path).await {
                                Ok(Some(message)) => set_status(&status, true, &message),
                                Ok(None) => {}
                                Err(error) => {
                                    set_status(&status, false, &format!("Export failed: {error}"))
                                }
                            }
                        });
                    });
                }

                {
                    let path = entry.path.clone();
                    let window = window.clone();
                    let status = status.clone();
                    let name = name.clone();
                    rename.connect_clicked(move |_| {
                        match rename_library_profile(&path, name.text().as_str()) {
                            Ok(stored) => {
                                let message =
                                    format!("Renamed saved profile to “{}”.", stored.profile.name);
                                let _ = refresh_window(&window, Some(&message));
                            }
                            Err(error) => {
                                set_status(&status, false, &format!("Rename failed: {error}"))
                            }
                        }
                    });
                }
                {
                    let rename = rename.clone();
                    name.connect_activate(move |_| rename.emit_clicked());
                }
                {
                    let path = entry.path;
                    let window = window.clone();
                    let status = status.clone();
                    trash.connect_clicked(move |_| {
                        let path = path.clone();
                        let window = window.clone();
                        let status = status.clone();
                        gtk::glib::spawn_future_local(async move {
                            match trash_saved_profile(&window, &path).await {
                                Ok(Some(message)) => {
                                    let _ = refresh_window(&window, Some(&message));
                                }
                                Ok(None) => {}
                                Err(error) => {
                                    set_status(&status, false, &format!("Move failed: {error}"))
                                }
                            }
                        });
                    });
                }

                row.append(&details);
                buttons.append(&apply);
                buttons.append(&export);
                buttons.append(&rename);
                buttons.append(&trash);
                row.append(&buttons);
                actions.append(&row);
            }
            if !library.skipped.is_empty() {
                let warning = gtk::Label::new(Some(&format!(
                    "{} invalid JSON profile{} skipped. Open the library folder to inspect them.",
                    library.skipped.len(),
                    if library.skipped.len() == 1 {
                        " was"
                    } else {
                        "s were"
                    }
                )));
                warning.set_xalign(0.0);
                warning.set_wrap(true);
                warning.add_css_class("warning-label");
                actions.append(&warning);
            }
            let location = gtk::Label::new(Some(&library.directory.display().to_string()));
            location.set_xalign(0.0);
            location.set_selectable(true);
            location.set_wrap(true);
            location.add_css_class("dim-label");
            actions.append(&location);
        }
        Err(error) => {
            let warning = gtk::Label::new(Some(&format!("Profile library unavailable: {error}")));
            warning.set_xalign(0.0);
            warning.set_wrap(true);
            warning.add_css_class("warning-label");
            actions.append(&warning);
        }
    }
    actions
}

fn profile_card(index: &str, title: &str, description: &str, actions: &gtk::Box) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    card.add_css_class("profile-card");

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let index = gtk::Label::new(Some(index));
    index.add_css_class("section-index");
    let title = gtk::Label::new(Some(title));
    title.set_xalign(0.0);
    title.add_css_class("section-title");
    heading.append(&index);
    heading.append(&title);
    card.append(&heading);

    let description = gtk::Label::new(Some(description));
    description.set_xalign(0.0);
    description.set_wrap(true);
    description.add_css_class("dim-label");
    card.append(&description);
    card.append(actions);
    card
}

async fn save_current_profile(
    window: &gtk::ApplicationWindow,
    card_index: i32,
) -> Result<Option<String>, String> {
    let Some(path) =
        save_json_path(window, "Save native AE-5 profile", "ae5-profile.json", true).await?
    else {
        return Ok(None);
    };
    let name = profile_name_from_path(&path)?;
    let profile = Profile::capture(
        &name,
        snapshot_controls(card_index).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    profile.save_new(&path).map_err(|error| error.to_string())?;
    Ok(Some(format!(
        "Saved “{}” with {} controls to {}.",
        profile.name,
        profile.controls.len(),
        path.display()
    )))
}

async fn apply_native_profile(
    window: &gtk::ApplicationWindow,
    card_index: i32,
) -> Result<Option<String>, String> {
    let Some(path) = open_native_profile_path(window, "Open native AE-5 profile").await? else {
        return Ok(None);
    };
    apply_profile_path(window, card_index, &path).await
}

async fn apply_profile_path(
    window: &gtk::ApplicationWindow,
    card_index: i32,
    path: &Path,
) -> Result<Option<String>, String> {
    let profile = Profile::load(path).map_err(|error| error.to_string())?;
    let mixer = Ae5Mixer::open(card_index).map_err(|error| error.to_string())?;
    profile
        .check(&mixer, true)
        .map_err(|error| error.to_string())?;

    let high_gain = profile_requires_high_gain(&profile);
    if !confirm_profile(window, &profile, high_gain, "Apply profile").await? {
        return Ok(None);
    }
    let report = profile
        .apply(&mixer, high_gain)
        .map_err(|error| error.to_string())?;
    Ok(Some(format!(
        "Applied “{}”; {} controls were verified against the hardware.",
        profile.name, report.controls_applied
    )))
}

async fn reset_linux_driver_defaults(
    window: &gtk::ApplicationWindow,
    card_index: i32,
) -> Result<Option<String>, String> {
    let mixer = Ae5Mixer::open(card_index).map_err(|error| error.to_string())?;
    let defaults = validate_linux_driver_defaults(&mixer).map_err(|error| error.to_string())?;
    let library = profile_library_directory().map_err(|error| error.to_string())?;

    if !confirm_linux_driver_defaults(window, &defaults, &library).await? {
        return Ok(None);
    }

    std::fs::create_dir_all(&library).map_err(|error| error.to_string())?;
    let backup = linux_driver_defaults_backup_path(&library)?;
    let report = apply_linux_driver_defaults(&mixer, &backup).map_err(|error| error.to_string())?;
    Ok(Some(format!(
        "Restored {} Linux-driver processing controls. Previous valid state saved as {}.",
        report.controls_applied,
        backup.display()
    )))
}

async fn import_windows_profile(
    window: &gtk::ApplicationWindow,
    card_index: i32,
) -> Result<Option<String>, String> {
    let Some(profile_path) =
        open_json_path(window, "Choose Sound Blaster Command profile JSON").await?
    else {
        return Ok(None);
    };
    let Some(eq_path) = open_json_path(window, "Choose Sound Blaster Command EQ JSON").await?
    else {
        return Ok(None);
    };
    let Some(target) = choose_import_target(window).await? else {
        return Ok(None);
    };
    let initial_name = format!("windows-{}.json", target);
    let Some(output) =
        save_json_path(window, "Save converted native profile", &initial_name, true).await?
    else {
        return Ok(None);
    };
    let name = profile_name_from_path(&output)?;
    let import = import_sbcommand_profile_with_report(&name, &profile_path, &eq_path, target)
        .map_err(|error| error.to_string())?;
    validate_confirm_save_import(window, card_index, import, target, output).await
}

async fn import_active_windows_profile(
    window: &gtk::ApplicationWindow,
    card_index: i32,
) -> Result<Option<String>, String> {
    let Some(windows_user) =
        select_folder_path(window, "Choose the mounted Windows user folder").await?
    else {
        return Ok(None);
    };
    let installation =
        discover_sbcommand_installation(&windows_user).map_err(|error| error.to_string())?;
    let Some(target) = choose_import_target(window).await? else {
        return Ok(None);
    };
    let initial_name = format!("windows-active-{}.json", target);
    let Some(output) =
        save_json_path(window, "Save converted native profile", &initial_name, true).await?
    else {
        return Ok(None);
    };
    let name = profile_name_from_path(&output)?;
    let import = import_discovered_sbcommand_profile_with_report(&name, &installation, target)
        .map_err(|error| error.to_string())?;
    validate_confirm_save_import(window, card_index, import, target, output).await
}

async fn validate_confirm_save_import(
    window: &gtk::ApplicationWindow,
    card_index: i32,
    import: SbCommandImport,
    target: SbCommandTarget,
    output: PathBuf,
) -> Result<Option<String>, String> {
    import
        .profile
        .check(
            &Ae5Mixer::open(card_index).map_err(|error| error.to_string())?,
            false,
        )
        .map_err(|error| error.to_string())?;

    if !confirm_import(window, &import).await? {
        return Ok(None);
    }
    import
        .profile
        .save_new(&output)
        .map_err(|error| error.to_string())?;
    Ok(Some(format!(
        "Converted {} Windows settings to “{}” with {} mapped controls; {} unsupported settings were skipped. Saved at {}.",
        target,
        import.profile.name,
        import.profile.controls.len(),
        import.report.unsupported.len(),
        output.display()
    )))
}

async fn choose_import_target(
    window: &gtk::ApplicationWindow,
) -> Result<Option<SbCommandTarget>, String> {
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message("Which Windows output should be imported?")
        .detail("Sound Blaster Command stores separate speaker and headphone settings.")
        .buttons(["Cancel", "Headphones", "Speakers"])
        .cancel_button(0)
        .default_button(1)
        .build();
    match dialog.choose_future(Some(window)).await {
        Ok(1) => Ok(Some(SbCommandTarget::Headphone)),
        Ok(2) => Ok(Some(SbCommandTarget::Speaker)),
        Ok(_) => Ok(None),
        Err(error) if is_cancelled(&error) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

async fn trash_saved_profile(
    window: &gtk::ApplicationWindow,
    path: &Path,
) -> Result<Option<String>, String> {
    let stored = library_profile(path).map_err(|error| error.to_string())?;
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message(format!("Move “{}” to Trash?", stored.profile.name))
        .detail("This removes the profile from AE-5 Control. You can restore it from the desktop Trash.")
        .buttons(["Cancel", "Move to Trash"])
        .cancel_button(0)
        .default_button(0)
        .build();
    match dialog.choose_future(Some(window)).await {
        Ok(1) => {}
        Ok(_) => return Ok(None),
        Err(error) if is_cancelled(&error) => return Ok(None),
        Err(error) => return Err(error.to_string()),
    }

    gio::File::for_path(&stored.path)
        .trash_future(gtk::glib::Priority::DEFAULT)
        .await
        .map_err(|error| error.to_string())?;
    Ok(Some(format!("Moved “{}” to Trash.", stored.profile.name)))
}

async fn export_saved_profile(
    window: &gtk::ApplicationWindow,
    path: &Path,
) -> Result<Option<String>, String> {
    let stored = library_profile(path).map_err(|error| error.to_string())?;
    let initial_name = stored
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("ae5-profile.json");
    let Some(destination) =
        save_json_path(window, "Export native AE-5 profile", initial_name, false).await?
    else {
        return Ok(None);
    };
    export_library_profile(&stored.path, &destination).map_err(|error| error.to_string())?;
    Ok(Some(format!(
        "Exported “{}” to {}.",
        stored.profile.name,
        destination.display()
    )))
}

async fn confirm_profile(
    window: &gtk::ApplicationWindow,
    profile: &Profile,
    high_gain: bool,
    action: &str,
) -> Result<bool, String> {
    let action_label = if high_gain {
        "Apply with high gain"
    } else {
        action
    };
    let warning = if high_gain {
        "\n\nWarning: this profile requests 150–600 Ω headphone gain."
    } else {
        ""
    };
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message(format!("Preview “{}”", profile.name))
        .detail(format!("{}{warning}", profile_preview(profile)))
        .buttons(["Cancel", action_label])
        .cancel_button(0)
        .default_button(0)
        .build();
    match dialog.choose_future(Some(window)).await {
        Ok(1) => Ok(true),
        Ok(_) => Ok(false),
        Err(error) if is_cancelled(&error) => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

async fn confirm_linux_driver_defaults(
    window: &gtk::ApplicationWindow,
    profile: &Profile,
    backup_directory: &Path,
) -> Result<bool, String> {
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message("Reset to AE-5 Linux driver defaults?")
        .detail(linux_driver_defaults_preview(profile, backup_directory))
        .buttons(["Cancel", "Save backup & reset"])
        .cancel_button(0)
        .default_button(0)
        .build();
    match dialog.choose_future(Some(window)).await {
        Ok(1) => Ok(true),
        Ok(_) => Ok(false),
        Err(error) if is_cancelled(&error) => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

fn linux_driver_defaults_preview(profile: &Profile, backup_directory: &Path) -> String {
    let preserved = LINUX_DRIVER_DEFAULTS_PRESERVED
        .iter()
        .map(|item| format!("• {item}"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n\nPreserved:\n{}\n\nBefore the first write, a restorable native profile will be saved in {}.",
        profile_preview(profile),
        preserved,
        backup_directory.display()
    )
}

fn linux_driver_defaults_backup_path(directory: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_millis();
    Ok(directory.join(format!("before-linux-driver-defaults-{timestamp}.json")))
}

async fn confirm_import(
    window: &gtk::ApplicationWindow,
    import: &SbCommandImport,
) -> Result<bool, String> {
    let dialog = gtk::AlertDialog::builder()
        .modal(true)
        .message(format!(
            "Review Windows migration for “{}”",
            import.profile.name
        ))
        .detail(migration_preview(import))
        .buttons(["Cancel", "Save converted profile"])
        .cancel_button(0)
        .default_button(0)
        .build();
    match dialog.choose_future(Some(window)).await {
        Ok(1) => Ok(true),
        Ok(_) => Ok(false),
        Err(error) if is_cancelled(&error) => Ok(false),
        Err(error) => Err(error.to_string()),
    }
}

fn migration_preview(import: &SbCommandImport) -> String {
    format!(
        "{} validated Linux controls will be saved. Unsupported settings are skipped.\n\n{}\n\n{}\n\n{}",
        import.profile.controls.len(),
        report_preview_section("Exact mappings", &import.report.exact, 8),
        report_preview_section("Approximate mappings", &import.report.approximate, 12),
        report_preview_section(
            "Unsupported settings",
            &import.report.unsupported,
            usize::MAX
        )
    )
}

fn report_preview_section(title: &str, items: &[String], limit: usize) -> String {
    let mut lines = items
        .iter()
        .take(limit)
        .map(|item| format!("• {item}"))
        .collect::<Vec<_>>();
    if items.len() > limit {
        lines.push(format!("…and {} more", items.len() - limit));
    }
    if lines.is_empty() {
        lines.push("None".to_owned());
    }
    format!("{title} ({})\n{}", items.len(), lines.join("\n"))
}

fn json_dialog(title: &str) -> gtk::FileDialog {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("JSON profiles"));
    filter.add_mime_type("application/json");
    filter.add_pattern("*.json");
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    gtk::FileDialog::builder()
        .title(title)
        .modal(true)
        .filters(&filters)
        .default_filter(&filter)
        .build()
}

async fn open_json_path(
    window: &gtk::ApplicationWindow,
    title: &str,
) -> Result<Option<PathBuf>, String> {
    match json_dialog(title).open_future(Some(window)).await {
        Ok(file) => file
            .path()
            .map(Some)
            .ok_or_else(|| "only local files are supported".to_owned()),
        Err(error) if is_cancelled(&error) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

async fn open_native_profile_path(
    window: &gtk::ApplicationWindow,
    title: &str,
) -> Result<Option<PathBuf>, String> {
    let dialog = json_dialog(title);
    set_initial_profile_folder(&dialog)?;
    match dialog.open_future(Some(window)).await {
        Ok(file) => file
            .path()
            .map(Some)
            .ok_or_else(|| "only local files are supported".to_owned()),
        Err(error) if is_cancelled(&error) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

async fn select_folder_path(
    window: &gtk::ApplicationWindow,
    title: &str,
) -> Result<Option<PathBuf>, String> {
    let dialog = gtk::FileDialog::builder().title(title).modal(true).build();
    match dialog.select_folder_future(Some(window)).await {
        Ok(folder) => folder
            .path()
            .map(Some)
            .ok_or_else(|| "only local folders are supported".to_owned()),
        Err(error) if is_cancelled(&error) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

async fn save_json_path(
    window: &gtk::ApplicationWindow,
    title: &str,
    initial_name: &str,
    start_in_library: bool,
) -> Result<Option<PathBuf>, String> {
    let dialog = json_dialog(title);
    if start_in_library {
        set_initial_profile_folder(&dialog)?;
    }
    dialog.set_initial_name(Some(initial_name));
    match dialog.save_future(Some(window)).await {
        Ok(file) => file
            .path()
            .map(Some)
            .ok_or_else(|| "only local files are supported".to_owned()),
        Err(error) if is_cancelled(&error) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn set_initial_profile_folder(dialog: &gtk::FileDialog) -> Result<(), String> {
    let directory = profile_library_directory().map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    dialog.set_initial_folder(Some(&gio::File::for_path(directory)));
    Ok(())
}

fn is_cancelled(error: &gtk::glib::Error) -> bool {
    error.matches(gio::IOErrorEnum::Cancelled)
        || error.message().eq_ignore_ascii_case("Dismissed by user")
}

fn profile_name_from_path(path: &Path) -> Result<String, String> {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "the selected filename has no usable profile name".to_owned())
}

fn profile_requires_high_gain(profile: &Profile) -> bool {
    profile.controls.iter().any(|(name, control)| {
        name == "AE-5: Headphone Gain"
            && control
                .choice
                .as_deref()
                .is_some_and(|choice| choice.to_ascii_lowercase().starts_with("high"))
    })
}

fn profile_preview(profile: &Profile) -> String {
    const SHOWN_CONTROLS: usize = 18;
    let mut lines = profile
        .controls
        .iter()
        .take(SHOWN_CONTROLS)
        .map(|(name, control)| format!("{name}: {}", profile_control_summary(control)))
        .collect::<Vec<_>>();
    if profile.controls.len() > SHOWN_CONTROLS {
        lines.push(format!(
            "…and {} more controls",
            profile.controls.len() - SHOWN_CONTROLS
        ));
    }
    format!(
        "{} validated Linux controls:\n\n{}",
        profile.controls.len(),
        lines.join("\n")
    )
}

fn profile_control_summary(control: &ProfileControl) -> String {
    let mut values = Vec::new();
    if let Some(choice) = &control.choice {
        values.push(choice.clone());
    }
    if let Some(enabled) = control.playback_switch {
        values.push(format!("playback {}", if enabled { "on" } else { "off" }));
    }
    if let Some(level) = control.playback_level {
        values.push(format!("playback level {level}"));
    }
    if !control.playback_channels.is_empty() {
        values.push(format!(
            "playback channels {}",
            format_profile_channels(&control.playback_channels)
        ));
    }
    if let Some(enabled) = control.capture_switch {
        values.push(format!("capture {}", if enabled { "on" } else { "off" }));
    }
    if let Some(level) = control.capture_level {
        values.push(format!("capture level {level}"));
    }
    if !control.capture_channels.is_empty() {
        values.push(format!(
            "capture channels {}",
            format_profile_channels(&control.capture_channels)
        ));
    }
    values.join(", ")
}

fn format_profile_channels(channels: &std::collections::BTreeMap<String, i64>) -> String {
    channels
        .iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn control_page<'a>(
    card_index: i32,
    status: &gtk::Label,
    all_controls: &[ControlSnapshot],
    controls: impl Iterator<Item = &'a ControlSnapshot>,
) -> gtk::ScrolledWindow {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("control-list");
    for control in controls {
        let playback_switch_block = (control.playback_switch == Some(false))
            .then(|| playback_switch_block_reason(&control.name, true, all_controls))
            .flatten();
        let edit_block = direct_mode_block_reason(&control.name, all_controls)
            .or_else(|| equalizer_band_block_reason(&control.name, all_controls));
        let capture_block = capture_control_block_reason(&control.name);
        list.append(&control_row(
            card_index,
            status,
            control,
            playback_switch_block,
            edit_block,
            capture_block,
        ));
    }

    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.set_margin_top(20);
    page.set_margin_bottom(20);
    page.set_margin_start(24);
    page.set_margin_end(24);
    page.append(&list);

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn control_row(
    card_index: i32,
    status: &gtk::Label,
    control: &ControlSnapshot,
    playback_switch_block: Option<&str>,
    edit_block: Option<&str>,
    capture_block: Option<&str>,
) -> gtk::ListBoxRow {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 24);
    row.add_css_class("control-row");

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 3);
    labels.set_hexpand(true);
    let display_name = control_display_name(&control.name);
    let name = gtk::Label::new(Some(&display_name));
    name.set_xalign(0.0);
    name.set_wrap(true);
    labels.append(&name);
    let explanation = (control.name == DIRECT_MODE_CONTROL)
        .then_some(DIRECT_MODE_DESCRIPTION)
        .or(playback_switch_block)
        .or(edit_block)
        .or(capture_block);
    if let Some(message) = explanation {
        let explanation = gtk::Label::new(Some(message));
        explanation.set_xalign(0.0);
        explanation.set_wrap(true);
        explanation.add_css_class("dim-label");
        labels.append(&explanation);
    }
    row.append(&labels);

    let editors = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    editors.set_halign(gtk::Align::End);
    let high_gain_permission = if control.name == "AE-5: Headphone Gain" {
        let permission = gtk::CheckButton::with_label("Allow 150–600 Ω");
        permission.set_tooltip_text(Some(
            "Enable only when high-impedance headphones are connected.",
        ));
        editors.append(&permission);
        Some(permission)
    } else {
        None
    };
    if control.selected.is_some() {
        editors.append(&choice_editor(
            card_index,
            status,
            control,
            high_gain_permission,
            edit_block,
        ));
    }
    if let Some(enabled) = control.playback_switch {
        editors.append(&labelled(
            if control.name == DIRECT_MODE_CONTROL {
                "Enabled"
            } else {
                "Playback"
            },
            &switch_editor(
                card_index,
                status,
                &control.name,
                enabled,
                false,
                playback_switch_block,
            ),
        ));
    }
    if let Some(level) = &control.playback_level {
        editors.append(&level_editors(
            card_index,
            status,
            &control.name,
            level,
            &control.playback_channels,
            false,
            edit_block,
        ));
    }
    if let Some(enabled) = control.capture_switch {
        editors.append(&labelled(
            "Capture",
            &switch_editor(
                card_index,
                status,
                &control.name,
                enabled,
                true,
                capture_block,
            ),
        ));
    }
    if let Some(level) = &control.capture_level {
        editors.append(&level_editors(
            card_index,
            status,
            &control.name,
            level,
            &control.capture_channels,
            true,
            capture_block,
        ));
    }
    row.append(&editors);

    let list_row = gtk::ListBoxRow::builder()
        .activatable(false)
        .selectable(false)
        .child(&row)
        .build();
    let description = control_row_description(control, explanation);
    list_row.update_property(&[
        gtk::accessible::Property::Label(&display_name),
        gtk::accessible::Property::Description(&description),
    ]);
    list_row
}

fn control_display_name(name: &str) -> String {
    let Some(index) = name
        .strip_prefix("EQ Band")
        .and_then(|value| value.parse::<usize>().ok())
    else {
        return name.to_owned();
    };
    EQ_BAND_LABELS
        .get(index)
        .map_or_else(|| name.to_owned(), |label| format!("{name} · {label}"))
}

fn control_row_description(
    control: &ControlSnapshot,
    playback_switch_block: Option<&str>,
) -> String {
    let mut description = format!("Current state: {control}");
    if let Some(reason) = playback_switch_block {
        description.push_str(". Unavailable: ");
        description.push_str(reason);
    }
    description
}

fn choice_editor(
    card_index: i32,
    status: &gtk::Label,
    control: &ControlSnapshot,
    high_gain_permission: Option<gtk::CheckButton>,
    block_reason: Option<&str>,
) -> gtk::DropDown {
    let choices = control
        .choices
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let dropdown = gtk::DropDown::from_strings(&choices);
    dropdown.update_property(&[gtk::accessible::Property::Label(&format!(
        "{} choice",
        control.name
    ))]);
    let selected = control
        .selected
        .as_ref()
        .and_then(|selected| control.choices.iter().position(|choice| choice == selected))
        .unwrap_or_default() as u32;
    dropdown.set_selected(selected);
    dropdown.set_tooltip_text(Some("Changes are written and read back immediately."));
    if let Some(reason) = block_reason {
        dropdown.set_sensitive(false);
        dropdown.set_tooltip_text(Some(reason));
    }

    let verified = Rc::new(Cell::new(selected));
    let updating = Rc::new(Cell::new(false));
    let name = control.name.clone();
    let choices = control.choices.clone();
    let status = status.clone();
    dropdown.connect_selected_notify(move |dropdown| {
        if updating.get() {
            return;
        }
        let requested_index = dropdown.selected();
        let Some(requested) = choices.get(requested_index as usize) else {
            return;
        };
        let allow_high_gain = high_gain_permission
            .as_ref()
            .is_some_and(gtk::CheckButton::is_active);
        if is_high_gain(&name, requested) && !allow_high_gain {
            revert_dropdown(dropdown, &updating, verified.get());
            set_status(
                &status,
                false,
                "High gain was not applied. Enable “Allow 150–600 Ω” first.",
            );
            return;
        }

        match with_mixer(card_index, |mixer| {
            mixer.set_choice_checked(&name, requested, allow_high_gain)
        }) {
            Ok(actual) => {
                verified.set(requested_index);
                set_status(
                    &status,
                    true,
                    &format!("Applied and verified: {}", control_summary(&actual)),
                );
            }
            Err(error) => {
                revert_dropdown(dropdown, &updating, verified.get());
                set_status(&status, false, &format!("Change failed: {error}"));
            }
        }
    });
    dropdown
}

fn switch_editor(
    card_index: i32,
    status: &gtk::Label,
    name: &str,
    enabled: bool,
    capture: bool,
    block_reason: Option<&str>,
) -> gtk::Switch {
    let control = gtk::Switch::builder()
        .active(enabled)
        .valign(gtk::Align::Center)
        .build();
    if let Some(reason) = block_reason {
        control.set_sensitive(false);
        control.set_tooltip_text(Some(reason));
    }
    control.update_property(&[gtk::accessible::Property::Label(&format!(
        "{} {} switch",
        name,
        if capture { "capture" } else { "playback" }
    ))]);
    let verified = Rc::new(Cell::new(enabled));
    let updating = Rc::new(Cell::new(false));
    let name = name.to_owned();
    let status = status.clone();
    control.connect_active_notify(move |control| {
        if updating.get() {
            return;
        }
        let requested = control.is_active();
        let result = with_mixer(card_index, |mixer| {
            if capture {
                mixer.set_capture_switch(&name, requested)
            } else {
                mixer.set_playback_switch(&name, requested)
            }
        });
        match result {
            Ok(actual) => {
                verified.set(requested);
                set_status(
                    &status,
                    true,
                    &format!("Applied and verified: {}", control_summary(&actual)),
                );
            }
            Err(error) => {
                updating.set(true);
                control.set_active(verified.get());
                updating.set(false);
                set_status(&status, false, &format!("Change failed: {error}"));
            }
        }
    });
    control
}

fn level_editors(
    card_index: i32,
    status: &gtk::Label,
    name: &str,
    level: &Level,
    channels: &[ChannelLevel],
    capture: bool,
    block_reason: Option<&str>,
) -> gtk::Widget {
    if channels.len() < 2 {
        return level_editor(card_index, status, name, level, capture, None, block_reason);
    }

    let group = gtk::Box::new(gtk::Orientation::Vertical, 6);
    for channel in channels {
        let channel_level = Level {
            value: channel.value,
            min: level.min,
            max: level.max,
        };
        let editor = level_editor(
            card_index,
            status,
            name,
            &channel_level,
            capture,
            Some(&channel.name),
            block_reason,
        );
        group.append(&labelled(
            &format!(
                "{} · {}",
                if capture { "Capture" } else { "Playback" },
                channel.name
            ),
            &editor,
        ));
    }
    group.upcast()
}

fn level_editor(
    card_index: i32,
    status: &gtk::Label,
    name: &str,
    level: &Level,
    capture: bool,
    channel: Option<&str>,
    block_reason: Option<&str>,
) -> gtk::Widget {
    if !(level.min..=level.max).contains(&level.value) {
        let warning = gtk::Label::new(Some(&format!(
            "{} (driver reports {}..{})",
            level.value, level.min, level.max
        )));
        warning.add_css_class("warning-value");
        warning.set_tooltip_text(Some(
            "The driver returned an out-of-range value; editing is disabled.",
        ));
        return warning.upcast();
    }

    let scale = gtk::Scale::with_range(
        gtk::Orientation::Horizontal,
        level.min as f64,
        level.max as f64,
        1.0,
    );
    scale.set_value(level.value as f64);
    scale.set_digits(0);
    scale.set_draw_value(true);
    scale.set_width_request(210);
    if let Some(reason) = block_reason {
        scale.set_sensitive(false);
        scale.set_tooltip_text(Some(reason));
    }
    let accessible_label = match channel {
        Some(channel) => format!(
            "{} {} {} level",
            name,
            if capture { "capture" } else { "playback" },
            channel
        ),
        None => format!(
            "{} {} level",
            name,
            if capture { "capture" } else { "playback" }
        ),
    };
    scale.update_property(&[gtk::accessible::Property::Label(&accessible_label)]);

    let verified = Rc::new(Cell::new(level.value));
    let updating = Rc::new(Cell::new(false));
    let pending = Rc::new(RefCell::new(None::<gtk::glib::SourceId>));
    let name = name.to_owned();
    let channel = channel.map(str::to_owned);
    let status = status.clone();
    scale.connect_value_changed(move |scale| {
        if updating.get() {
            return;
        }
        if let Some(source) = pending.borrow_mut().take() {
            source.remove();
        }
        let requested = scale.value().round() as i64;
        let scale = scale.clone();
        let verified = verified.clone();
        let updating = updating.clone();
        let pending_for_timeout = pending.clone();
        let name = name.clone();
        let channel = channel.clone();
        let status = status.clone();
        let source = gtk::glib::timeout_add_local_once(Duration::from_millis(160), move || {
            pending_for_timeout.borrow_mut().take();
            let result = with_mixer(card_index, |mixer| match (capture, channel.as_deref()) {
                (true, Some(channel)) => mixer.set_capture_channel_level(&name, channel, requested),
                (false, Some(channel)) => {
                    mixer.set_playback_channel_level(&name, channel, requested)
                }
                (true, None) => mixer.set_capture_level(&name, requested),
                (false, None) => mixer.set_playback_level(&name, requested),
            });
            match result {
                Ok(actual) => {
                    verified.set(requested);
                    set_status(
                        &status,
                        true,
                        &format!("Applied and verified: {}", control_summary(&actual)),
                    );
                }
                Err(error) => {
                    updating.set(true);
                    scale.set_value(verified.get() as f64);
                    updating.set(false);
                    set_status(&status, false, &format!("Change failed: {error}"));
                }
            }
        });
        *pending.borrow_mut() = Some(source);
    });
    scale.upcast()
}

fn labelled(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::Box {
    let group = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let label = gtk::Label::new(Some(label));
    label.add_css_class("dim-label");
    group.append(&label);
    group.append(widget);
    group
}

fn with_mixer<T>(
    card_index: i32,
    operation: impl FnOnce(&Ae5Mixer) -> Result<T, ControlError>,
) -> Result<T, String> {
    let mixer = Ae5Mixer::open(card_index).map_err(|error| error.to_string())?;
    operation(&mixer).map_err(|error| error.to_string())
}

fn is_high_gain(name: &str, requested: &str) -> bool {
    name == "AE-5: Headphone Gain" && requested.to_ascii_lowercase().starts_with("high")
}

fn revert_dropdown(dropdown: &gtk::DropDown, updating: &Cell<bool>, selected: u32) {
    updating.set(true);
    dropdown.set_selected(selected);
    updating.set(false);
}

fn control_summary(control: &ControlSnapshot) -> String {
    control.to_string()
}

fn set_status(status: &gtk::Label, success: bool, message: &str) {
    status.remove_css_class("operation-ok");
    status.remove_css_class("operation-error");
    status.add_css_class(if success {
        "operation-ok"
    } else {
        "operation-error"
    });
    status.set_text(message);
}

fn start_mixer_watch(window: &gtk::ApplicationWindow, card_index: i32) -> Result<(), String> {
    let mixer = Ae5Mixer::open(card_index).map_err(|error| error.to_string())?;
    let running = Arc::new(AtomicBool::new(true));
    let refresh_queued = Arc::new(AtomicBool::new(false));

    let running_on_close = running.clone();
    window.connect_close_request(move |_| {
        running_on_close.store(false, Ordering::Release);
        gtk::glib::Propagation::Proceed
    });

    thread::Builder::new()
        .name("ae5-mixer-events".to_owned())
        .spawn(move || {
            while running.load(Ordering::Acquire) {
                match mixer.wait_for_event(Duration::from_millis(500)) {
                    Ok(false) => {}
                    Ok(true) if !refresh_queued.swap(true, Ordering::AcqRel) => {
                        let refresh_queued = refresh_queued.clone();
                        gtk::glib::MainContext::default().invoke(move || {
                            refresh_queued.store(false, Ordering::Release);
                            if let Some(window) = active_main_window() {
                                let _ = refresh_window(
                                    &window,
                                    Some("Synchronized after an ALSA mixer event."),
                                );
                            }
                        });
                    }
                    Ok(true) => {}
                    Err(error) => {
                        let message = format!("Live synchronization stopped: {error}");
                        gtk::glib::MainContext::default().invoke(move || {
                            if let Some(window) = active_main_window() {
                                set_main_status(&window, false, &message);
                            }
                        });
                        break;
                    }
                }
            }
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn active_main_window() -> Option<gtk::ApplicationWindow> {
    gio::Application::default()?
        .downcast::<gtk::Application>()
        .ok()?
        .active_window()?
        .downcast()
        .ok()
}

fn main_stack(window: &gtk::ApplicationWindow) -> Option<gtk::Stack> {
    find_widget(window.child()?, |widget| {
        widget
            .downcast_ref::<gtk::Stack>()
            .is_some_and(|stack| stack.widget_name() == MAIN_STACK_NAME)
    })?
    .downcast()
    .ok()
}

fn set_main_status(window: &gtk::ApplicationWindow, success: bool, message: &str) {
    if let Some(status) = find_widget(
        window.child().unwrap_or_else(|| window.clone().upcast()),
        |widget| widget.has_css_class("operation-status"),
    )
    .and_then(|widget| widget.downcast::<gtk::Label>().ok())
    {
        set_status(&status, success, message);
    }
}

fn find_widget(root: gtk::Widget, predicate: impl Fn(&gtk::Widget) -> bool) -> Option<gtk::Widget> {
    let mut pending = vec![root];
    while let Some(widget) = pending.pop() {
        if predicate(&widget) {
            return Some(widget);
        }
        let mut child = widget.first_child();
        while let Some(current) = child {
            child = current.next_sibling();
            pending.push(current);
        }
    }
    None
}

fn error_view(message: &str) -> gtk::Box {
    let view = gtk::Box::new(gtk::Orientation::Vertical, 12);
    view.set_valign(gtk::Align::Center);
    view.set_halign(gtk::Align::Center);
    view.set_margin_start(32);
    view.set_margin_end(32);

    let title = gtk::Label::new(Some("AE-5 unavailable"));
    title.add_css_class("hero-title");
    let detail = gtk::Label::new(Some(message));
    detail.set_wrap(true);
    detail.add_css_class("dim-label");
    view.append(&title);
    view.append(&detail);
    view
}

#[derive(Copy, Clone)]
enum Category {
    Playback,
    Effects,
    Equalizer,
    Recording,
}

impl Category {
    const ALL: [Self; 4] = [
        Self::Playback,
        Self::Effects,
        Self::Equalizer,
        Self::Recording,
    ];

    fn id(self) -> &'static str {
        match self {
            Self::Playback => "playback",
            Self::Effects => "effects",
            Self::Equalizer => "equalizer",
            Self::Recording => "recording",
        }
    }

    fn matches(self, name: &str) -> bool {
        match self {
            Self::Equalizer => name.starts_with("EQ Band") || name == "FX: Equalizer Preset",
            Self::Recording => {
                name.contains("Capture")
                    || name.starts_with("Input")
                    || name.starts_with("Mic ")
                    || name.starts_with("SVM ")
                    || name.starts_with("Voice")
                    || name.starts_with("Wedge")
                    || name == "Enable InFX"
                    || name.starts_with("FX: Mic")
                    || name.starts_with("FX: Noise")
                    || name.starts_with("FX: Voice")
                    || name == "What U Hear"
            }
            Self::Effects => {
                !Self::Recording.matches(name)
                    && (name == "Enable OutFX"
                        || (name.starts_with("FX:") && name != "FX: Equalizer Preset"))
            }
            Self::Playback => {
                !Self::Effects.matches(name)
                    && !Self::Equalizer.matches(name)
                    && !Self::Recording.matches(name)
            }
        }
    }
}

fn install_css() {
    let Some(display) = Display::default() else {
        return;
    };
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "
        window { background: #11161c; color: #e9eef5; }
        .hero { padding: 28px 30px; background: #18212b; }
        .hero-title { font-size: 24px; font-weight: 700; }
        .dim-label { color: #9daebe; }
        .status-pill {
            background: #173d35;
            color: #8ee3c5;
            border-radius: 999px;
            padding: 7px 12px;
            font-weight: 600;
        }
        .operation-status {
            padding: 8px 30px;
            background: #11161c;
            color: #9daebe;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        .operation-ok { color: #8ee3c5; }
        .operation-error, .warning-label, .warning-value { color: #ffb4a9; }
        .navigation-sidebar { background: #141b22; padding: 12px 8px; }
        .profile-page { padding: 26px 30px; }
        .page-title { font-size: 22px; font-weight: 700; }
        .profile-card {
            background: #151d25;
            border: 1px solid alpha(#ffffff, 0.10);
            border-left: 3px solid #39d0aa;
            border-radius: 4px;
            padding: 20px;
        }
        .profile-library-row {
            padding: 10px 0;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        .feature-entry {
            padding: 8px 0;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        .section-index {
            background: #173d35;
            color: #8ee3c5;
            border-radius: 3px;
            padding: 4px 7px;
            font-family: monospace;
            font-weight: 700;
        }
        .section-title { font-size: 17px; font-weight: 700; }
        .control-list { background: transparent; }
        .control-row {
            padding: 14px 16px;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        scale { min-width: 210px; }
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

#[cfg(test)]
mod tests {
    use super::*;
    use ae5_control::SbCommandImportReport;

    #[test]
    fn compatibility_page_summarizes_the_embedded_matrix() {
        let summary = compatibility_summary();
        let features = feature_parity().collect::<Vec<_>>();
        assert!(summary.contains(&format!("{} tracked features", features.len())));
        for (label, support) in [
            ("Verified", FeatureSupport::Verified),
            ("Linux-native equivalents", FeatureSupport::Substituted),
            ("Pending acceptance", FeatureSupport::Deferred),
            ("Unavailable", FeatureSupport::Unsupported),
        ] {
            let count = features
                .iter()
                .filter(|feature| feature.support == support)
                .count();
            assert!(summary.contains(&format!("{label}: {count}")));
        }
    }

    #[test]
    fn every_control_category_is_exclusive() {
        for name in [
            "Output Select",
            "FX: Crystalizer",
            "EQ Band0",
            "FX: Noise Reduction",
        ] {
            assert_eq!(
                Category::ALL
                    .iter()
                    .filter(|category| category.matches(name))
                    .count(),
                1
            );
        }
    }

    #[test]
    fn labels_equalizer_frequencies_and_command_aliases() {
        assert_eq!(control_display_name("EQ Band0"), "EQ Band0 · 31 Hz");
        assert_eq!(
            control_display_name("EQ Band1"),
            "EQ Band1 · 62 Hz (Bass in Command)"
        );
        assert_eq!(
            control_display_name("EQ Band8"),
            "EQ Band8 · 8 kHz (Treble in Command)"
        );
        assert_eq!(control_display_name("EQ Band9"), "EQ Band9 · 16 kHz");
        assert_eq!(control_display_name("EQ Band10"), "EQ Band10");
        assert_eq!(control_display_name("Master"), "Master");
    }

    #[test]
    fn summarizes_pipewire_default_state() {
        let node = PipeWireNode {
            id: 58,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5 Analog Stereo".to_owned(),
            is_default: true,
        };

        assert_eq!(
            pipewire_node_summary(&node),
            "AE-5 Analog Stereo\nalsa_output.pci-ae5.analog-stereo — currently default"
        );
    }

    #[test]
    fn reports_matched_and_split_desktop_routes() {
        let controls = [
            ControlSnapshot {
                name: "Output Select".to_owned(),
                selected: Some("Headphone".to_owned()),
                choices: vec!["Speakers".to_owned(), "Headphone".to_owned()],
                playback_switch: None,
                capture_switch: None,
                playback_level: None,
                capture_level: None,
                playback_channels: Vec::new(),
                capture_channels: Vec::new(),
            },
            ControlSnapshot {
                name: "Input Source".to_owned(),
                selected: Some("Microphone".to_owned()),
                choices: vec![
                    "Microphone".to_owned(),
                    "Line In".to_owned(),
                    "Front Microphone".to_owned(),
                ],
                playback_switch: None,
                capture_switch: None,
                playback_level: None,
                capture_level: None,
                playback_channels: Vec::new(),
                capture_channels: Vec::new(),
            },
            ControlSnapshot {
                name: "Surround Channel Config".to_owned(),
                selected: Some("2.0".to_owned()),
                choices: vec![
                    "2.0".to_owned(),
                    "2.1".to_owned(),
                    "4.0".to_owned(),
                    "4.1".to_owned(),
                    "5.1".to_owned(),
                ],
                playback_switch: None,
                capture_switch: None,
                playback_level: None,
                capture_level: None,
                playback_channels: Vec::new(),
                capture_channels: Vec::new(),
            },
        ];
        let mut state = PipeWireRouteState {
            profile_set: Some("sound-blaster-ae5.conf".to_owned()),
            soft_mixer: Some(true),
            active_profile: Some("output:analog-stereo+input:analog-stereo".to_owned()),
            input_route: Some("sound-blaster-ae5-input-microphone".to_owned()),
            output_route: Some("sound-blaster-ae5-output-headphones;output-headphones".to_owned()),
        };

        let (summary, healthy) = route_health_summary(&controls, Ok(state.clone()));
        assert!(healthy);
        assert!(summary.contains("Matched\nALSA output: Headphone"));
        assert!(summary.contains("ALSA input: Microphone"));
        assert!(summary.contains("sound-blaster-ae5.conf"));

        state.output_route = Some("analog-output-lineout;output-speaker".to_owned());
        let (summary, healthy) = route_health_summary(&controls, Ok(state.clone()));
        assert!(!healthy);
        assert!(summary.contains("Needs attention"));
        assert!(summary.contains("reapply the output choice"));

        state.output_route =
            Some("sound-blaster-ae5-output-headphones;output-headphones".to_owned());
        state.input_route = Some("sound-blaster-ae5-input-line-in".to_owned());
        let (summary, healthy) = route_health_summary(&controls, Ok(state));
        assert!(!healthy);
        assert!(summary.contains("reapply the input choice"));
    }

    #[test]
    fn summarizes_native_rate_configuration() {
        let mut config = NativeRatesConfig {
            path: PathBuf::from("/tmp/91-ae5-control-rates.conf"),
            enabled: false,
        };
        assert_eq!(
            native_rates_summary(&config),
            "Disabled\nUsing the distribution PipeWire defaults"
        );

        config.enabled = true;
        assert_eq!(
            native_rates_summary(&config),
            "Enabled in PipeWire configuration\n44.1, 48, and 96 kHz"
        );
    }

    #[test]
    fn round_trips_lighting_colors_through_gtk() {
        for color in [
            RgbColor::new(0, 0, 0),
            RgbColor::new(255, 255, 255),
            RgbColor::new(12, 127, 241),
        ] {
            assert_eq!(rgb_from_rgba(&rgba_from_rgb(color)), color);
        }
    }

    #[test]
    fn profile_helpers_name_preview_and_protect_high_gain() {
        let profile = Profile {
            format_version: 1,
            name: "Test".to_owned(),
            target: "1102:0012/1102:0051".to_owned(),
            controls: std::collections::BTreeMap::from([
                (
                    "AE-5: Headphone Gain".to_owned(),
                    ProfileControl {
                        choice: Some("High (150-600 Ohms)".to_owned()),
                        ..ProfileControl::default()
                    },
                ),
                (
                    "Front".to_owned(),
                    ProfileControl {
                        playback_level: Some(90),
                        playback_channels: std::collections::BTreeMap::from([
                            ("Front Left".to_owned(), 90),
                            ("Front Right".to_owned(), 82),
                        ]),
                        ..ProfileControl::default()
                    },
                ),
            ]),
        };

        assert_eq!(
            profile_name_from_path(Path::new("/tmp/Studio headphones.json")).unwrap(),
            "Studio headphones"
        );
        assert!(profile_requires_high_gain(&profile));
        assert!(profile_preview(&profile).contains("High (150-600 Ohms)"));
        assert!(profile_preview(&profile).contains("Front Right=82"));
    }

    #[test]
    fn linux_default_preview_names_preserved_state_and_backup_location() {
        let profile = linux_driver_defaults_for(&[]).unwrap();
        let preview =
            linux_driver_defaults_preview(&profile, Path::new("/tmp/ae5-control/profiles"));

        assert!(preview.contains("29 validated Linux controls"));
        assert!(preview.contains("output selection and headphone auto-detect"));
        assert!(preview.contains("playback and capture volumes"));
        assert!(preview.contains("/tmp/ae5-control/profiles"));
    }

    #[test]
    fn migration_preview_separates_categories_and_keeps_every_unsupported_item() {
        let import = SbCommandImport {
            profile: Profile {
                format_version: 1,
                name: "Windows headphones".to_owned(),
                target: "1102:0012/1102:0051".to_owned(),
                controls: std::collections::BTreeMap::from([(
                    "FX: Surround".to_owned(),
                    ProfileControl {
                        playback_level: Some(68),
                        ..ProfileControl::default()
                    },
                )]),
            },
            report: SbCommandImportReport {
                exact: [
                    "Sound Blaster Command 3.5.10.0 → active configuration".to_owned(),
                    "Creative AE-5 driver 6.0.105.0065 → active Windows driver package".to_owned(),
                ]
                .into_iter()
                .chain((0..10).map(|index| format!("exact mapping {index}")))
                .collect(),
                approximate: vec!["surround 67.5 → 68".to_owned()],
                unsupported: (0..20)
                    .map(|index| format!("unsupported {index}"))
                    .collect(),
            },
        };

        let preview = migration_preview(&import);
        assert!(preview.contains("Exact mappings (12)"));
        assert!(preview.contains("Sound Blaster Command 3.5.10.0"));
        assert!(preview.contains("Creative AE-5 driver 6.0.105.0065"));
        assert!(preview.contains("Approximate mappings (1)"));
        assert!(preview.contains("Unsupported settings (20)"));
        assert!(preview.contains("unsupported 19"));
    }

    #[test]
    fn treats_gio_and_kde_portal_dismissals_as_cancellation() {
        let gio_cancelled =
            gtk::glib::Error::new(gio::IOErrorEnum::Cancelled, "Operation was cancelled");
        let portal_cancelled = gtk::glib::Error::new(gio::IOErrorEnum::Failed, "Dismissed by user");

        assert!(is_cancelled(&gio_cancelled));
        assert!(is_cancelled(&portal_cancelled));
    }

    #[test]
    fn diagnostics_path_is_one_literal_process_argument() {
        let path = Path::new("/tmp/ae5 report;touch nope.txt");
        let argv = diagnostics_argv(path);

        assert_eq!(argv[0], std::ffi::OsStr::new("ae5-collect-report"));
        assert_eq!(argv[1], path.as_os_str());
    }

    #[test]
    fn reports_driver_values_outside_their_declared_range() {
        let controls = vec![ControlSnapshot {
            name: "Wedge Angle".to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: None,
            capture_switch: None,
            playback_level: None,
            capture_level: Some(Level {
                value: 10,
                min: 20,
                max: 180,
            }),
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        }];

        assert_eq!(
            driver_range_warnings(&controls),
            ["Wedge Angle capture value 10 is outside 20..180"]
        );
    }

    #[test]
    fn control_rows_describe_current_and_blocked_state() {
        let control = ControlSnapshot {
            name: "Bass Redirection".to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: Some(false),
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        };

        assert_eq!(
            control_row_description(
                &control,
                Some("Select Speakers output before enabling bass redirection.")
            ),
            "Current state: Bass Redirection | playback off. Unavailable: Select Speakers output before enabling bass redirection."
        );
    }
}
