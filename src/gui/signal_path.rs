//! The signal-path spine.
//!
//! The card is not a menu of features; it is a chain. Audio enters as PCM,
//! passes the CA0132 effect processor, crosses the output stage, and leaves
//! through a jack at a chosen gain. Every fault this project has chased was a
//! chain fault — one stage not being what the user believed it was: a muted
//! output behind an unmuted desktop, a DSP oscillating with nothing playing, a
//! profile that read back correctly and applied nothing.
//!
//! So the chain is the primary structure rather than a footer afterthought,
//! and colour here carries exactly one meaning: whether signal is passing,
//! wants attention, or is stopped.

use gtk::prelude::*;

use crate::ControlSnapshot;

/// Whether a stage is passing signal, wants attention, or is stopping it.
///
/// This is the only thing colour encodes in the spine. Reusing a hue for
/// branding or emphasis elsewhere would dilute it.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum StageState {
    /// Signal passes and the stage is at a safe setting.
    Passing,
    /// Signal passes but something is worth knowing about.
    Attention,
    /// Signal stops here.
    Blocked,
    /// The control backing this stage was not found.
    Unknown,
}

impl StageState {
    fn css_class(self) -> &'static str {
        match self {
            Self::Passing => "stage-passing",
            Self::Attention => "stage-attention",
            Self::Blocked => "stage-blocked",
            Self::Unknown => "stage-unknown",
        }
    }

    /// Text carries the state as well as colour, so the spine is readable
    /// without relying on hue.
    fn mark(self) -> &'static str {
        match self {
            Self::Passing => "\u{25cf}",
            Self::Attention => "\u{25b2}",
            Self::Blocked => "\u{25a0}",
            Self::Unknown => "\u{25cb}",
        }
    }
}

struct Stage {
    label: &'static str,
    reading: String,
    state: StageState,
}

fn switch_of(controls: &[ControlSnapshot], name: &str) -> Option<bool> {
    controls
        .iter()
        .find(|control| control.name == name)
        .and_then(|control| control.playback_switch)
}

fn effects_stage(controls: &[ControlSnapshot]) -> Stage {
    const EFFECTS: [&str; 5] = [
        "FX: Surround",
        "FX: Crystalizer",
        "FX: X-Bass",
        "FX: Smart Volume",
        "FX: Dialog Plus",
    ];

    match switch_of(controls, "Enable OutFX") {
        Some(true) => {
            let active = EFFECTS
                .iter()
                .filter(|name| switch_of(controls, name) == Some(true))
                .count();
            Stage {
                label: "Processing",
                reading: match active {
                    0 => "on, none active".to_owned(),
                    1 => "1 effect".to_owned(),
                    many => format!("{many} effects"),
                },
                state: StageState::Passing,
            }
        }
        Some(false) => Stage {
            label: "Processing",
            reading: "bypassed".to_owned(),
            state: StageState::Attention,
        },
        None => Stage {
            label: "Processing",
            reading: "unknown".to_owned(),
            state: StageState::Unknown,
        },
    }
}

fn output_stage(controls: &[ControlSnapshot]) -> Stage {
    let master = controls.iter().find(|control| control.name == "Master");
    let Some(master) = master else {
        return Stage {
            label: "Output",
            reading: "unknown".to_owned(),
            state: StageState::Unknown,
        };
    };

    if master.playback_switch == Some(false) {
        // The single most consequential fact the interface can report. It used
        // to be footer small-caps beside a decorative build string.
        return Stage {
            label: "Output",
            reading: "muted".to_owned(),
            state: StageState::Blocked,
        };
    }

    let reading = match &master.playback_level {
        Some(level) if level.value == level.max => "0 dB".to_owned(),
        Some(level) => format!("{}/{}", level.value, level.max),
        None => "on".to_owned(),
    };
    Stage {
        label: "Output",
        reading,
        state: StageState::Passing,
    }
}

fn jack_stage(controls: &[ControlSnapshot]) -> Stage {
    let route = controls
        .iter()
        .find(|control| control.name == "Output Select")
        .and_then(|control| control.selected.as_deref());
    let gain = controls
        .iter()
        .find(|control| control.name == "AE-5: Headphone Gain")
        .and_then(|control| control.selected.as_deref());

    // High gain is not a fault, but on this host it is always worth seeing:
    // every acoustic test in this project is required to stay on Low.
    let high_gain = gain.is_some_and(|gain| gain.starts_with("High"));
    let reading = match (route, gain) {
        (Some(route), Some(gain)) => {
            let gain = gain.split_whitespace().next().unwrap_or(gain);
            format!("{route}, {gain} gain")
        }
        (Some(route), None) => route.to_owned(),
        _ => "unknown".to_owned(),
    };

    Stage {
        label: "Jack",
        reading,
        state: match (route.is_some(), high_gain) {
            (false, _) => StageState::Unknown,
            (true, true) => StageState::Attention,
            (true, false) => StageState::Passing,
        },
    }
}

fn stage_widget(stage: &Stage) -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 3);
    column.add_css_class("path-stage");
    column.add_css_class(stage.state.css_class());

    let label = gtk::Label::new(Some(stage.label));
    label.set_xalign(0.0);
    label.add_css_class("path-stage-label");

    let reading_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let mark = gtk::Label::new(Some(stage.state.mark()));
    mark.add_css_class("path-stage-mark");
    let reading = gtk::Label::new(Some(&stage.reading));
    reading.set_xalign(0.0);
    // Monospace is reserved for readings. If it is in this face, it is
    // something the hardware reported.
    reading.add_css_class("path-stage-reading");
    reading_row.append(&mark);
    reading_row.append(&reading);

    column.append(&label);
    column.append(&reading_row);
    column.update_property(&[gtk::accessible::Property::Label(&format!(
        "{}: {}",
        stage.label, stage.reading
    ))]);
    column
}

/// Build the signal path across the card, source-side first.
pub fn signal_path(controls: &[ControlSnapshot]) -> gtk::Box {
    let path = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    path.add_css_class("signal-path");

    let stages = [
        effects_stage(controls),
        output_stage(controls),
        jack_stage(controls),
    ];
    let blocked = stages
        .iter()
        .any(|stage| stage.state == StageState::Blocked);

    for (index, stage) in stages.iter().enumerate() {
        if index > 0 {
            // The arrow states the direction signal travels; it is content,
            // not ornament.
            let link = gtk::Label::new(Some("\u{2192}"));
            link.add_css_class("path-link");
            path.append(&link);
        }
        path.append(&stage_widget(stage));
    }

    if blocked {
        path.add_css_class("signal-path-blocked");
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Level;

    fn control(name: &str) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: None,
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        }
    }

    fn switch(name: &str, on: bool) -> ControlSnapshot {
        ControlSnapshot {
            playback_switch: Some(on),
            ..control(name)
        }
    }

    fn choice(name: &str, selected: &str) -> ControlSnapshot {
        ControlSnapshot {
            selected: Some(selected.to_owned()),
            ..control(name)
        }
    }

    #[test]
    fn a_muted_output_stage_blocks_the_path() {
        let controls = vec![switch("Master", false)];
        let stage = output_stage(&controls);
        assert_eq!(stage.state, StageState::Blocked);
        assert_eq!(stage.reading, "muted");
    }

    #[test]
    fn an_output_at_its_ceiling_reads_as_zero_db() {
        let controls = vec![ControlSnapshot {
            playback_switch: Some(true),
            playback_level: Some(Level {
                value: 99,
                min: 0,
                max: 99,
            }),
            ..control("Master")
        }];
        let stage = output_stage(&controls);
        assert_eq!(stage.state, StageState::Passing);
        assert_eq!(stage.reading, "0 dB");
    }

    #[test]
    fn a_bypassed_processor_wants_attention_rather_than_looking_healthy() {
        // Effects off is a legitimate state, but it is not the state the user
        // configured, so it must not render as "fine".
        let stage = effects_stage(&[switch("Enable OutFX", false)]);
        assert_eq!(stage.state, StageState::Attention);
        assert_eq!(stage.reading, "bypassed");
    }

    #[test]
    fn active_effects_are_counted_with_natural_grammar() {
        let one = effects_stage(&[switch("Enable OutFX", true), switch("FX: X-Bass", true)]);
        assert_eq!(one.reading, "1 effect");

        let several = effects_stage(&[
            switch("Enable OutFX", true),
            switch("FX: X-Bass", true),
            switch("FX: Crystalizer", true),
        ]);
        assert_eq!(several.reading, "2 effects");

        let none = effects_stage(&[switch("Enable OutFX", true)]);
        assert_eq!(none.reading, "on, none active");
    }

    #[test]
    fn high_headphone_gain_is_always_surfaced() {
        // The project forbids High gain for acoustic tests, so it may never be
        // indistinguishable from Low in the interface.
        let controls = vec![
            choice("Output Select", "Headphone"),
            choice("AE-5: Headphone Gain", "High (150-600 Ohms)"),
        ];
        let stage = jack_stage(&controls);
        assert_eq!(stage.state, StageState::Attention);
        assert_eq!(stage.reading, "Headphone, High gain");

        let low = vec![
            choice("Output Select", "Headphone"),
            choice("AE-5: Headphone Gain", "Low (16-31  Ohms)"),
        ];
        assert_eq!(jack_stage(&low).state, StageState::Passing);
    }

    #[test]
    fn missing_controls_read_as_unknown_never_as_healthy() {
        assert_eq!(output_stage(&[]).state, StageState::Unknown);
        assert_eq!(effects_stage(&[]).state, StageState::Unknown);
        assert_eq!(jack_stage(&[]).state, StageState::Unknown);
    }

    #[test]
    fn every_state_has_a_distinct_mark_so_colour_is_not_load_bearing() {
        let marks = [
            StageState::Passing.mark(),
            StageState::Attention.mark(),
            StageState::Blocked.mark(),
            StageState::Unknown.mark(),
        ];
        let unique: std::collections::BTreeSet<_> = marks.iter().collect();
        assert_eq!(unique.len(), marks.len());
    }
}
