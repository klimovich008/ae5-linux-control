use alsa::mixer::{Mixer, Selem, SelemChannelId};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Level {
    pub value: i64,
    pub min: i64,
    pub max: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlSnapshot {
    pub name: String,
    pub selected: Option<String>,
    pub choices: Vec<String>,
    pub playback_switch: Option<bool>,
    pub capture_switch: Option<bool>,
    pub playback_level: Option<Level>,
    pub capture_level: Option<Level>,
}

pub fn snapshot_controls(card_index: i32) -> alsa::Result<Vec<ControlSnapshot>> {
    let mixer = Mixer::new(&format!("hw:{card_index}"), false)?;
    let mut controls = Vec::new();

    for element in mixer.iter().filter_map(Selem::new) {
        let id = element.get_id();
        let name = id.get_name()?.to_owned();
        let (selected, choices) = if element.is_enumerated() {
            let choices = element.iter_enum()?.collect::<alsa::Result<Vec<_>>>()?;
            let selected_index = element.get_enum_item(SelemChannelId::FrontLeft)? as usize;
            (choices.get(selected_index).cloned(), choices)
        } else {
            (None, Vec::new())
        };

        controls.push(ControlSnapshot {
            name,
            selected,
            choices,
            playback_switch: element
                .has_playback_switch()
                .then(|| {
                    element
                        .get_playback_switch(SelemChannelId::FrontLeft)
                        .map(|value| value != 0)
                })
                .transpose()?,
            capture_switch: element
                .has_capture_switch()
                .then(|| {
                    element
                        .get_capture_switch(SelemChannelId::FrontLeft)
                        .map(|value| value != 0)
                })
                .transpose()?,
            playback_level: element
                .has_playback_volume()
                .then(|| {
                    let (min, max) = element.get_playback_volume_range();
                    element
                        .get_playback_volume(SelemChannelId::FrontLeft)
                        .map(|value| Level { value, min, max })
                })
                .transpose()?,
            capture_level: element
                .has_capture_volume()
                .then(|| {
                    let (min, max) = element.get_capture_volume_range();
                    element
                        .get_capture_volume(SelemChannelId::FrontLeft)
                        .map(|value| Level { value, min, max })
                })
                .transpose()?,
        });
    }

    controls.sort_unstable_by(|left, right| left.name.cmp(&right.name));
    Ok(controls)
}

impl fmt::Display for ControlSnapshot {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{}", self.name)?;
        if let Some(selected) = &self.selected {
            write!(output, ": {selected}")?;
        }
        if let Some(enabled) = self.playback_switch {
            write!(output, " | playback {}", on_off(enabled))?;
        }
        if let Some(level) = &self.playback_level {
            write!(
                output,
                " | playback level {} [{}..{}]",
                level.value, level.min, level.max
            )?;
        }
        if let Some(enabled) = self.capture_switch {
            write!(output, " | capture {}", on_off(enabled))?;
        }
        if let Some(level) = &self.capture_level {
            write!(
                output,
                " | capture level {} [{}..{}]",
                level.value, level.min, level.max
            )?;
        }
        Ok(())
    }
}

fn on_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_a_compound_control_readably() {
        let control = ControlSnapshot {
            name: "FX: Crystalizer".to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: Some(true),
            capture_switch: None,
            playback_level: Some(Level {
                value: 65,
                min: 0,
                max: 100,
            }),
            capture_level: None,
        };

        assert_eq!(
            control.to_string(),
            "FX: Crystalizer | playback on | playback level 65 [0..100]"
        );
    }
}
