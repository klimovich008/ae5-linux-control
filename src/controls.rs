use alsa::mixer::{Mixer, Selem, SelemChannelId};
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ControlError {
    Alsa(alsa::Error),
    Missing(String),
    Invalid(String),
    Verification(String),
}

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

#[derive(Debug)]
pub struct Ae5Mixer {
    mixer: Mixer,
}

pub fn snapshot_controls(card_index: i32) -> alsa::Result<Vec<ControlSnapshot>> {
    Ae5Mixer::open(card_index)?.snapshots()
}

impl Ae5Mixer {
    pub fn open(card_index: i32) -> alsa::Result<Self> {
        Ok(Self {
            mixer: Mixer::new(&format!("hw:{card_index}"), false)?,
        })
    }

    pub fn snapshots(&self) -> alsa::Result<Vec<ControlSnapshot>> {
        let mut controls = self
            .mixer
            .iter()
            .filter_map(Selem::new)
            .map(read_control)
            .collect::<alsa::Result<Vec<_>>>()?;
        controls.sort_unstable_by(|left, right| left.name.cmp(&right.name));
        Ok(controls)
    }

    pub fn snapshot(&self, name: &str) -> Result<ControlSnapshot, ControlError> {
        read_control(self.find(name)?).map_err(Into::into)
    }

    pub fn set_choice(&self, name: &str, requested: &str) -> Result<ControlSnapshot, ControlError> {
        let element = self.find(name)?;
        if !element.is_enumerated() {
            return Err(ControlError::Invalid(format!(
                "'{name}' is not an enumerated control"
            )));
        }

        let choices = element.iter_enum()?.collect::<alsa::Result<Vec<_>>>()?;
        let Some(index) = choice_index(&choices, requested) else {
            return Err(ControlError::Invalid(format!(
                "'{requested}' is not valid for '{name}'; expected one of: {}",
                choices.join(", ")
            )));
        };
        let expected = &choices[index];
        element.set_enum_item(SelemChannelId::FrontLeft, index as u32)?;
        let actual = read_control(element)?;
        if actual.selected.as_deref() != Some(expected) {
            return Err(ControlError::Verification(format!(
                "'{name}' read back as {:?}, expected '{expected}'",
                actual.selected
            )));
        }
        Ok(actual)
    }

    pub fn set_playback_switch(
        &self,
        name: &str,
        enabled: bool,
    ) -> Result<ControlSnapshot, ControlError> {
        let element = self.find(name)?;
        if !element.has_playback_switch() {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no playback switch"
            )));
        }
        element.set_playback_switch_all(i32::from(enabled))?;
        let actual = read_control(element)?;
        verify(name, "playback switch", enabled, actual.playback_switch)?;
        Ok(actual)
    }

    pub fn set_capture_switch(
        &self,
        name: &str,
        enabled: bool,
    ) -> Result<ControlSnapshot, ControlError> {
        let element = self.find(name)?;
        if !element.has_capture_switch() {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no capture switch"
            )));
        }
        element.set_capture_switch_all(i32::from(enabled))?;
        let actual = read_control(element)?;
        verify(name, "capture switch", enabled, actual.capture_switch)?;
        Ok(actual)
    }

    pub fn set_playback_level(
        &self,
        name: &str,
        value: i64,
    ) -> Result<ControlSnapshot, ControlError> {
        let element = self.find(name)?;
        if !element.has_playback_volume() {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no playback level"
            )));
        }
        let (min, max) = element.get_playback_volume_range();
        validate_range(name, value, min, max)?;
        element.set_playback_volume_all(value)?;
        let actual = read_control(element)?;
        verify(
            name,
            "playback level",
            value,
            actual.playback_level.as_ref().map(|level| level.value),
        )?;
        Ok(actual)
    }

    pub fn set_capture_level(
        &self,
        name: &str,
        value: i64,
    ) -> Result<ControlSnapshot, ControlError> {
        let element = self.find(name)?;
        if !element.has_capture_volume() {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no capture level"
            )));
        }
        let (min, max) = element.get_capture_volume_range();
        validate_range(name, value, min, max)?;
        element.set_capture_volume_all(value)?;
        let actual = read_control(element)?;
        verify(
            name,
            "capture level",
            value,
            actual.capture_level.as_ref().map(|level| level.value),
        )?;
        Ok(actual)
    }

    fn find(&self, name: &str) -> Result<Selem<'_>, ControlError> {
        self.mixer
            .find_selem(&alsa::mixer::SelemId::new(name, 0))
            .ok_or_else(|| ControlError::Missing(name.to_owned()))
    }
}

fn read_control(element: Selem<'_>) -> alsa::Result<ControlSnapshot> {
    let id = element.get_id();
    let name = id.get_name()?.to_owned();
    let (selected, choices) = if element.is_enumerated() {
        let choices = element.iter_enum()?.collect::<alsa::Result<Vec<_>>>()?;
        let selected_index = element.get_enum_item(SelemChannelId::FrontLeft)? as usize;
        (choices.get(selected_index).cloned(), choices)
    } else {
        (None, Vec::new())
    };

    Ok(ControlSnapshot {
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
    })
}

fn choice_index(choices: &[String], requested: &str) -> Option<usize> {
    choices
        .iter()
        .position(|choice| choice.eq_ignore_ascii_case(requested))
}

fn validate_range(name: &str, value: i64, min: i64, max: i64) -> Result<(), ControlError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(ControlError::Invalid(format!(
            "{value} is outside the valid range for '{name}' ({min}..{max})"
        )))
    }
}

fn verify<T>(name: &str, field: &str, expected: T, actual: Option<T>) -> Result<(), ControlError>
where
    T: Copy + fmt::Debug + PartialEq,
{
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(ControlError::Verification(format!(
            "'{name}' {field} read back as {actual:?}, expected {expected:?}"
        )))
    }
}

impl fmt::Display for ControlError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Alsa(error) => write!(output, "{error}"),
            Self::Missing(name) => write!(output, "ALSA control '{name}' is unavailable"),
            Self::Invalid(message) | Self::Verification(message) => output.write_str(message),
        }
    }
}

impl Error for ControlError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Alsa(error) => Some(error),
            _ => None,
        }
    }
}

impl From<alsa::Error> for ControlError {
    fn from(error: alsa::Error) -> Self {
        Self::Alsa(error)
    }
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

    #[test]
    fn validates_choices_and_ranges_before_hardware_writes() {
        let choices = vec!["Speakers".to_owned(), "Headphone".to_owned()];
        assert_eq!(choice_index(&choices, "headphone"), Some(1));
        assert_eq!(choice_index(&choices, "HDMI"), None);
        assert!(validate_range("Level", 50, 0, 100).is_ok());
        assert!(validate_range("Level", 101, 0, 100).is_err());
    }
}
