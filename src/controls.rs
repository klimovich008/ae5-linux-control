use alsa::mixer::{Mixer, Selem, SelemChannelId};
use std::error::Error;
use std::fmt;
use std::time::Duration;

const CHANNELS: &[SelemChannelId] = &[
    SelemChannelId::FrontLeft,
    SelemChannelId::FrontRight,
    SelemChannelId::RearLeft,
    SelemChannelId::RearRight,
    SelemChannelId::FrontCenter,
    SelemChannelId::Woofer,
    SelemChannelId::SideLeft,
    SelemChannelId::SideRight,
    SelemChannelId::RearCenter,
];

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
pub struct ChannelLevel {
    pub name: String,
    pub value: i64,
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
    pub playback_channels: Vec<ChannelLevel>,
    pub capture_channels: Vec<ChannelLevel>,
}

#[derive(Debug)]
pub struct Ae5Mixer {
    mixer: Mixer,
}

pub fn snapshot_controls(card_index: i32) -> alsa::Result<Vec<ControlSnapshot>> {
    Ae5Mixer::open(card_index)?.snapshots()
}

pub fn playback_switch_block_reason(
    name: &str,
    enabled: bool,
    controls: &[ControlSnapshot],
) -> Option<&'static str> {
    if !enabled {
        return None;
    }
    let speakers_selected = controls.iter().any(|control| {
        control.name == "Output Select" && control.selected.as_deref() == Some("Speakers")
    });
    let has_lfe = controls.iter().any(|control| {
        control.name == "Surround Channel Config"
            && control
                .selected
                .as_deref()
                .is_some_and(|layout| layout.ends_with(".1"))
    });

    match name {
        "Bass Redirection" if !speakers_selected => {
            Some("Select Speakers output before enabling bass redirection.")
        }
        "Bass Redirection" if !has_lfe => Some("Select a 2.1, 4.1, or 5.1 speaker layout first."),
        "Bass Redirection"
            if controls.iter().any(|control| {
                control.name == "FX: X-Bass" && control.playback_switch == Some(true)
            }) =>
        {
            Some("Turn off X-Bass before enabling speaker bass redirection.")
        }
        "FX: X-Bass" if speakers_selected && has_lfe => {
            Some("X-Bass is unavailable for speaker layouts with an LFE channel.")
        }
        _ => None,
    }
}

pub(crate) fn invalid_bass_state_reason(controls: &[ControlSnapshot]) -> Option<&'static str> {
    ["Bass Redirection", "FX: X-Bass"]
        .into_iter()
        .find_map(|name| {
            let enabled = controls
                .iter()
                .find(|control| control.name == name)
                .and_then(|control| control.playback_switch)
                .unwrap_or(false);
            playback_switch_block_reason(name, enabled, controls)
        })
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

    pub fn wait_for_event(&self, timeout: Duration) -> alsa::Result<bool> {
        let timeout_ms = timeout.as_millis().min(u32::MAX.into()) as u32;
        self.mixer.wait(Some(timeout_ms))?;
        Ok(self.mixer.handle_events()? > 0)
    }

    pub fn snapshot(&self, name: &str) -> Result<ControlSnapshot, ControlError> {
        read_control(self.find(name)?).map_err(Into::into)
    }

    pub fn set_choice(&self, name: &str, requested: &str) -> Result<ControlSnapshot, ControlError> {
        self.set_choice_checked(name, requested, false)
    }

    pub fn set_choice_checked(
        &self,
        name: &str,
        requested: &str,
        allow_high_gain: bool,
    ) -> Result<ControlSnapshot, ControlError> {
        if is_high_headphone_gain(name, requested) && !allow_high_gain {
            return Err(ControlError::Invalid(
                "high headphone gain requires explicit approval".to_owned(),
            ));
        }
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
        if matches!(name, "Output Select" | "Surround Channel Config") {
            let mut controls = self.snapshots()?;
            if let Some(control) = controls.iter_mut().find(|control| control.name == name)
                && control.selected.as_deref() != Some(expected)
            {
                control.selected = Some(expected.clone());
                if let Some(reason) = invalid_bass_state_reason(&controls) {
                    return Err(ControlError::Invalid(reason.to_owned()));
                }
            }
        }
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
        if enabled
            && matches!(name, "Bass Redirection" | "FX: X-Bass")
            && let Some(reason) = playback_switch_block_reason(name, true, &self.snapshots()?)
        {
            return Err(ControlError::Invalid(reason.to_owned()));
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
        verify_channels(name, "playback level", value, &actual.playback_channels)?;
        Ok(actual)
    }

    pub fn set_playback_channel_level(
        &self,
        name: &str,
        channel: &str,
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
        let channel_id = find_channel(&element, channel, false)?;
        element.set_playback_volume(channel_id, value)?;
        let actual = read_control(element)?;
        verify_channel(name, "playback", channel, value, &actual.playback_channels)?;
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
        verify_channels(name, "capture level", value, &actual.capture_channels)?;
        Ok(actual)
    }

    pub fn set_capture_channel_level(
        &self,
        name: &str,
        channel: &str,
        value: i64,
    ) -> Result<ControlSnapshot, ControlError> {
        let element = self.find(name)?;
        if !element.has_capture_volume() || name == "Bass Redirection Crossover" {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no capture level"
            )));
        }
        let (min, max) = element.get_capture_volume_range();
        validate_range(name, value, min, max)?;
        let channel_id = find_channel(&element, channel, true)?;
        element.set_capture_volume(channel_id, value)?;
        let actual = read_control(element)?;
        verify_channel(name, "capture", channel, value, &actual.capture_channels)?;
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
    let has_capture_level = name != "Bass Redirection Crossover" && element.has_capture_volume();
    let playback_channels = if element.has_playback_volume() {
        read_channels(&element, false)?
    } else {
        Vec::new()
    };
    let capture_channels = if has_capture_level {
        read_channels(&element, true)?
    } else {
        Vec::new()
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
        playback_level: playback_channels.first().map(|channel| {
            let (min, max) = element.get_playback_volume_range();
            Level {
                value: channel.value,
                min,
                max,
            }
        }),
        capture_level: capture_channels.first().map(|channel| {
            let (min, max) = element.get_capture_volume_range();
            Level {
                value: channel.value,
                min,
                max,
            }
        }),
        playback_channels,
        capture_channels,
    })
}

fn read_channels(element: &Selem<'_>, capture: bool) -> alsa::Result<Vec<ChannelLevel>> {
    CHANNELS
        .iter()
        .copied()
        .filter(|channel| {
            if capture {
                element.has_capture_channel(*channel)
            } else {
                element.has_playback_channel(*channel)
            }
        })
        .map(|channel| {
            let value = if capture {
                element.get_capture_volume(channel)?
            } else {
                element.get_playback_volume(channel)?
            };
            Ok(ChannelLevel {
                name: Selem::channel_name(channel)?.to_owned(),
                value,
            })
        })
        .collect()
}

fn find_channel(
    element: &Selem<'_>,
    requested: &str,
    capture: bool,
) -> Result<SelemChannelId, ControlError> {
    let channels = CHANNELS
        .iter()
        .copied()
        .filter(|channel| {
            if capture {
                element.has_capture_channel(*channel)
            } else {
                element.has_playback_channel(*channel)
            }
        })
        .collect::<Vec<_>>();
    channels
        .iter()
        .copied()
        .find(|channel| {
            Selem::channel_name(*channel).is_ok_and(|name| name.eq_ignore_ascii_case(requested))
        })
        .ok_or_else(|| {
            let choices = channels
                .iter()
                .filter_map(|channel| Selem::channel_name(*channel).ok())
                .collect::<Vec<_>>()
                .join(", ");
            ControlError::Invalid(format!(
                "'{requested}' is not a valid channel; expected one of: {choices}"
            ))
        })
}

fn choice_index(choices: &[String], requested: &str) -> Option<usize> {
    choices
        .iter()
        .position(|choice| choice.eq_ignore_ascii_case(requested))
}

fn is_high_headphone_gain(name: &str, requested: &str) -> bool {
    name == "AE-5: Headphone Gain" && requested.to_ascii_lowercase().starts_with("high")
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

fn verify_channels(
    name: &str,
    field: &str,
    expected: i64,
    actual: &[ChannelLevel],
) -> Result<(), ControlError> {
    if !actual.is_empty() && actual.iter().all(|channel| channel.value == expected) {
        Ok(())
    } else {
        Err(ControlError::Verification(format!(
            "'{name}' {field} read back as {actual:?}, expected every channel to be {expected}"
        )))
    }
}

fn verify_channel(
    name: &str,
    field: &str,
    channel: &str,
    expected: i64,
    actual: &[ChannelLevel],
) -> Result<(), ControlError> {
    let value = actual
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(channel))
        .map(|candidate| candidate.value);
    verify(
        name,
        &format!("{field} channel '{channel}' level"),
        expected,
        value,
    )
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
        if self.playback_channels.len() > 1 {
            write!(
                output,
                " | playback {}",
                format_channels(&self.playback_channels)
            )?;
        }
        if self.capture_channels.len() > 1 {
            write!(
                output,
                " | capture {}",
                format_channels(&self.capture_channels)
            )?;
        }
        Ok(())
    }
}

fn format_channels(channels: &[ChannelLevel]) -> String {
    channels
        .iter()
        .map(|channel| format!("{}={}", channel.name, channel.value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn on_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playback_switch(name: &str, enabled: bool) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: Some(enabled),
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        }
    }

    fn selected_choice(name: &str, selected: &str) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: Some(selected.to_owned()),
            choices: vec![selected.to_owned()],
            playback_switch: None,
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        }
    }

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
            playback_channels: vec![ChannelLevel {
                name: "Front Left".to_owned(),
                value: 65,
            }],
            capture_channels: Vec::new(),
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
        assert!(is_high_headphone_gain(
            "AE-5: Headphone Gain",
            "High (150-600 Ohms)"
        ));
        assert!(!is_high_headphone_gain(
            "AE-5: Headphone Gain",
            "Medium (32-149 Ohms)"
        ));
        assert!(validate_range("Level", 50, 0, 100).is_ok());
        assert!(validate_range("Level", 101, 0, 100).is_err());
    }

    #[test]
    fn rejects_incompatible_bass_features_but_always_allows_disabling() {
        let controls = vec![
            playback_switch("FX: X-Bass", true),
            playback_switch("Bass Redirection", false),
            selected_choice("Surround Channel Config", "5.1"),
            selected_choice("Output Select", "Speakers"),
        ];

        assert_eq!(
            playback_switch_block_reason("Bass Redirection", true, &controls),
            Some("Turn off X-Bass before enabling speaker bass redirection.")
        );
        assert_eq!(
            playback_switch_block_reason("FX: X-Bass", false, &controls),
            None
        );
        assert_eq!(
            invalid_bass_state_reason(&controls),
            Some("X-Bass is unavailable for speaker layouts with an LFE channel.")
        );

        let controls = vec![
            playback_switch("FX: X-Bass", false),
            playback_switch("Bass Redirection", false),
            selected_choice("Surround Channel Config", "2.0"),
            selected_choice("Output Select", "Speakers"),
        ];
        assert_eq!(
            playback_switch_block_reason("Bass Redirection", true, &controls),
            Some("Select a 2.1, 4.1, or 5.1 speaker layout first.")
        );

        let controls = vec![
            playback_switch("FX: X-Bass", false),
            playback_switch("Bass Redirection", false),
            selected_choice("Surround Channel Config", "5.1"),
            selected_choice("Output Select", "Headphone"),
        ];
        assert_eq!(
            playback_switch_block_reason("Bass Redirection", true, &controls),
            Some("Select Speakers output before enabling bass redirection.")
        );
        assert_eq!(
            playback_switch_block_reason("FX: X-Bass", true, &controls),
            None
        );
    }
}
