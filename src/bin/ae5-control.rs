use ae5_control::{Ae5Device, Ae5Mixer, ControlError, ControlSnapshot, Level};
use gtk::gdk::Display;
use gtk::prelude::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Duration;

const APP_ID: &str = "io.github.klimovich008.Ae5Control";

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

    match load_hardware() {
        Ok((device, controls)) => window.set_child(Some(&content(&device, &controls))),
        Err(error) => window.set_child(Some(&error_view(&error))),
    }
    window.present();
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

fn content(device: &Ae5Device, controls: &[ControlSnapshot]) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&hero(device, controls));
    let status = gtk::Label::new(Some(
        "Ready — every change is verified against the hardware.",
    ));
    status.set_xalign(0.0);
    status.add_css_class("operation-status");
    root.append(&status);

    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .hexpand(true)
        .vexpand(true)
        .build();

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
        editors.append(&level_editor(
            card_index,
            status,
            &control.name,
            level,
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
        editors.append(&level_editor(
            card_index,
            status,
            &control.name,
            level,
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

fn level_editor(
    card_index: i32,
    status: &gtk::Label,
    name: &str,
    level: &Level,
    capture: bool,
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
    scale.update_property(&[gtk::accessible::Property::Label(&format!(
        "{} {} level",
        name,
        if capture { "capture" } else { "playback" }
    ))]);

    let verified = Rc::new(Cell::new(level.value));
    let updating = Rc::new(Cell::new(false));
    let pending = Rc::new(RefCell::new(None::<gtk::glib::SourceId>));
    let name = name.to_owned();
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
        let status = status.clone();
        let source = gtk::glib::timeout_add_local_once(Duration::from_millis(160), move || {
            pending_for_timeout.borrow_mut().take();
            let result = with_mixer(card_index, |mixer| {
                if capture {
                    mixer.set_capture_level(&name, requested)
                } else {
                    mixer.set_playback_level(&name, requested)
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
}
