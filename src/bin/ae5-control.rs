use ae5_control::{Ae5Device, Ae5Mixer, ControlSnapshot};
use gtk::gdk::Display;
use gtk::prelude::*;

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

    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .hexpand(true)
        .vexpand(true)
        .build();

    for category in Category::ALL {
        let page = control_page(
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

fn control_page<'a>(controls: impl Iterator<Item = &'a ControlSnapshot>) -> gtk::ScrolledWindow {
    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::None);
    list.add_css_class("control-list");
    for control in controls {
        list.append(&control_row(control));
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

fn control_row(control: &ControlSnapshot) -> gtk::ListBoxRow {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 24);
    row.add_css_class("control-row");

    let name = gtk::Label::new(Some(&control.name));
    name.set_xalign(0.0);
    name.set_hexpand(true);
    name.set_wrap(true);
    row.append(&name);

    let value = gtk::Label::new(Some(&control_value(control)));
    value.set_xalign(1.0);
    value.set_selectable(true);
    value.add_css_class("control-value");
    row.append(&value);

    gtk::ListBoxRow::builder()
        .activatable(false)
        .selectable(false)
        .child(&row)
        .build()
}

fn control_value(control: &ControlSnapshot) -> String {
    let mut values = Vec::new();
    if let Some(value) = &control.selected {
        values.push(value.clone());
    }
    if let Some(value) = control.playback_switch {
        values.push(if value { "Playback on" } else { "Playback off" }.to_owned());
    }
    if let Some(level) = &control.playback_level {
        values.push(format!("{}", level.value));
    }
    if let Some(value) = control.capture_switch {
        values.push(if value { "Capture on" } else { "Capture off" }.to_owned());
    }
    if let Some(level) = &control.capture_level {
        values.push(format!("{}", level.value));
    }
    values.join(" · ")
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
        .navigation-sidebar { background: #141b22; padding: 12px 8px; }
        .control-list { background: transparent; }
        .control-row {
            padding: 14px 16px;
            border-bottom: 1px solid alpha(#ffffff, 0.08);
        }
        .control-value { color: #9fd6ff; font-weight: 600; }
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
