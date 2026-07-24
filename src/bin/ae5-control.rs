use ae5_control::{
    Ae5Device, Ae5Mixer, ChannelLevel, ControlError, ControlSnapshot, Level, PipeWireNode, Profile,
    ProfileControl, SbCommandImport, SbCommandTarget, ae5_input, ae5_output,
    import_sbcommand_profile_with_report, set_ae5_default_input, set_ae5_default_output,
    snapshot_controls,
};
use gtk::prelude::*;
use gtk::{gdk::Display, gio};
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

const APP_ID: &str = "io.github.klimovich008.ae5control";
const MAIN_STACK_NAME: &str = "main-navigation";

fn main() -> gtk::glib::ExitCode {
    let application = gtk::Application::builder().application_id(APP_ID).build();
    application.connect_activate(build_window);
    application.run()
}

fn build_window(application: &gtk::Application) {
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

    stack.add_titled(
        &routing_page(device.card_index, &status),
        Some("routing"),
        "System audio",
    );
    stack.add_titled(
        &profile_page(window, device.card_index, &status),
        Some("profiles"),
        "Profiles",
    );
    for category in Category::ALL {
        let page = control_page(
            device.card_index,
            &status,
            controls
                .iter()
                .filter(|control| category.matches(&control.name)),
        );
        stack.add_titled(&page, Some(category.id()), category.title());
    }
    if let Some(page) = visible_page {
        stack.set_visible_child_name(page);
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

    gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&page)
        .build()
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
         Sound Blaster Command JSON without altering the source files.",
    ));
    intro.set_xalign(0.0);
    intro.set_wrap(true);
    intro.add_css_class("dim-label");
    page.append(&intro);

    let native_actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let save = gtk::Button::with_label("Save current state");
    save.add_css_class("suggested-action");
    let apply = gtk::Button::with_label("Preview & apply profile");
    native_actions.append(&save);
    native_actions.append(&apply);
    page.append(&profile_card(
        "01",
        "Native profiles",
        "Portable JSON uses semantic ALSA names. Applying validates every value, \
         verifies readback, and rolls back the targeted controls on failure.",
        &native_actions,
    ));

    let import = gtk::Button::with_label("Import Windows profile");
    let import_actions = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    import_actions.append(&import);
    page.append(&profile_card(
        "02",
        "Sound Blaster Command",
        "Choose the Creative profile and EQ JSON files, select headphones or \
         speakers, inspect the mapped Linux controls, then save a native copy.",
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
                    Ok(Some(message)) => set_status(&status, true, &message),
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
        import.connect_clicked(move |_| {
            let window = window.clone();
            let status = status.clone();
            gtk::glib::spawn_future_local(async move {
                match import_windows_profile(&window, card_index).await {
                    Ok(Some(message)) => set_status(&status, true, &message),
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
    let Some(path) = save_json_path(window, "Save native AE-5 profile", "ae5-profile.json").await?
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
    let Some(path) = open_json_path(window, "Open native AE-5 profile").await? else {
        return Ok(None);
    };
    let profile = Profile::load(&path).map_err(|error| error.to_string())?;
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
        save_json_path(window, "Save converted native profile", &initial_name).await?
    else {
        return Ok(None);
    };
    let name = profile_name_from_path(&output)?;
    let import = import_sbcommand_profile_with_report(&name, &profile_path, &eq_path, target)
        .map_err(|error| error.to_string())?;
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

async fn save_json_path(
    window: &gtk::ApplicationWindow,
    title: &str,
    initial_name: &str,
) -> Result<Option<PathBuf>, String> {
    let dialog = json_dialog(title);
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

fn is_cancelled(error: &gtk::glib::Error) -> bool {
    error.matches(gio::IOErrorEnum::Cancelled)
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
    controls: impl Iterator<Item = &'a ControlSnapshot>,
) -> gtk::ScrolledWindow {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("control-list");
    for control in controls {
        list.append(&control_row(card_index, status, control));
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

fn control_row(card_index: i32, status: &gtk::Label, control: &ControlSnapshot) -> gtk::ListBoxRow {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 24);
    row.add_css_class("control-row");

    let name = gtk::Label::new(Some(&control.name));
    name.set_xalign(0.0);
    name.set_hexpand(true);
    name.set_wrap(true);
    row.append(&name);

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
        ));
    }
    if let Some(enabled) = control.playback_switch {
        editors.append(&labelled(
            "Playback",
            &switch_editor(card_index, status, &control.name, enabled, false),
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
        ));
    }
    if let Some(enabled) = control.capture_switch {
        editors.append(&labelled(
            "Capture",
            &switch_editor(card_index, status, &control.name, enabled, true),
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
        ));
    }
    row.append(&editors);

    gtk::ListBoxRow::builder()
        .activatable(false)
        .selectable(false)
        .child(&row)
        .build()
}

fn choice_editor(
    card_index: i32,
    status: &gtk::Label,
    control: &ControlSnapshot,
    high_gain_permission: Option<gtk::CheckButton>,
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
) -> gtk::Switch {
    let control = gtk::Switch::builder()
        .active(enabled)
        .valign(gtk::Align::Center)
        .build();
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
) -> gtk::Widget {
    if channels.len() < 2 {
        return level_editor(card_index, status, name, level, capture, None);
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

    fn title(self) -> &'static str {
        match self {
            Self::Playback => "Playback",
            Self::Effects => "Sound effects",
            Self::Equalizer => "Equalizer",
            Self::Recording => "Recording",
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
        .operation-error, .warning-value { color: #ffb4a9; }
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
                exact: vec!["master → output effects".to_owned()],
                approximate: vec!["surround 67.5 → 68".to_owned()],
                unsupported: (0..20)
                    .map(|index| format!("unsupported {index}"))
                    .collect(),
            },
        };

        let preview = migration_preview(&import);
        assert!(preview.contains("Exact mappings (1)"));
        assert!(preview.contains("Approximate mappings (1)"));
        assert!(preview.contains("Unsupported settings (20)"));
        assert!(preview.contains("unsupported 19"));
    }
}
