//! Generic ALSA control editors.
//!
//! Every widget here writes through the checked mixer path and reflects the
//! value the hardware reported back, so a control that cannot be applied
//! shows its real state rather than the requested one.

use std::cell::Cell;

use crate::{
    Ae5Mixer, ChannelLevel, ControlSnapshot, Level, capture_control_block_reason,
    direct_mode_block_reason, equalizer_band_block_reason, front_vmaster_clamp_warning,
    playback_switch_block_reason, smart_volume_level_block_reason,
};
use gtk::prelude::*;

use crate::{ControlError, DIRECT_MODE_CONTROL};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

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

#[derive(Copy, Clone)]
pub enum Category {
    Playback,
    Effects,
    Equalizer,
    Recording,
}

impl Category {
    pub const ALL: [Self; 4] = [
        Self::Playback,
        Self::Effects,
        Self::Equalizer,
        Self::Recording,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Self::Playback => "playback",
            Self::Effects => "effects",
            Self::Equalizer => "equalizer",
            Self::Recording => "recording",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::Playback => "Playback",
            Self::Effects => "Sound effects",
            Self::Equalizer => "Equalizer",
            Self::Recording => "Recording",
        }
    }

    pub fn description(self) -> &'static str {
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

    pub fn control_order(self, name: &str) -> u8 {
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

    pub fn matches(self, name: &str) -> bool {
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

pub fn control_page<'a>(
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

pub fn control_list<'a>(
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

pub fn control_row(
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
    // Rows carry different editor mixes — a dropdown, a switch, a slider, or
    // several. Reserving one width for the cluster keeps the sliders on a
    // shared vertical line instead of stepping in and out with the label
    // lengths beside them.
    editors.add_css_class("control-row-editors");
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

pub fn control_display_name(name: &str) -> String {
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

pub fn control_row_description(
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

pub fn choice_editor(
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
    mark_interactive(&dropdown);
    dropdown
}

/// A playback switch rendered as the effect's own dial rather than as a
/// separate toggle beside it.
///
/// The dial already showed the effect's level and the switch sat apart from it,
/// so the one obvious target — the big circle with the number in it — did
/// nothing. Pressing the dial now switches the effect, which is what it looks
/// like it should do.
///
/// Shares the verified write path with [`switch_editor`]: the request goes to
/// the mixer, the readback decides the reported state, and a failed write snaps
/// the control back to the value the hardware last confirmed.
pub fn dial_switch(
    card_index: i32,
    status: &gtk::Label,
    name: &str,
    enabled: bool,
    reading: &str,
    block_reason: Option<&str>,
) -> gtk::ToggleButton {
    let dial = gtk::ToggleButton::builder()
        .active(enabled)
        .halign(gtk::Align::Center)
        .label(reading)
        .build();
    dial.add_css_class("effect-dial");
    if let Some(reason) = block_reason {
        dial.set_sensitive(false);
        dial.set_tooltip_text(Some(reason));
    } else {
        dial.set_tooltip_text(Some(&format!(
            "{name}: press to switch {}",
            if enabled { "off" } else { "on" }
        )));
    }
    dial.update_property(&[gtk::accessible::Property::Label(&format!(
        "{name} playback switch, level {reading}"
    ))]);
    mark_interactive(&dial);

    let verified = Rc::new(Cell::new(enabled));
    let updating = Rc::new(Cell::new(false));
    let name = name.to_owned();
    let status = status.clone();
    dial.connect_toggled(move |dial| {
        if updating.get() {
            return;
        }
        let requested = dial.is_active();
        match with_mixer(card_index, |mixer| {
            mixer.set_playback_switch(&name, requested)
        }) {
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
                dial.set_active(verified.get());
                updating.set(false);
                set_status(&status, false, &format!("Change failed: {error}"));
            }
        }
    });
    dial
}

/// Give a control the cursor that states whether it can be operated.
///
/// GTK keeps the default arrow over switches, sliders and dropdowns, so the
/// interface offered no hover feedback about what was interactive. An
/// insensitive control gets the blocked cursor, which pairs with its greyed
/// styling instead of silently swallowing the click.
pub fn mark_interactive(widget: &impl IsA<gtk::Widget>) {
    let widget = widget.as_ref();
    widget.set_cursor_from_name(Some(if widget.is_sensitive() {
        "pointer"
    } else {
        "not-allowed"
    }));
}

pub fn switch_editor(
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
    mark_interactive(&control);
    control
}

pub fn level_editors(
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

pub fn level_editor(
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
pub fn level_editor_oriented(
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
    mark_interactive(&scale);
    scale.upcast()
}

pub fn labelled(label: &str, widget: &impl IsA<gtk::Widget>) -> gtk::Box {
    let group = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let label = gtk::Label::new(Some(label));
    label.add_css_class("dim-label");
    group.append(&label);
    group.append(widget);
    group
}

pub fn with_mixer<T>(
    card_index: i32,
    operation: impl FnOnce(&Ae5Mixer) -> Result<T, ControlError>,
) -> Result<T, String> {
    let mixer = Ae5Mixer::open(card_index).map_err(|error| error.to_string())?;
    operation(&mixer).map_err(|error| error.to_string())
}

pub fn is_high_gain(name: &str, requested: &str) -> bool {
    name == "AE-5: Headphone Gain" && requested.to_ascii_lowercase().starts_with("high")
}

pub fn revert_dropdown(dropdown: &gtk::DropDown, updating: &Cell<bool>, selected: u32) {
    updating.set(true);
    dropdown.set_selected(selected);
    updating.set(false);
}

pub fn control_summary(control: &ControlSnapshot) -> String {
    control.to_string()
}

pub fn set_status(status: &gtk::Label, success: bool, message: &str) {
    status.remove_css_class("operation-ok");
    status.remove_css_class("operation-error");
    status.add_css_class(if success {
        "operation-ok"
    } else {
        "operation-error"
    });
    status.set_text(message);
}
