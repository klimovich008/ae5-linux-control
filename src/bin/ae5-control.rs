#[cfg(test)]
use ae5_control::linux_driver_defaults_for;
use ae5_control::{
    Ae5Device, Ae5Lighting, Ae5Mixer, BuiltinProfile, COMMAND_DEFAULT_PROFILE_COUNT, ChannelLevel,
    ControlError, ControlSnapshot, DIRECT_MODE_CONTROL, FeatureSupport,
    LINUX_DRIVER_DEFAULTS_PRESERVED, Level, NativeRatesConfig, ONBOARD_LED_COUNT, PipeWireNode,
    PipeWireRouteState, Profile, ProfileControl, RgbColor, SbCommandImport, SbCommandTarget,
    ae5_input, ae5_output, ae5_route_state, apply_linux_driver_defaults, builtin_profiles,
    capture_control_block_reason, direct_mode_block_reason, discover_sbcommand_installation,
    equalizer_band_block_reason, export_library_profile, feature_parity,
    front_vmaster_clamp_warning, headphone_playback_issue,
    import_discovered_sbcommand_profile_with_report, import_sbcommand_profile_with_report,
    library_profile, native_rates_config, playback_switch_block_reason, profile_library,
    profile_library_directory, rename_library_profile, set_ae5_default_input,
    set_ae5_default_output, set_native_rates_enabled, set_saved_led, set_saved_lighting,
    smart_volume_level_block_reason, snapshot_controls, validate_linux_driver_defaults,
};
use gtk::prelude::*;
use gtk::{gdk::Display, gio};
use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::fs;
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
const BUILTIN_PROFILE_PICKER_NAME: &str = "builtin-profile-picker";
const PERSONAL_PROFILE_CAROUSEL_NAME: &str = "personal-profile-carousel";
const BUILTIN_PROFILE_CAROUSEL_NAME: &str = "builtin-profile-carousel";
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum KernelReadiness {
    Ready,
    Stock,
    Attention,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct OperationStatus {
    success: bool,
    message: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct RefreshViewState {
    visible_page: Option<String>,
    builtin_profile: Option<u32>,
    personal_profile_scroll: Option<f64>,
    builtin_profile_scroll: Option<f64>,
    operation_status: Option<OperationStatus>,
}

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
        .default_width(1120)
        .default_height(700)
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
    let view_state = capture_refresh_view_state(window);
    let operation_status =
        operation_status_for_refresh(message, view_state.operation_status.clone());
    match load_hardware() {
        Ok((device, controls)) => {
            let card_index = device.card_index;
            window.set_child(Some(&content(
                window,
                &device,
                &controls,
                operation_status
                    .as_ref()
                    .map(|status| status.message.as_str()),
                view_state.visible_page.as_deref(),
            )));
            restore_refresh_view_state(window, &view_state);
            if let Some(status) = operation_status {
                set_main_status(window, status.success, &status.message);
            }
            Some(card_index)
        }
        Err(error) => {
            window.set_child(Some(&error_view(window, &error)));
            None
        }
    }
}

fn operation_status_for_refresh(
    message: Option<&str>,
    retained: Option<OperationStatus>,
) -> Option<OperationStatus> {
    message
        .map(|message| OperationStatus {
            success: true,
            message: message.to_owned(),
        })
        .or(retained)
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
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    root.add_css_class("application-shell");
    let status = gtk::Label::new(Some(
        message.unwrap_or("Ready — every change is verified against the hardware."),
    ));
    status.set_xalign(0.0);
    status.set_hexpand(true);
    status.set_ellipsize(gtk::pango::EllipsizeMode::End);
    status.add_css_class("operation-status");

    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .hexpand(true)
        .vexpand(true)
        .build();
    stack.set_widget_name(MAIN_STACK_NAME);

    for (name, title) in [
        ("effects", "Sound effects"),
        ("equalizer", "Equalizer"),
        ("playback", "Playback"),
        ("recording", "Recording"),
        ("scout", "Scout Mode"),
        ("mixer", "Mixer"),
        ("lighting", "Lighting"),
        ("profiles", "Profiles"),
        ("settings", "Settings"),
    ] {
        let holder = gtk::Box::new(gtk::Orientation::Vertical, 0);
        holder.set_hexpand(true);
        holder.set_vexpand(true);
        stack.add_titled(&holder, Some(name), title);
    }

    let initial_page = visible_page.unwrap_or("effects");
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
        .width_request(208)
        .vexpand(true)
        .build();
    sidebar.add_css_class("navigation-sidebar");

    let sidebar_panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar_panel.add_css_class("sidebar-panel");
    sidebar_panel.append(&sidebar_brand(device));
    sidebar_panel.append(&sidebar);
    let sidebar_footer = gtk::Label::new(Some("LINUX NATIVE  ·  WAYLAND"));
    sidebar_footer.set_xalign(0.0);
    sidebar_footer.add_css_class("sidebar-footer");
    sidebar_panel.append(&sidebar_footer);

    let main = gtk::Box::new(gtk::Orientation::Vertical, 0);
    main.set_hexpand(true);
    main.set_vexpand(true);
    main.add_css_class("main-panel");
    main.append(&hero(device, controls));
    main.append(&stack);
    main.append(&status_rail(&status, device.card_index, controls));

    root.append(&sidebar_panel);
    root.append(&main);
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
        "settings" => settings_page(window, device, controls, status).upcast(),
        "scout" => scout_page().upcast(),
        "mixer" => mixer_page(device.card_index, status, controls).upcast(),
        "equalizer" => equalizer_page(device.card_index, status, controls).upcast(),
        "effects" => sound_effects_page(window, device.card_index, status, controls).upcast(),
        "playback" => playback_page(device.card_index, status, controls).upcast(),
        "recording" => recording_page(device.card_index, status, controls).upcast(),
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
                category,
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

fn sidebar_brand(device: &Ae5Device) -> gtk::Box {
    let brand = gtk::Box::new(gtk::Orientation::Vertical, 3);
    brand.add_css_class("sidebar-brand");

    let title = gtk::Label::new(Some("AE-5 CONTROL"));
    title.set_xalign(0.0);
    title.add_css_class("sidebar-title");
    let device = gtk::Label::new(Some(
        device
            .codec_name
            .as_deref()
            .unwrap_or("Sound BlasterX AE-5"),
    ));
    device.set_xalign(0.0);
    device.set_ellipsize(gtk::pango::EllipsizeMode::End);
    device.add_css_class("sidebar-device");

    brand.append(&title);
    brand.append(&device);
    brand
}

fn status_rail(status: &gtk::Label, card_index: i32, controls: &[ControlSnapshot]) -> gtk::Box {
    let rail = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    rail.add_css_class("status-rail");

    let mark = gtk::Label::new(Some("AE-5"));
    mark.add_css_class("status-mark");
    rail.append(&mark);
    rail.append(status);

    let output = controls
        .iter()
        .find(|control| control.name == "Master")
        .map(
            |control| match (&control.playback_level, control.playback_switch) {
                (_, Some(false)) => "OUTPUT MUTED".to_owned(),
                (Some(level), _) => format!("HARDWARE {}", hardware_level_label(level)),
                _ => "OUTPUT ACTIVE".to_owned(),
            },
        )
        .unwrap_or_else(|| "OUTPUT UNKNOWN".to_owned());
    let output = gtk::Label::new(Some(&output));
    output.add_css_class("output-state");
    rail.append(&output);
    if let Some(route) = controls
        .iter()
        .find(|control| control.name == "Output Select")
    {
        rail.append(&footer_output_selector(card_index, status, route));
    }
    rail
}

fn hardware_level_label(level: &Level) -> String {
    if level.value == level.max {
        "0 dB".to_owned()
    } else {
        format!("{}/{} raw", level.value, level.max)
    }
}

fn hero(device: &Ae5Device, controls: &[ControlSnapshot]) -> gtk::Box {
    let header = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    header.add_css_class("hero");

    let titles = gtk::Box::new(gtk::Orientation::Vertical, 2);
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

    let status = gtk::Label::new(Some(&format!("ONLINE · {} CONTROLS", controls.len())));
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

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Device & diagnostics"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    heading.append(&title);
    let matched = gtk::Label::new(Some("PCI DEVICE MATCHED"));
    matched.add_css_class("status-pill");
    heading.append(&matched);
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

    let release = fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|_| "unavailable".to_owned());
    let taint = fs::read_to_string("/proc/sys/kernel/tainted")
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok());
    let direct_mode = controls
        .iter()
        .any(|control| control.name == DIRECT_MODE_CONTROL);
    let lighting = Ae5Lighting::discover().is_ok();
    let (kernel_summary, kernel_readiness) =
        kernel_readiness_summary(&release, taint, direct_mode, lighting);
    let kernel_status = gtk::Label::new(Some(&kernel_summary));
    kernel_status.set_xalign(0.0);
    kernel_status.set_wrap(true);
    kernel_status.set_selectable(true);
    kernel_status.add_css_class(match kernel_readiness {
        KernelReadiness::Ready => "operation-ok",
        KernelReadiness::Stock => "dim-label",
        KernelReadiness::Attention => "warning-value",
    });
    let kernel_details = gtk::Box::new(gtk::Orientation::Vertical, 0);
    kernel_details.append(&kernel_status);
    page.append(&profile_card(
        "02",
        "Kernel & project interfaces",
        "Read-only boot readiness. Interface detection does not replace physical driver acceptance.",
        &kernel_details,
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
        "03",
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
    if !route_healthy {
        let repair = gtk::Button::with_label("Repair current route");
        repair.set_halign(gtk::Align::Start);
        repair.set_tooltip_text(Some(
            "Explicitly reapplies the current ALSA/PipeWire routes and may unmute hardware Master and the Front DAC.",
        ));
        route_actions.append(&repair);

        let card_index = device.card_index;
        let status = status.clone();
        let window = window.clone();
        repair.connect_clicked(move |button| {
            button.set_sensitive(false);
            let result = Ae5Mixer::open(card_index)
                .map_err(ControlError::from)
                .and_then(|mixer| mixer.repair_routes());
            match result {
                Ok(changes) => {
                    let message = if changes.is_empty() {
                        "Desktop routes were already healthy; no changes made.".to_owned()
                    } else {
                        format!("Desktop route repaired: {}.", changes.join(", "))
                    };
                    let _ = refresh_window(&window, Some(&message));
                }
                Err(error) => {
                    set_status(&status, false, &format!("Route repair failed: {error}"));
                    button.set_sensitive(true);
                }
            }
        });
    }
    page.append(&profile_card(
        "04",
        "Desktop route health",
        "The check is read-only. The repair action is explicit and may unmute hardware Master or Front when Headphone output requires it.",
        &route_actions,
    ));

    let report_actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let save_report = gtk::Button::with_label("Save diagnostics report");
    report_actions.append(&save_report);
    page.append(&profile_card(
        "05",
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

fn kernel_readiness_summary(
    release: &str,
    taint: Option<u64>,
    direct_mode: bool,
    lighting: bool,
) -> (String, KernelReadiness) {
    let taint_text = match taint {
        Some(0) => "0 (clean)".to_owned(),
        Some(value) => format!("{value} (review before driver testing)"),
        None => "unavailable".to_owned(),
    };
    let readiness = match taint {
        Some(0) if direct_mode && lighting => KernelReadiness::Ready,
        Some(0) => KernelReadiness::Stock,
        _ => KernelReadiness::Attention,
    };
    let direct_mode_text = if direct_mode {
        "available"
    } else {
        "unavailable"
    };
    let lighting_text = if lighting {
        "available (five LEDs)"
    } else {
        "unavailable"
    };
    let conclusion = match readiness {
        KernelReadiness::Ready => {
            "Project interfaces detected; physical acceptance is still pending."
        }
        KernelReadiness::Stock => {
            "Stock-compatible control path active; project-only interfaces are not all present."
        }
        KernelReadiness::Attention => "Kernel state needs review before physical driver testing.",
    };

    (
        format!(
            "Running kernel: {release}\nKernel taint: {taint_text}\n\
             Direct Mode: {direct_mode_text}\nOnboard lighting: {lighting_text}\n{conclusion}"
        ),
        readiness,
    )
}

fn settings_page(
    window: &gtk::ApplicationWindow,
    device: &Ae5Device,
    controls: &[ControlSnapshot],
    status: &gtk::Label,
) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let header = gtk::Box::new(gtk::Orientation::Vertical, 12);
    header.add_css_class("settings-header");
    let heading = gtk::Label::new(Some("Settings"));
    heading.set_xalign(0.0);
    heading.add_css_class("page-title");
    header.append(&heading);

    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .hexpand(true)
        .vexpand(true)
        .build();
    stack.add_titled(
        &device_page(window, device, controls, status),
        Some("device"),
        "Device",
    );
    stack.add_titled(
        &compatibility_page(),
        Some("compatibility"),
        "Compatibility",
    );
    let switcher = gtk::StackSwitcher::builder()
        .stack(&stack)
        .halign(gtk::Align::Start)
        .build();
    switcher.add_css_class("page-tabs");
    header.append(&switcher);

    page.append(&header);
    page.append(&stack);
    page
}

fn scout_page() -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Scout Mode"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    heading.append(&title);
    let status = gtk::Label::new(Some("UNAVAILABLE IN LINUX"));
    status.add_css_class("unavailable-pill");
    heading.append(&status);
    page.append(&heading);

    let explanation = gtk::Label::new(Some(
        "Scout Mode is visible here so Windows users can account for the feature during \
         migration. The Creative implementation and its hotkey integration are proprietary, \
         and the Linux CA0132 driver does not expose an equivalent control.",
    ));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.add_css_class("dim-label");
    page.append(&explanation);

    let alternatives = gtk::Box::new(gtk::Orientation::Vertical, 8);
    for text in [
        "Use the Equalizer page for a transparent, user-controlled frequency emphasis.",
        "Use a PipeWire filter-chain preset only when you can verify its gain and latency.",
        "Imported Windows profiles retain Scout Mode as an explicit unsupported item.",
    ] {
        let row = gtk::Label::new(Some(&format!("• {text}")));
        row.set_xalign(0.0);
        row.set_wrap(true);
        alternatives.append(&row);
    }
    page.append(&profile_card(
        "STATUS",
        "Linux status: unavailable",
        "AE-5 Control does not present a decorative switch as working hardware support.",
        &alternatives,
    ));

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn sound_effects_page(
    window: &gtk::ApplicationWindow,
    card_index: i32,
    status: &gtk::Label,
    controls: &[ControlSnapshot],
) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Sound effects"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    header.append(&title);
    if let Some(engine) = controls
        .iter()
        .find(|control| control.name == "Enable OutFX")
        && let Some(enabled) = engine.playback_switch
    {
        let engine_label = gtk::Label::new(Some("Acoustic engine"));
        engine_label.add_css_class("dim-label");
        header.append(&engine_label);
        header.append(&switch_editor(
            card_index,
            status,
            &engine.name,
            enabled,
            false,
            None,
        ));
    }
    page.append(&header);

    let profile_heading = gtk::Label::new(Some("Your profiles"));
    profile_heading.set_xalign(0.0);
    profile_heading.add_css_class("mixer-section");
    page.append(&profile_heading);

    let profile_strip = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    profile_strip.append(&sound_profile_card(
        "LIVE",
        "Current hardware",
        &format!("{} controls read from the AE-5", controls.len()),
        None,
        false,
    ));
    match profile_library() {
        Ok(library) => {
            for entry in library.profiles {
                let active = profile_matches_controls(&entry.profile, controls);
                let detail = if active {
                    active_profile_detail(&entry.profile)
                } else {
                    format!("{} validated controls", entry.profile.controls.len())
                };
                let card = sound_profile_card(
                    if active { "ACTIVE" } else { "PROFILE" },
                    &entry.profile.name,
                    &detail,
                    (!active).then_some("Preview & apply"),
                    active,
                );
                if let Some(button) = find_widget(card.clone().upcast(), |widget| {
                    widget.has_css_class("profile-card-action")
                })
                .and_then(|widget| widget.downcast::<gtk::Button>().ok())
                {
                    let path = entry.path;
                    let window = window.clone();
                    let status = status.clone();
                    button.connect_clicked(move |_| {
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
                profile_strip.append(&card);
            }
        }
        Err(error) => {
            profile_strip.append(&sound_profile_card(
                "PROFILES",
                "Library unavailable",
                &error.to_string(),
                None,
                false,
            ));
        }
    }
    let profiles = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(148)
        .child(&profile_strip)
        .build();
    profiles.set_widget_name(PERSONAL_PROFILE_CAROUSEL_NAME);
    profiles.add_css_class("profile-carousel");
    page.append(&profiles);

    let defaults_heading = gtk::Label::new(Some(&format!(
        "Sound Blaster Command defaults · {COMMAND_DEFAULT_PROFILE_COUNT}"
    )));
    defaults_heading.set_xalign(0.0);
    defaults_heading.add_css_class("mixer-section");
    page.append(&defaults_heading);

    let defaults_strip = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    match builtin_profiles() {
        Ok(profiles) => {
            for preset in profiles {
                let layout = controls
                    .iter()
                    .find(|control| control.name == "Surround Channel Config")
                    .and_then(|control| control.selected.as_deref());
                let live_profile = controls
                    .iter()
                    .find(|control| control.name == "Output Select")
                    .and_then(|control| control.selected.as_deref())
                    .and_then(|output| preset.profile_for(output, layout).ok());
                let active = live_profile
                    .as_ref()
                    .is_some_and(|profile| profile_matches_controls(profile, controls));
                let detail = live_profile.as_ref().filter(|_| active).map_or_else(
                    || "Speaker + headphone variants".to_owned(),
                    active_profile_detail,
                );
                let card = sound_profile_card(
                    if active { "ACTIVE" } else { "BUILT-IN" },
                    &preset.name,
                    &detail,
                    (!active).then_some("Preview & apply"),
                    active,
                );
                if let Some(button) = find_widget(card.clone().upcast(), |widget| {
                    widget.has_css_class("profile-card-action")
                })
                .and_then(|widget| widget.downcast::<gtk::Button>().ok())
                {
                    let preset = preset.clone();
                    let window = window.clone();
                    let status = status.clone();
                    button.connect_clicked(move |_| {
                        let preset = preset.clone();
                        let window = window.clone();
                        let status = status.clone();
                        gtk::glib::spawn_future_local(async move {
                            match apply_builtin_profile(&window, card_index, &preset).await {
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
                defaults_strip.append(&card);
            }
        }
        Err(error) => {
            defaults_strip.append(&sound_profile_card(
                "BUILT-IN",
                "Defaults unavailable",
                error,
                None,
                false,
            ));
        }
    }
    let defaults = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Never)
        .min_content_height(148)
        .child(&defaults_strip)
        .build();
    defaults.set_widget_name(BUILTIN_PROFILE_CAROUSEL_NAME);
    defaults.add_css_class("profile-carousel");
    page.append(&defaults);

    let effects_heading = gtk::Label::new(Some("Acoustic engine"));
    effects_heading.set_xalign(0.0);
    effects_heading.add_css_class("mixer-section");
    page.append(&effects_heading);

    let effects = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    effects.set_homogeneous(true);
    for name in [
        "FX: Surround",
        "FX: Crystalizer",
        "FX: X-Bass",
        "FX: Smart Volume",
        "FX: Dialog Plus",
    ] {
        if let Some(control) = controls.iter().find(|control| control.name == name) {
            effects.append(&effect_control_card(card_index, status, control, controls));
        }
    }
    page.append(&effects);

    let secondary_controls = ["FX: Smart Volume Setting", "FX: X-Bass Crossover"]
        .into_iter()
        .filter_map(|name| controls.iter().find(|control| control.name == name))
        .collect::<Vec<_>>();
    if !secondary_controls.is_empty() {
        let advanced = gtk::Expander::new(Some("Advanced effect tuning"));
        advanced.set_child(Some(&control_list(
            card_index,
            status,
            controls,
            secondary_controls.into_iter(),
        )));
        page.append(&advanced);
    }

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn sound_profile_card(
    kicker: &str,
    title: &str,
    detail: &str,
    action: Option<&str>,
    active: bool,
) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 6);
    card.set_size_request(178, 126);
    card.add_css_class("sound-profile-card");
    if active {
        card.add_css_class("sound-profile-card-active");
    }

    let kicker = gtk::Label::new(Some(kicker));
    kicker.set_xalign(0.0);
    kicker.add_css_class("profile-card-kicker");
    let accessible_title = title.to_owned();
    let title = gtk::Label::new(Some(title));
    title.set_xalign(0.0);
    title.set_ellipsize(gtk::pango::EllipsizeMode::End);
    title.add_css_class("profile-card-title");
    let detail = gtk::Label::new(Some(detail));
    detail.set_xalign(0.0);
    detail.set_wrap(true);
    detail.set_lines(2);
    detail.add_css_class("dim-label");

    card.append(&kicker);
    card.append(&title);
    card.append(&detail);
    if let Some(action) = action {
        let button = gtk::Button::with_label(action);
        button.set_halign(gtk::Align::Start);
        button.add_css_class("profile-card-action");
        let accessible_label = format!("{action} “{accessible_title}”");
        button.update_property(&[gtk::accessible::Property::Label(&accessible_label)]);
        card.append(&button);
    } else if active {
        let active = gtk::Label::new(Some("ACTIVE"));
        active.set_halign(gtk::Align::Start);
        active.add_css_class("profile-card-active-label");
        card.append(&active);
    }
    card
}

fn profile_matches_controls(profile: &Profile, controls: &[ControlSnapshot]) -> bool {
    let skip_equalizer_bands = profile
        .controls
        .get("FX: Equalizer Preset")
        .and_then(|control| control.choice.as_deref())
        .is_some_and(|preset| !preset.eq_ignore_ascii_case("Flat"));

    profile.controls.iter().all(|(name, expected)| {
        if capture_control_block_reason(name).is_some()
            || skip_equalizer_bands && name.starts_with("EQ Band")
        {
            return true;
        }
        let Some(actual) = controls.iter().find(|control| control.name == *name) else {
            return false;
        };
        expected.choice.as_ref().is_none_or(|value| {
            actual
                .selected
                .as_ref()
                .is_some_and(|actual| actual.eq_ignore_ascii_case(value))
        }) && expected
            .playback_switch
            .is_none_or(|value| actual.playback_switch == Some(value))
            && expected
                .capture_switch
                .is_none_or(|value| actual.capture_switch == Some(value))
            && expected.playback_level.is_none_or(|value| {
                actual.playback_level.as_ref().map(|level| level.value) == Some(value)
            })
            && expected.capture_level.is_none_or(|value| {
                actual.capture_level.as_ref().map(|level| level.value) == Some(value)
            })
            && expected.playback_channels.iter().all(|(channel, value)| {
                actual
                    .playback_channels
                    .iter()
                    .find(|actual| actual.name.eq_ignore_ascii_case(channel))
                    .map(|actual| actual.value)
                    == Some(*value)
            })
            && expected.capture_channels.iter().all(|(channel, value)| {
                actual
                    .capture_channels
                    .iter()
                    .find(|actual| actual.name.eq_ignore_ascii_case(channel))
                    .map(|actual| actual.value)
                    == Some(*value)
            })
    })
}

fn active_profile_detail(profile: &Profile) -> String {
    let effects = [
        "FX: Surround",
        "FX: Crystalizer",
        "FX: X-Bass",
        "FX: Smart Volume",
        "FX: Dialog Plus",
    ];
    let enabled = effects
        .iter()
        .filter(|name| {
            profile
                .controls
                .get(**name)
                .and_then(|control| control.playback_switch)
                == Some(true)
        })
        .count();
    let disabled = effects
        .iter()
        .filter(|name| {
            profile
                .controls
                .get(**name)
                .and_then(|control| control.playback_switch)
                == Some(false)
        })
        .count();
    let equalizer = profile
        .controls
        .get("FX: Equalizer")
        .and_then(|control| control.playback_switch)
        == Some(true);

    match (enabled, disabled, equalizer) {
        (0, disabled, true) if disabled > 0 => {
            format!("Equalizer only · {disabled} effects off by profile")
        }
        (0, _, true) => "Equalizer only".to_owned(),
        (enabled, _, true) => format!("{enabled} effects + equalizer enabled"),
        (enabled, _, false) if enabled > 0 => format!("{enabled} effects enabled"),
        _ => "All acoustic effects disabled".to_owned(),
    }
}

fn effect_control_card(
    card_index: i32,
    status: &gtk::Label,
    control: &ControlSnapshot,
    all_controls: &[ControlSnapshot],
) -> gtk::Box {
    let card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    card.set_size_request(152, 176);
    card.add_css_class("effect-card");

    let header = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title = gtk::Label::new(Some(match control.name.as_str() {
        "FX: X-Bass" => "Bass",
        "FX: Dialog Plus" => "Dialog+",
        name => name.strip_prefix("FX: ").unwrap_or(name),
    }));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("effect-card-title");
    header.append(&title);
    if let Some(enabled) = control.playback_switch {
        header.append(&switch_editor(
            card_index,
            status,
            &control.name,
            enabled,
            false,
            playback_switch_block_reason(&control.name, true, all_controls),
        ));
    }
    card.append(&header);

    if let Some(level) = &control.playback_level {
        let value = gtk::Label::new(Some(&level.value.to_string()));
        value.set_halign(gtk::Align::Center);
        value.add_css_class("effect-dial-value");
        card.append(&value);

        let editor = level_editor(
            card_index,
            status,
            &control.name,
            level,
            false,
            None,
            direct_mode_block_reason(&control.name, all_controls)
                .or_else(|| smart_volume_level_block_reason(&control.name, all_controls)),
        );
        if let Ok(scale) = editor.clone().downcast::<gtk::Scale>() {
            scale.set_width_request(126);
            scale.set_hexpand(true);
        }
        card.append(&editor);
    }

    card
}

fn playback_page(card_index: i32, status: &gtk::Label, controls: &[ControlSnapshot]) -> gtk::Box {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);

    let header = gtk::Box::new(gtk::Orientation::Vertical, 10);
    header.add_css_class("settings-header");
    let title = gtk::Label::new(Some("Playback"));
    title.set_xalign(0.0);
    title.add_css_class("page-title");
    header.append(&title);

    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .hexpand(true)
        .vexpand(true)
        .build();
    stack.add_titled(
        &analog_playback_page(card_index, status, controls),
        Some("analog"),
        "Analog",
    );
    stack.add_titled(
        &digital_playback_page(card_index, status, controls),
        Some("digital"),
        "Digital",
    );
    let switcher = gtk::StackSwitcher::builder()
        .stack(&stack)
        .halign(gtk::Align::Start)
        .build();
    switcher.add_css_class("page-tabs");
    header.append(&switcher);

    page.append(&header);
    page.append(&stack);
    page
}

fn analog_playback_page(
    card_index: i32,
    status: &gtk::Label,
    controls: &[ControlSnapshot],
) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 16);
    page.add_css_class("profile-page");

    if let Some(warning) = front_vmaster_clamp_warning(controls) {
        let notice = gtk::Label::new(Some(&format!("Gain staging\n{warning}")));
        notice.set_xalign(0.0);
        notice.set_wrap(true);
        notice.add_css_class("gain-stage-notice");
        page.append(&notice);
    }

    if let Some(output) = controls
        .iter()
        .find(|control| control.name == "Output Select")
    {
        let route = gtk::Label::new(Some(&format!(
            "Active route: {} · change Speakers / Headphones from the footer",
            output.selected.as_deref().unwrap_or("unknown")
        )));
        route.set_xalign(0.0);
        route.set_wrap(true);
        route.add_css_class("playback-route-note");
        page.append(&route);
    }

    let settings_title = gtk::Label::new(Some("Playback setup"));
    settings_title.set_xalign(0.0);
    settings_title.add_css_class("mixer-section");
    page.append(&settings_title);

    let grid = gtk::Grid::builder()
        .column_spacing(12)
        .row_spacing(12)
        .column_homogeneous(true)
        .build();
    for (index, name) in [
        "AE-5: Headphone Gain",
        "AE-5: Sound Filter",
        "Surround Channel Config",
        "HP/Speaker Auto Detect",
    ]
    .into_iter()
    .enumerate()
    {
        if let Some(control) = controls.iter().find(|control| control.name == name) {
            grid.attach(
                &playback_setting_tile(card_index, status, control, controls),
                (index % 2) as i32,
                (index / 2) as i32,
                1,
                1,
            );
        }
    }

    let direct_tile = if let Some(control) = controls
        .iter()
        .find(|control| control.name == DIRECT_MODE_CONTROL)
    {
        playback_setting_tile(card_index, status, control, controls)
    } else {
        playback_unavailable_tile(
            "Direct Mode",
            "Available after booting the installed 7.1.4-ae5-current test kernel.",
        )
    };
    grid.attach(&direct_tile, 0, 2, 1, 1);
    grid.attach(
        &playback_unavailable_tile(
            "Audio quality",
            "Native PipeWire rates: 44.1, 48, and 96 kHz. Linux exposes the transport format separately from Creative's Windows label.",
        ),
        1,
        2,
        1,
        1,
    );
    grid.attach(
        &playback_unavailable_tile(
            "Headphone model tuning",
            "Command's model files contain display metadata only; the correction response \
             remains inside Creative's Windows driver/APO. Imported custom EQ profiles are \
             available on Linux without pretending those proprietary curves were copied.",
        ),
        0,
        3,
        2,
        1,
    );
    page.append(&grid);

    let advanced_controls = [
        "Full-Range Front Speakers",
        "Full-Range Rear Speakers",
        "Bass Redirection",
        "Bass Redirection Crossover",
    ]
    .into_iter()
    .filter_map(|name| controls.iter().find(|control| control.name == name))
    .collect::<Vec<_>>();
    if !advanced_controls.is_empty() {
        let advanced = gtk::Expander::new(Some("Advanced speaker controls"));
        advanced.set_child(Some(&control_list(
            card_index,
            status,
            controls,
            advanced_controls.into_iter(),
        )));
        page.append(&advanced);
    }

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn footer_output_selector(
    card_index: i32,
    status: &gtk::Label,
    control: &ControlSnapshot,
) -> gtk::Box {
    let choices = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    choices.add_css_class("footer-output-selector");
    let label = gtk::Label::new(Some("ROUTE"));
    label.add_css_class("footer-route-label");
    choices.append(&label);

    let speakers = gtk::ToggleButton::with_label("Speakers");
    let headphones = gtk::ToggleButton::with_label("Headphones");
    headphones.set_group(Some(&speakers));
    for button in [&speakers, &headphones] {
        button.add_css_class("footer-output-choice");
    }

    let selected_index = if control.selected.as_deref() == Some("Headphone") {
        1
    } else {
        0
    };
    if selected_index == 0 {
        speakers.set_active(true);
    } else {
        headphones.set_active(true);
    }
    let buttons = [speakers.clone(), headphones.clone()];
    let verified = Rc::new(Cell::new(selected_index));
    let updating = Rc::new(Cell::new(false));
    for (index, (button, requested)) in [
        (speakers.clone(), "Speakers"),
        (headphones.clone(), "Headphone"),
    ]
    .into_iter()
    .enumerate()
    {
        let buttons = buttons.clone();
        let verified = verified.clone();
        let updating = updating.clone();
        let status = status.clone();
        let control_name = control.name.clone();
        button.connect_toggled(move |button| {
            if updating.get() || !button.is_active() {
                return;
            }
            match with_mixer(card_index, |mixer| {
                mixer.set_choice_checked(&control_name, requested, false)
            }) {
                Ok(actual) => {
                    verified.set(index);
                    set_status(
                        &status,
                        true,
                        &format!("Applied and verified: {}", control_summary(&actual)),
                    );
                }
                Err(error) => {
                    updating.set(true);
                    buttons[verified.get()].set_active(true);
                    updating.set(false);
                    set_status(&status, false, &format!("Output change failed: {error}"));
                }
            }
        });
    }
    choices.append(&speakers);
    choices.append(&headphones);
    choices
}

fn playback_setting_tile(
    card_index: i32,
    status: &gtk::Label,
    control: &ControlSnapshot,
    all_controls: &[ControlSnapshot],
) -> gtk::Box {
    let tile = gtk::Box::new(gtk::Orientation::Vertical, 8);
    tile.add_css_class("playback-setting-tile");
    let title = gtk::Label::new(Some(&control_display_name(&control.name)));
    title.set_xalign(0.0);
    title.add_css_class("section-title");
    tile.append(&title);

    let block = direct_mode_block_reason(&control.name, all_controls);
    let permission = if control.name == "AE-5: Headphone Gain" {
        let permission = gtk::CheckButton::with_label("Allow 150–600 Ω");
        permission.set_tooltip_text(Some(
            "Enable only when high-impedance headphones are connected.",
        ));
        tile.append(&permission);
        Some(permission)
    } else {
        None
    };
    if control.selected.is_some() {
        tile.append(&choice_editor(
            card_index, status, control, permission, block,
        ));
    }
    if let Some(enabled) = control.playback_switch {
        let editor = switch_editor(card_index, status, &control.name, enabled, false, block);
        editor.set_halign(gtk::Align::Start);
        tile.append(&editor);
    }
    tile
}

fn playback_unavailable_tile(title: &str, detail: &str) -> gtk::Box {
    let tile = gtk::Box::new(gtk::Orientation::Vertical, 8);
    tile.add_css_class("playback-setting-tile");
    let title = gtk::Label::new(Some(title));
    title.set_xalign(0.0);
    title.add_css_class("section-title");
    let detail = gtk::Label::new(Some(detail));
    detail.set_xalign(0.0);
    detail.set_wrap(true);
    detail.add_css_class("dim-label");
    tile.append(&title);
    tile.append(&detail);
    tile
}

fn digital_playback_page(
    card_index: i32,
    status: &gtk::Label,
    controls: &[ControlSnapshot],
) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 16);
    page.add_css_class("profile-page");
    let intro = gtk::Label::new(Some(
        "Digital output controls are separate from the analog Speakers / Headphones path.",
    ));
    intro.set_xalign(0.0);
    intro.set_wrap(true);
    intro.add_css_class("dim-label");
    page.append(&intro);
    page.append(&control_list(
        card_index,
        status,
        controls,
        ["IEC958", "IEC958 Default PCM"]
            .into_iter()
            .filter_map(|name| controls.iter().find(|control| control.name == name)),
    ));
    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn equalizer_page(
    card_index: i32,
    status: &gtk::Label,
    controls: &[ControlSnapshot],
) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Equalizer"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    heading.append(&title);
    let enabled = controls
        .iter()
        .find(|control| control.name == "FX: Equalizer")
        .and_then(|control| control.playback_switch)
        .is_some_and(|enabled| enabled);
    let preset = controls
        .iter()
        .find(|control| control.name == "FX: Equalizer Preset")
        .and_then(|control| control.selected.as_deref())
        .unwrap_or("custom");
    let summary = gtk::Label::new(Some(&format!(
        "EQ {} · {}",
        if enabled { "ON" } else { "OFF" },
        preset.to_uppercase()
    )));
    summary.add_css_class("status-pill");
    heading.append(&summary);
    page.append(&heading);

    let intro = gtk::Label::new(Some(
        "Ten hardware bands follow the same center frequencies used by Sound Blaster Command. \
         Bass maps to 62 Hz and Treble maps to 8 kHz during Windows profile migration.",
    ));
    intro.set_xalign(0.0);
    intro.set_wrap(true);
    intro.add_css_class("dim-label");
    page.append(&intro);

    page.append(&control_list(
        card_index,
        status,
        controls,
        ["FX: Equalizer", "FX: Equalizer Preset"]
            .into_iter()
            .filter_map(|name| controls.iter().find(|control| control.name == name)),
    ));

    let bands = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    bands.set_homogeneous(true);
    bands.add_css_class("equalizer-bands");
    for (index, frequency) in [
        "31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k",
    ]
    .into_iter()
    .enumerate()
    {
        let name = format!("EQ Band{index}");
        let Some(control) = controls.iter().find(|control| control.name == name) else {
            continue;
        };
        let Some(level) = &control.playback_level else {
            continue;
        };
        let band = gtk::Box::new(gtk::Orientation::Vertical, 6);
        band.set_halign(gtk::Align::Center);
        let editor = level_editor_oriented(
            card_index,
            status,
            &control.name,
            level,
            false,
            None,
            direct_mode_block_reason(&control.name, controls)
                .or_else(|| equalizer_band_block_reason(&control.name, controls)),
            gtk::Orientation::Vertical,
        );
        let label = gtk::Label::new(Some(frequency));
        label.add_css_class("equalizer-frequency");
        band.append(&editor);
        band.append(&label);
        bands.append(&band);
    }
    page.append(&bands);

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn recording_page(
    card_index: i32,
    status: &gtk::Label,
    controls: &[ControlSnapshot],
) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Recording"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    heading.append(&title);

    if let Some(source) = controls
        .iter()
        .find(|control| control.name == "Input Source")
    {
        let current = gtk::Label::new(Some(&format!(
            "INPUT · {}",
            source.selected.as_deref().unwrap_or("UNKNOWN")
        )));
        current.add_css_class("status-pill");
        heading.append(&current);
    }
    page.append(&heading);

    let intro = gtk::Label::new(Some(
        "Select the physical input first, then configure capture gain and the CA0132 \
         recording processor. Every available change is written and read back immediately.",
    ));
    intro.set_xalign(0.0);
    intro.set_wrap(true);
    intro.add_css_class("dim-label");
    page.append(&intro);

    if let Some(source) = controls
        .iter()
        .find(|control| control.name == "Input Source")
    {
        let source_panel = gtk::Box::new(gtk::Orientation::Horizontal, 18);
        source_panel.add_css_class("recording-source-panel");
        let labels = gtk::Box::new(gtk::Orientation::Vertical, 4);
        labels.set_hexpand(true);
        let title = gtk::Label::new(Some("Recording source"));
        title.set_xalign(0.0);
        title.add_css_class("section-title");
        let detail = gtk::Label::new(Some(
            "This selects the card input; desktop application routing remains on the Mixer page.",
        ));
        detail.set_xalign(0.0);
        detail.set_wrap(true);
        detail.add_css_class("dim-label");
        labels.append(&title);
        labels.append(&detail);
        source_panel.append(&labels);
        source_panel.append(&choice_editor(card_index, status, source, None, None));
        page.append(&source_panel);
    }

    let capture = gtk::Label::new(Some("Capture path"));
    capture.set_xalign(0.0);
    capture.add_css_class("mixer-section");
    page.append(&capture);
    page.append(&control_list(
        card_index,
        status,
        controls,
        ["Capture", "Mic Boost"]
            .into_iter()
            .filter_map(|name| controls.iter().find(|control| control.name == name)),
    ));

    let processing_controls = [
        "Enable InFX",
        "FX: Noise Reduction",
        "FX: Mic SVM",
        "SVM Level",
        "FX: Voice Focus",
        "VoiceFX",
    ]
    .into_iter()
    .filter_map(|name| controls.iter().find(|control| control.name == name))
    .collect::<Vec<_>>();
    if !processing_controls.is_empty() {
        let processing = gtk::Label::new(Some("Recording processor"));
        processing.set_xalign(0.0);
        processing.add_css_class("mixer-section");
        page.append(&processing);
        page.append(&control_list(
            card_index,
            status,
            controls,
            processing_controls.into_iter(),
        ));
    }

    if let Some(loopback) = controls
        .iter()
        .find(|control| control.name == "What U Hear")
    {
        let advanced = gtk::Expander::new(Some("Desktop loopback"));
        advanced.set_child(Some(&control_list(
            card_index,
            status,
            controls,
            std::iter::once(loopback),
        )));
        page.append(&advanced);
    }

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn mixer_page(
    card_index: i32,
    status: &gtk::Label,
    controls: &[ControlSnapshot],
) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Mixer"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    heading.append(&title);
    if let Some(master) = controls.iter().find(|control| control.name == "Master") {
        let level = master
            .playback_level
            .as_ref()
            .map_or_else(|| "LEVEL UNKNOWN".to_owned(), hardware_level_label);
        let summary = gtk::Label::new(Some(&format!(
            "MASTER {} · {level}",
            if master.playback_switch == Some(false) {
                "MUTED"
            } else {
                "ACTIVE"
            }
        )));
        summary.add_css_class("status-pill");
        heading.append(&summary);
    }
    page.append(&heading);

    let intro = gtk::Label::new(Some(
        "Hardware playback and recording levels are shown together. Desktop default-device \
         choices remain separate from the card's ALSA mixer state.",
    ));
    intro.set_xalign(0.0);
    intro.set_wrap(true);
    intro.add_css_class("dim-label");
    page.append(&intro);

    if let Some(warning) = front_vmaster_clamp_warning(controls) {
        let notice = gtk::Label::new(Some(&format!("Gain staging\n{warning}")));
        notice.set_xalign(0.0);
        notice.set_wrap(true);
        notice.add_css_class("gain-stage-notice");
        page.append(&notice);
    }

    let playback = gtk::Label::new(Some("Playback"));
    playback.set_xalign(0.0);
    playback.add_css_class("mixer-section");
    page.append(&playback);
    page.append(&control_list(
        card_index,
        status,
        controls,
        ["Master", "PCM", "Front", "Surround", "Center", "LFE"]
            .into_iter()
            .filter_map(|name| controls.iter().find(|control| control.name == name)),
    ));

    let recording = gtk::Label::new(Some("Recording"));
    recording.set_xalign(0.0);
    recording.add_css_class("mixer-section");
    page.append(&recording);
    page.append(&control_list(
        card_index,
        status,
        controls,
        ["Capture", "What U Hear"]
            .into_iter()
            .filter_map(|name| controls.iter().find(|control| control.name == name)),
    ));

    let routing = gtk::Label::new(Some("Desktop routing"));
    routing.set_xalign(0.0);
    routing.add_css_class("mixer-section");
    page.append(&routing);
    page.append(&routing_card(
        card_index,
        status,
        "01",
        "Desktop playback output",
        ae5_output(card_index),
        set_ae5_default_output,
    ));
    page.append(&routing_card(
        card_index,
        status,
        "02",
        "Desktop recording input",
        ae5_input(card_index),
        set_ae5_default_input,
    ));
    page.append(&native_rates_card(status, native_rates_config()));

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn compatibility_page() -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Sound Blaster Command compatibility"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    heading.append(&title);
    let count = gtk::Label::new(Some(&format!(
        "{} FEATURES TRACKED",
        feature_parity().count()
    )));
    count.add_css_class("status-pill");
    heading.append(&count);
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

fn lighting_page(window: &gtk::ApplicationWindow, status: &gtk::Label) -> gtk::ScrolledWindow {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 18);
    page.add_css_class("profile-page");

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Onboard lighting"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    heading.append(&title);
    let available = Ae5Lighting::discover().is_ok();
    let state = gtk::Label::new(Some(if available {
        "5 LEDS ONLINE"
    } else {
        "KERNEL SUPPORT REQUIRED"
    }));
    state.add_css_class(if available {
        "status-pill"
    } else {
        "unavailable-pill"
    });
    heading.append(&state);
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
    let volume = node.volume_percent.map_or_else(
        || "PipeWire node volume unavailable".to_owned(),
        |volume| {
            format!(
                "PipeWire node volume: {volume}%\nWith the installed AE-5 soft-mixer profile, \
                 this is software attenuation and does not rewrite Master, Front, or PCM."
            )
        },
    );
    format!(
        "{}\n{} — {}\n{volume}",
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
            let issues = [
                state.output_issue(output, layout),
                headphone_playback_issue(controls).map(str::to_owned),
                state.input_issue(input),
            ]
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

    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = gtk::Label::new(Some("Profiles & migration"));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("page-title");
    heading.append(&title);
    if let Ok(library) = profile_library() {
        let summary = gtk::Label::new(Some(&format!(
            "{} PERSONAL · {} BUILT-IN",
            library.profiles.len(),
            COMMAND_DEFAULT_PROFILE_COUNT
        )));
        summary.add_css_class("status-pill");
        heading.append(&summary);
    }
    page.append(&heading);

    let intro = gtk::Label::new(Some(
        "Capture the live card, preview transactional changes, or convert your \
         Sound Blaster Command setup without altering the source files.",
    ));
    intro.set_xalign(0.0);
    intro.set_wrap(true);
    intro.add_css_class("dim-label");
    page.append(&intro);

    let defaults_actions = builtin_profile_actions(window, card_index, status);
    page.append(&profile_card(
        "01",
        "Sound Blaster Command defaults",
        "All 33 factory Sound Effects profiles from Command 3.5.10.0 are embedded as \
         validated Linux controls. The live Speakers / Headphones route chooses the matching \
         variant; built-ins never change the selected output.",
        &defaults_actions,
    ));

    let saved_actions = saved_profile_actions(window, card_index, status);
    page.append(&profile_card(
        "02",
        "Personal profiles",
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
        "03",
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
        "04",
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
        "05",
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

fn builtin_profile_actions(
    window: &gtk::ApplicationWindow,
    card_index: i32,
    status: &gtk::Label,
) -> gtk::Box {
    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let Ok(profiles) = builtin_profiles() else {
        let unavailable = gtk::Label::new(Some("The embedded profile catalog is unavailable."));
        unavailable.add_css_class("warning-label");
        actions.append(&unavailable);
        return actions;
    };

    let names = profiles
        .iter()
        .map(|profile| profile.name.as_str())
        .collect::<Vec<_>>();
    let picker = gtk::DropDown::from_strings(&names);
    picker.set_widget_name(BUILTIN_PROFILE_PICKER_NAME);
    picker.set_hexpand(true);
    picker.update_property(&[gtk::accessible::Property::Label(
        "Built-in Sound Blaster Command profile",
    )]);
    let apply = gtk::Button::with_label("Preview & apply");
    apply.add_css_class("suggested-action");

    {
        let picker = picker.clone();
        let window = window.clone();
        let status = status.clone();
        apply.connect_clicked(move |_| {
            let selected = picker.selected() as usize;
            let Some(preset) = builtin_profiles()
                .ok()
                .and_then(|profiles| profiles.get(selected))
                .cloned()
            else {
                set_status(
                    &status,
                    false,
                    "The selected built-in profile is unavailable.",
                );
                return;
            };
            let window = window.clone();
            let status = status.clone();
            gtk::glib::spawn_future_local(async move {
                match apply_builtin_profile(&window, card_index, &preset).await {
                    Ok(Some(message)) => {
                        let _ = refresh_window(&window, Some(&message));
                    }
                    Ok(None) => {}
                    Err(error) => set_status(&status, false, &format!("Apply failed: {error}")),
                }
            });
        });
    }

    actions.append(&picker);
    actions.append(&apply);
    actions
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
                apply.add_css_class("suggested-action");
                let export = gtk::Button::with_label("Export copy");
                let rename = gtk::Button::with_label("Rename");
                let trash = gtk::Button::with_label("Move to Trash");
                for (button, action) in [
                    (&apply, "Preview and apply"),
                    (&export, "Export a copy of"),
                    (&rename, "Rename"),
                    (&trash, "Move to Trash"),
                ] {
                    let accessible_label = format!("{action} “{}”", entry.profile.name);
                    button.update_property(&[gtk::accessible::Property::Label(&accessible_label)]);
                }
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
    apply_profile(window, card_index, profile).await
}

async fn apply_builtin_profile(
    window: &gtk::ApplicationWindow,
    card_index: i32,
    preset: &BuiltinProfile,
) -> Result<Option<String>, String> {
    let controls = snapshot_controls(card_index).map_err(|error| error.to_string())?;
    let selected = |name| {
        controls
            .iter()
            .find(|control| control.name == name)
            .and_then(|control| control.selected.as_deref())
    };
    let output = selected("Output Select")
        .ok_or_else(|| "the live Speakers / Headphones route is unavailable".to_owned())?;
    let layout = selected("Surround Channel Config");
    let profile = preset
        .profile_for(output, layout)
        .map_err(|error| error.to_string())?;
    apply_profile(window, card_index, profile).await
}

async fn apply_profile(
    window: &gtk::ApplicationWindow,
    card_index: i32,
    profile: Profile,
) -> Result<Option<String>, String> {
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
    category: Category,
    controls: impl Iterator<Item = &'a ControlSnapshot>,
) -> gtk::ScrolledWindow {
    let mut controls = controls.collect::<Vec<_>>();
    controls.sort_by(|left, right| {
        category
            .control_order(&left.name)
            .cmp(&category.control_order(&right.name))
            .then_with(|| left.name.cmp(&right.name))
    });

    let page = gtk::Box::new(gtk::Orientation::Vertical, 14);
    page.add_css_class("control-page");

    let heading = gtk::Label::new(Some(category.title()));
    heading.set_xalign(0.0);
    heading.add_css_class("page-title");
    page.append(&heading);
    let intro = gtk::Label::new(Some(category.description()));
    intro.set_xalign(0.0);
    intro.set_wrap(true);
    intro.add_css_class("dim-label");
    page.append(&intro);

    if matches!(category, Category::Playback)
        && let Some(warning) = front_vmaster_clamp_warning(all_controls)
    {
        let notice = gtk::Label::new(Some(&format!("Gain staging\n{warning}")));
        notice.set_xalign(0.0);
        notice.set_wrap(true);
        notice.set_selectable(true);
        notice.add_css_class("gain-stage-notice");
        page.append(&notice);
    }
    page.append(&control_list(
        card_index,
        status,
        all_controls,
        controls.into_iter(),
    ));

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
}

fn control_list<'a>(
    card_index: i32,
    status: &gtk::Label,
    all_controls: &[ControlSnapshot],
    controls: impl Iterator<Item = &'a ControlSnapshot>,
) -> gtk::ListBox {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("control-list");
    for control in controls {
        let playback_switch_block = (control.playback_switch == Some(false))
            .then(|| playback_switch_block_reason(&control.name, true, all_controls))
            .flatten();
        let edit_block = direct_mode_block_reason(&control.name, all_controls)
            .or_else(|| equalizer_band_block_reason(&control.name, all_controls))
            .or_else(|| smart_volume_level_block_reason(&control.name, all_controls));
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
    list
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
    level_editor_oriented(
        card_index,
        status,
        name,
        level,
        capture,
        channel,
        block_reason,
        gtk::Orientation::Horizontal,
    )
}

#[allow(clippy::too_many_arguments)]
fn level_editor_oriented(
    card_index: i32,
    status: &gtk::Label,
    name: &str,
    level: &Level,
    capture: bool,
    channel: Option<&str>,
    block_reason: Option<&str>,
    orientation: gtk::Orientation,
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

    let scale = gtk::Scale::with_range(orientation, level.min as f64, level.max as f64, 1.0);
    scale.set_value(level.value as f64);
    scale.set_digits(0);
    scale.set_draw_value(true);
    if orientation == gtk::Orientation::Horizontal {
        scale.set_width_request(190);
    } else {
        scale.set_height_request(178);
        scale.set_value_pos(gtk::PositionType::Top);
    }
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
                                let _ = refresh_window(&window, None);
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

fn capture_refresh_view_state(window: &gtk::ApplicationWindow) -> RefreshViewState {
    RefreshViewState {
        visible_page: main_stack(window)
            .and_then(|stack| stack.visible_child_name())
            .map(|name| name.to_string()),
        builtin_profile: named_widget(window, BUILTIN_PROFILE_PICKER_NAME)
            .and_then(|widget| widget.downcast::<gtk::DropDown>().ok())
            .map(|picker| picker.selected()),
        personal_profile_scroll: horizontal_scroll_value(window, PERSONAL_PROFILE_CAROUSEL_NAME),
        builtin_profile_scroll: horizontal_scroll_value(window, BUILTIN_PROFILE_CAROUSEL_NAME),
        operation_status: current_operation_status(window),
    }
}

fn restore_refresh_view_state(window: &gtk::ApplicationWindow, state: &RefreshViewState) {
    if let Some(selected) = state.builtin_profile
        && let Some(picker) = named_widget(window, BUILTIN_PROFILE_PICKER_NAME)
            .and_then(|widget| widget.downcast::<gtk::DropDown>().ok())
        && picker
            .model()
            .is_some_and(|model| selected < model.n_items())
    {
        picker.set_selected(selected);
    }
    for (name, value) in [
        (
            PERSONAL_PROFILE_CAROUSEL_NAME,
            state.personal_profile_scroll,
        ),
        (BUILTIN_PROFILE_CAROUSEL_NAME, state.builtin_profile_scroll),
    ] {
        let Some(value) = value else {
            continue;
        };
        let Some(scroll) = named_widget(window, name)
            .and_then(|widget| widget.downcast::<gtk::ScrolledWindow>().ok())
        else {
            continue;
        };
        let adjustment = scroll.hadjustment();
        gtk::glib::idle_add_local_once(move || {
            let lower = adjustment.lower();
            let upper = (adjustment.upper() - adjustment.page_size()).max(lower);
            adjustment.set_value(value.clamp(lower, upper));
        });
    }
}

fn horizontal_scroll_value(window: &gtk::ApplicationWindow, name: &str) -> Option<f64> {
    named_widget(window, name)
        .and_then(|widget| widget.downcast::<gtk::ScrolledWindow>().ok())
        .map(|scroll| scroll.hadjustment().value())
}

fn current_operation_status(window: &gtk::ApplicationWindow) -> Option<OperationStatus> {
    let status = operation_status_label(window)?;
    let success = if status.has_css_class("operation-ok") {
        true
    } else if status.has_css_class("operation-error") {
        false
    } else {
        return None;
    };
    Some(OperationStatus {
        success,
        message: status.text().to_string(),
    })
}

fn named_widget(window: &gtk::ApplicationWindow, name: &str) -> Option<gtk::Widget> {
    find_widget(window.child()?, |widget| widget.widget_name() == name)
}

fn operation_status_label(window: &gtk::ApplicationWindow) -> Option<gtk::Label> {
    find_widget(
        window.child().unwrap_or_else(|| window.clone().upcast()),
        |widget| widget.has_css_class("operation-status"),
    )
    .and_then(|widget| widget.downcast::<gtk::Label>().ok())
}

fn set_main_status(window: &gtk::ApplicationWindow, success: bool, message: &str) {
    if let Some(status) = operation_status_label(window) {
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

fn error_view(window: &gtk::ApplicationWindow, message: &str) -> gtk::Box {
    let view = gtk::Box::new(gtk::Orientation::Vertical, 0);
    view.set_valign(gtk::Align::Center);
    view.set_halign(gtk::Align::Center);
    view.set_margin_start(32);
    view.set_margin_end(32);
    view.add_css_class("error-view");

    let card = gtk::Box::new(gtk::Orientation::Vertical, 12);
    card.set_size_request(520, -1);
    card.add_css_class("unavailable-card");

    let icon = gtk::Image::from_icon_name("audio-card-symbolic");
    icon.set_pixel_size(42);
    icon.set_halign(gtk::Align::Start);
    icon.add_css_class("offline-icon");

    let kicker = gtk::Label::new(Some("HARDWARE STATUS  //  OFFLINE"));
    kicker.set_xalign(0.0);
    kicker.add_css_class("error-kicker");

    let title = gtk::Label::new(Some("AE-5 unavailable"));
    title.set_xalign(0.0);
    title.add_css_class("hero-title");
    let detail = gtk::Label::new(Some(message));
    detail.set_xalign(0.0);
    detail.set_wrap(true);
    detail.set_max_width_chars(64);
    detail.add_css_class("dim-label");

    let hint = gtk::Label::new(Some(
        "Make sure the card is bound to snd_hda_intel and is not assigned to a virtual machine, then retry detection.",
    ));
    hint.set_xalign(0.0);
    hint.set_wrap(true);
    hint.set_max_width_chars(64);
    hint.add_css_class("error-hint");

    let retry = gtk::Button::with_label("Retry detection");
    retry.set_halign(gtk::Align::Start);
    retry.add_css_class("error-action");
    {
        let window = window.clone();
        retry.connect_clicked(move |_| {
            if let Some(card_index) = refresh_window(&window, Some("Hardware connection restored."))
                && let Err(error) = start_mixer_watch(&window, card_index)
            {
                set_main_status(
                    &window,
                    false,
                    &format!("Live synchronization failed: {error}"),
                );
            }
        });
    }

    card.append(&icon);
    card.append(&kicker);
    card.append(&title);
    card.append(&detail);
    card.append(&hint);
    card.append(&retry);
    view.append(&card);
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

    fn title(self) -> &'static str {
        match self {
            Self::Playback => "Playback",
            Self::Effects => "Sound effects",
            Self::Equalizer => "Equalizer",
            Self::Recording => "Recording",
        }
    }

    fn description(self) -> &'static str {
        match self {
            Self::Playback => {
                "Choose the physical output, speaker layout, headphone gain, filter, and \
                 low-level playback controls exposed by the Linux CA0132 driver."
            }
            Self::Effects => {
                "Shape the CA0132 DSP processing path. Changes are written to ALSA and read \
                 back from the card immediately."
            }
            Self::Equalizer => {
                "Control the ten hardware equalizer bands and the driver's preset selector."
            }
            Self::Recording => {
                "Choose the input and configure capture processing. Unsupported or unsafe \
                 driver controls stay visible with an explanation."
            }
        }
    }

    fn control_order(self, name: &str) -> u8 {
        let ordered: &[&str] = match self {
            Self::Playback => &[
                "Output Select",
                "HP/Speaker Auto Detect",
                "Surround Channel Config",
                "Full-Range Front Speakers",
                "Full-Range Rear Speakers",
                "AE-5: Headphone Gain",
                "AE-5: Sound Filter",
                DIRECT_MODE_CONTROL,
                "Bass Redirection",
                "Bass Redirection Crossover",
            ],
            Self::Effects => &[
                "Enable OutFX",
                "FX: Surround",
                "FX: Crystalizer",
                "FX: X-Bass",
                "FX: X-Bass Crossover",
                "FX: Smart Volume",
                "FX: Smart Volume Setting",
                "FX: Dialog Plus",
            ],
            Self::Equalizer => &["FX: Equalizer", "FX: Equalizer Preset"],
            Self::Recording => &[
                "Input Source",
                "Capture",
                "Mic Boost",
                "Enable InFX",
                "FX: Noise Reduction",
                "FX: Mic SVM",
                "SVM Level",
                "FX: Voice Focus",
                "VoiceFX",
                "What U Hear",
            ],
        };
        ordered
            .iter()
            .position(|control| *control == name)
            .map_or(u8::MAX, |index| index as u8)
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
        window {
            background: #111827;
            color: #edf2f7;
        }
        .application-shell {
            background: #1d1c2e;
        }
        .sidebar-panel {
            min-width: 208px;
            background: #162040;
            border-right: 1px solid alpha(#ffffff, 0.08);
        }
        .sidebar-brand {
            padding: 18px 16px 14px 16px;
            background: #0d1828;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        .sidebar-title {
            color: #21c6d4;
            font-family: monospace;
            font-size: 13px;
            font-weight: 800;
            letter-spacing: 1px;
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
            background: #1d1c2e;
        }
        .hero {
            min-height: 48px;
            padding: 10px 22px;
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
            font-size: 15px;
            font-weight: 700;
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
            padding: 9px 0;
        }
        .navigation-sidebar row {
            min-height: 39px;
            margin: 0;
            padding: 0 12px;
            background: #162040;
            border-radius: 0;
            border-left: 3px solid transparent;
        }
        .navigation-sidebar row:hover { background: alpha(#ffffff, 0.035); }
        .navigation-sidebar row:focus-visible {
            box-shadow: inset 0 0 0 2px #57dce5;
        }
        .navigation-sidebar row:selected {
            background: #49536e;
            color: #43d5df;
            border-left: 3px solid #22c7d4;
        }
        .settings-header {
            padding: 18px 24px 0 24px;
            background: #25213c;
            border-bottom: 1px solid alpha(#ffffff, 0.07);
        }
        .page-tabs button {
            min-height: 30px;
            padding: 5px 14px;
            color: #98a7b7;
            background: transparent;
            border: 0;
            border-radius: 0;
            border-bottom: 2px solid transparent;
        }
        .page-tabs button:checked {
            color: #35d3de;
            border-bottom: 2px solid #22c7d4;
        }
        .profile-page, .control-page { padding: 18px 24px 24px 24px; }
        .page-title {
            font-size: 20px;
            font-weight: 760;
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
    fn command_primary_controls_appear_first() {
        assert!(
            Category::Playback.control_order("Output Select")
                < Category::Playback.control_order("AE-5: Headphone Gain")
        );
        assert!(
            Category::Effects.control_order("FX: Surround")
                < Category::Effects.control_order("FX: Dialog Plus")
        );
        assert!(
            Category::Recording.control_order("Input Source")
                < Category::Recording.control_order("FX: Noise Reduction")
        );
        assert_eq!(
            Category::Playback.control_order("unrecognized future control"),
            u8::MAX
        );
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
    fn labels_the_fixed_hardware_stage_as_zero_db() {
        assert_eq!(
            hardware_level_label(&Level {
                value: 99,
                min: 0,
                max: 99,
            }),
            "0 dB"
        );
        assert_eq!(
            hardware_level_label(&Level {
                value: 19,
                min: 0,
                max: 99,
            }),
            "19/99 raw"
        );
    }

    #[test]
    fn summarizes_pipewire_default_state() {
        let node = PipeWireNode {
            id: 58,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5 Analog Stereo".to_owned(),
            is_default: true,
            volume_percent: Some(43),
        };

        assert_eq!(
            pipewire_node_summary(&node),
            "AE-5 Analog Stereo\nalsa_output.pci-ae5.analog-stereo — currently default\n\
             PipeWire node volume: 43%\nWith the installed AE-5 soft-mixer profile, this is \
             software attenuation and does not rewrite Master, Front, or PCM."
        );
    }

    #[test]
    fn reports_matched_and_split_desktop_routes() {
        let mut controls = vec![
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
            ControlSnapshot {
                name: "Master".to_owned(),
                selected: None,
                choices: Vec::new(),
                playback_switch: Some(true),
                capture_switch: None,
                playback_level: None,
                capture_level: None,
                playback_channels: Vec::new(),
                capture_channels: Vec::new(),
            },
            ControlSnapshot {
                name: "Front".to_owned(),
                selected: None,
                choices: Vec::new(),
                playback_switch: Some(true),
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
            ignore_db: Some(true),
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

        controls[4].playback_switch = Some(false);
        let (summary, healthy) = route_health_summary(
            &controls,
            Ok(PipeWireRouteState {
                profile_set: Some("sound-blaster-ae5.conf".to_owned()),
                soft_mixer: Some(true),
                ignore_db: Some(true),
                active_profile: Some("output:analog-stereo+input:analog-stereo".to_owned()),
                input_route: Some("sound-blaster-ae5-input-microphone".to_owned()),
                output_route: Some(
                    "sound-blaster-ae5-output-headphones;output-headphones".to_owned(),
                ),
            }),
        );
        assert!(!healthy);
        assert!(summary.contains("Front playback is muted"));

        controls[4].playback_switch = Some(true);
        controls[3].playback_switch = Some(false);
        let (summary, healthy) = route_health_summary(
            &controls,
            Ok(PipeWireRouteState {
                profile_set: Some("sound-blaster-ae5.conf".to_owned()),
                soft_mixer: Some(true),
                ignore_db: Some(true),
                active_profile: Some("output:analog-stereo+input:analog-stereo".to_owned()),
                input_route: Some("sound-blaster-ae5-input-microphone".to_owned()),
                output_route: Some(
                    "sound-blaster-ae5-output-headphones;output-headphones".to_owned(),
                ),
            }),
        );
        assert!(!healthy);
        assert!(summary.contains("hardware Master playback is muted"));
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
    fn matches_profiles_against_live_hardware_and_describes_eq_only_defaults() {
        let profile = Profile {
            format_version: 1,
            name: "DOTA 2".to_owned(),
            target: "1102:0012/1102:0051".to_owned(),
            controls: std::collections::BTreeMap::from([
                (
                    "Output Select".to_owned(),
                    ProfileControl {
                        choice: Some("Headphone".to_owned()),
                        ..ProfileControl::default()
                    },
                ),
                (
                    "FX: Crystalizer".to_owned(),
                    ProfileControl {
                        playback_switch: Some(false),
                        playback_level: Some(38),
                        ..ProfileControl::default()
                    },
                ),
            ]),
        };
        let mut controls = vec![
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
                name: "FX: Crystalizer".to_owned(),
                selected: None,
                choices: Vec::new(),
                playback_switch: Some(false),
                capture_switch: None,
                playback_level: Some(Level {
                    value: 38,
                    min: 0,
                    max: 100,
                }),
                capture_level: None,
                playback_channels: Vec::new(),
                capture_channels: Vec::new(),
            },
        ];

        assert!(profile_matches_controls(&profile, &controls));
        controls[1].playback_switch = Some(true);
        assert!(!profile_matches_controls(&profile, &controls));

        let mut eq_only = profile;
        for effect in [
            "FX: Surround",
            "FX: X-Bass",
            "FX: Smart Volume",
            "FX: Dialog Plus",
        ] {
            eq_only.controls.insert(
                effect.to_owned(),
                ProfileControl {
                    playback_switch: Some(false),
                    ..ProfileControl::default()
                },
            );
        }
        eq_only.controls.insert(
            "FX: Equalizer".to_owned(),
            ProfileControl {
                playback_switch: Some(true),
                ..ProfileControl::default()
            },
        );
        assert_eq!(
            active_profile_detail(&eq_only),
            "Equalizer only · 5 effects off by profile"
        );
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
    fn summarizes_clean_project_kernel_readiness() {
        let (summary, readiness) =
            kernel_readiness_summary("7.1.4-ae5-current", Some(0), true, true);

        assert_eq!(readiness, KernelReadiness::Ready);
        assert!(summary.contains("Running kernel: 7.1.4-ae5-current"));
        assert!(summary.contains("Kernel taint: 0 (clean)"));
        assert!(summary.contains("Direct Mode: available"));
        assert!(summary.contains("Onboard lighting: available (five LEDs)"));
        assert!(summary.contains("physical acceptance is still pending"));
    }

    #[test]
    fn distinguishes_the_clean_stock_kernel_path() {
        let (summary, readiness) =
            kernel_readiness_summary("7.1.4-200.nobara.fc44.x86_64", Some(0), false, false);

        assert_eq!(readiness, KernelReadiness::Stock);
        assert!(summary.contains("Direct Mode: unavailable"));
        assert!(summary.contains("Onboard lighting: unavailable"));
        assert!(summary.contains("Stock-compatible control path active"));
    }

    #[test]
    fn warns_when_the_kernel_is_tainted() {
        let (summary, readiness) =
            kernel_readiness_summary("7.1.4-ae5-current", Some(512), true, true);

        assert_eq!(readiness, KernelReadiness::Attention);
        assert!(summary.contains("Kernel taint: 512"));
        assert!(summary.contains("needs review before physical driver testing"));
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

    #[test]
    fn refresh_retains_verified_feedback_until_an_explicit_result_replaces_it() {
        let applied = OperationStatus {
            success: true,
            message: "Applied “Gaming”; 20 controls were verified against the hardware.".to_owned(),
        };
        assert_eq!(
            operation_status_for_refresh(None, Some(applied.clone())),
            Some(applied)
        );
        assert_eq!(
            operation_status_for_refresh(
                Some("Applied “My profile · Headphones”; 21 controls were verified."),
                None,
            ),
            Some(OperationStatus {
                success: true,
                message: "Applied “My profile · Headphones”; 21 controls were verified.".to_owned(),
            })
        );
    }
}
