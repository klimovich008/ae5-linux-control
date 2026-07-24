use crate::{Ae5Mixer, ControlError, ControlSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

const FORMAT_VERSION: u32 = 1;
const TARGET: &str = "1102:0012/1102:0051";
const MAX_PROFILE_BYTES: u64 = 1024 * 1024;
const MAX_CONTROLS: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub format_version: u32,
    pub name: String,
    pub target: String,
    pub controls: BTreeMap<String, ProfileControl>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProfileControl {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_switch: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_switch: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playback_level: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_level: Option<i64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub playback_channels: BTreeMap<String, i64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub capture_channels: BTreeMap<String, i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApplyReport {
    pub controls_applied: usize,
}

#[derive(Debug)]
pub enum ProfileError {
    Io(io::Error),
    Json(serde_json::Error),
    Control(ControlError),
    Invalid(String),
    Apply {
        failure: String,
        rollback_failure: Option<String>,
    },
}

impl Profile {
    pub fn new(
        name: &str,
        controls: BTreeMap<String, ProfileControl>,
    ) -> Result<Self, ProfileError> {
        let profile = Self {
            format_version: FORMAT_VERSION,
            name: name.to_owned(),
            target: TARGET.to_owned(),
            controls,
        };
        profile.validate_structure()?;
        Ok(profile)
    }

    pub fn capture(name: &str, controls: Vec<ControlSnapshot>) -> Result<Self, ProfileError> {
        Self::new(
            name,
            controls
                .into_iter()
                .filter_map(|control| {
                    let name = control.name.clone();
                    let value = ProfileControl::from(control);
                    (!value.is_empty()).then_some((name, value))
                })
                .collect(),
        )
    }

    pub fn load(path: &Path) -> Result<Self, ProfileError> {
        let file = fs::File::open(path)?;
        if file.metadata()?.len() > MAX_PROFILE_BYTES {
            return Err(ProfileError::Invalid(format!(
                "profile exceeds the {MAX_PROFILE_BYTES}-byte limit"
            )));
        }
        let mut contents = Vec::new();
        file.take(MAX_PROFILE_BYTES + 1)
            .read_to_end(&mut contents)?;
        if contents.len() as u64 > MAX_PROFILE_BYTES {
            return Err(ProfileError::Invalid(format!(
                "profile exceeds the {MAX_PROFILE_BYTES}-byte limit"
            )));
        }
        let profile: Self = serde_json::from_slice(&contents)?;
        profile.validate_structure()?;
        Ok(profile)
    }

    pub fn save_new(&self, path: &Path) -> Result<(), ProfileError> {
        self.validate_structure()?;
        let contents = serde_json::to_vec_pretty(self)?;
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(&contents)?;
        file.write_all(b"\n")?;
        Ok(())
    }

    pub fn check(&self, mixer: &Ae5Mixer, allow_high_gain: bool) -> Result<(), ProfileError> {
        self.validate_against(mixer, allow_high_gain).map(|_| ())
    }

    pub fn apply(
        &self,
        mixer: &Ae5Mixer,
        allow_high_gain: bool,
    ) -> Result<ApplyReport, ProfileError> {
        let before = self.validate_against(mixer, allow_high_gain)?;
        if let Err(failure) = apply_controls(mixer, &self.controls, allow_high_gain) {
            let rollback_failure = apply_controls(mixer, &before, true)
                .err()
                .map(|error| error.to_string());
            return Err(ProfileError::Apply {
                failure: failure.to_string(),
                rollback_failure,
            });
        }
        Ok(ApplyReport {
            controls_applied: self.controls.len(),
        })
    }

    fn validate_structure(&self) -> Result<(), ProfileError> {
        if self.format_version != FORMAT_VERSION {
            return Err(ProfileError::Invalid(format!(
                "unsupported profile format version {}",
                self.format_version
            )));
        }
        if self.target != TARGET {
            return Err(ProfileError::Invalid(format!(
                "profile targets '{}', expected '{TARGET}'",
                self.target
            )));
        }
        let name_length = self.name.trim().chars().count();
        if !(1..=80).contains(&name_length) {
            return Err(ProfileError::Invalid(
                "profile name must contain 1 to 80 characters".to_owned(),
            ));
        }
        if self.controls.is_empty() || self.controls.len() > MAX_CONTROLS {
            return Err(ProfileError::Invalid(format!(
                "profile must contain 1 to {MAX_CONTROLS} controls"
            )));
        }
        if let Some(name) = self
            .controls
            .iter()
            .find_map(|(name, value)| (name.is_empty() || value.is_empty()).then_some(name))
        {
            return Err(ProfileError::Invalid(format!(
                "profile control '{name}' has no value"
            )));
        }
        Ok(())
    }

    fn validate_against(
        &self,
        mixer: &Ae5Mixer,
        allow_high_gain: bool,
    ) -> Result<BTreeMap<String, ProfileControl>, ProfileError> {
        self.validate_structure()?;
        let mut before = BTreeMap::new();
        for (name, requested) in &self.controls {
            let current = mixer.snapshot(name)?;
            validate_control(name, requested, &current, allow_high_gain)?;
            before.insert(name.clone(), ProfileControl::from(current));
        }
        Ok(before)
    }
}

impl ProfileControl {
    fn is_empty(&self) -> bool {
        self.choice.is_none()
            && self.playback_switch.is_none()
            && self.capture_switch.is_none()
            && self.playback_level.is_none()
            && self.capture_level.is_none()
            && self.playback_channels.is_empty()
            && self.capture_channels.is_empty()
    }
}

impl From<ControlSnapshot> for ProfileControl {
    fn from(control: ControlSnapshot) -> Self {
        let playback_channels = profile_channels(&control.playback_channels);
        let capture_channels = profile_channels(&control.capture_channels);
        Self {
            choice: control.selected,
            playback_switch: control.playback_switch,
            capture_switch: control.capture_switch,
            playback_level: control.playback_level.and_then(|level| {
                (level.min..=level.max)
                    .contains(&level.value)
                    .then_some(level.value)
            }),
            capture_level: control.capture_level.and_then(|level| {
                (level.min..=level.max)
                    .contains(&level.value)
                    .then_some(level.value)
            }),
            playback_channels,
            capture_channels,
        }
    }
}

fn profile_channels(channels: &[crate::ChannelLevel]) -> BTreeMap<String, i64> {
    if channels.len() < 2 {
        BTreeMap::new()
    } else {
        channels
            .iter()
            .map(|channel| (channel.name.clone(), channel.value))
            .collect()
    }
}

fn validate_control(
    name: &str,
    requested: &ProfileControl,
    current: &ControlSnapshot,
    allow_high_gain: bool,
) -> Result<(), ProfileError> {
    if let Some(choice) = &requested.choice {
        if current
            .choices
            .iter()
            .all(|candidate| !candidate.eq_ignore_ascii_case(choice))
        {
            return Err(ProfileError::Invalid(format!(
                "'{choice}' is not valid for '{name}'"
            )));
        }
        if name == "AE-5: Headphone Gain"
            && choice.to_ascii_lowercase().starts_with("high")
            && !allow_high_gain
        {
            return Err(ProfileError::Invalid(
                "high headphone gain requires explicit approval".to_owned(),
            ));
        }
    }
    validate_field(
        name,
        "playback switch",
        requested.playback_switch,
        current.playback_switch,
    )?;
    validate_field(
        name,
        "capture switch",
        requested.capture_switch,
        current.capture_switch,
    )?;
    validate_level(
        name,
        "playback level",
        requested.playback_level,
        current.playback_level.as_ref().map(|level| level.value),
        current
            .playback_level
            .as_ref()
            .map(|level| (level.min, level.max)),
    )?;
    validate_level(
        name,
        "capture level",
        requested.capture_level,
        current.capture_level.as_ref().map(|level| level.value),
        current
            .capture_level
            .as_ref()
            .map(|level| (level.min, level.max)),
    )?;
    validate_channels(
        name,
        "playback",
        &requested.playback_channels,
        &current.playback_channels,
        current
            .playback_level
            .as_ref()
            .map(|level| (level.min, level.max)),
    )?;
    validate_channels(
        name,
        "capture",
        &requested.capture_channels,
        &current.capture_channels,
        current
            .capture_level
            .as_ref()
            .map(|level| (level.min, level.max)),
    )
}

fn validate_field<T: Copy>(
    name: &str,
    field: &str,
    requested: Option<T>,
    available: Option<T>,
) -> Result<(), ProfileError> {
    if requested.is_some() && available.is_none() {
        Err(ProfileError::Invalid(format!("'{name}' has no {field}")))
    } else {
        Ok(())
    }
}

fn validate_level(
    name: &str,
    field: &str,
    requested: Option<i64>,
    available: Option<i64>,
    range: Option<(i64, i64)>,
) -> Result<(), ProfileError> {
    validate_field(name, field, requested, available)?;
    if let (Some(value), Some((min, max))) = (requested, range)
        && !(min..=max).contains(&value)
    {
        return Err(ProfileError::Invalid(format!(
            "{value} is outside the valid range for '{name}' {field} ({min}..{max})"
        )));
    }
    Ok(())
}

fn validate_channels(
    name: &str,
    field: &str,
    requested: &BTreeMap<String, i64>,
    available: &[crate::ChannelLevel],
    range: Option<(i64, i64)>,
) -> Result<(), ProfileError> {
    for (channel, value) in requested {
        if available.iter().all(|item| item.name != *channel) {
            return Err(ProfileError::Invalid(format!(
                "'{name}' has no {field} channel '{channel}'"
            )));
        }
        let Some((min, max)) = range else {
            return Err(ProfileError::Invalid(format!(
                "'{name}' has no {field} level"
            )));
        };
        if !(min..=max).contains(value) {
            return Err(ProfileError::Invalid(format!(
                "{value} is outside the valid range for '{name}' {field} channel '{channel}' ({min}..{max})"
            )));
        }
    }
    Ok(())
}

fn apply_controls(
    mixer: &Ae5Mixer,
    controls: &BTreeMap<String, ProfileControl>,
    allow_high_gain: bool,
) -> Result<(), ControlError> {
    for (name, control) in controls {
        let current = mixer.snapshot(name)?;
        if let Some(choice) = &control.choice
            && current
                .selected
                .as_ref()
                .is_none_or(|value| !value.eq_ignore_ascii_case(choice))
        {
            mixer.set_choice_checked(name, choice, allow_high_gain)?;
        }
        if let Some(value) = control.playback_switch
            && current.playback_switch != Some(value)
        {
            mixer.set_playback_switch(name, value)?;
        }
        if let Some(value) = control.capture_switch
            && current.capture_switch != Some(value)
        {
            mixer.set_capture_switch(name, value)?;
        }
        if let Some(value) = control.playback_level
            && current.playback_level.as_ref().map(|level| level.value) != Some(value)
        {
            mixer.set_playback_level(name, value)?;
        }
        if let Some(value) = control.capture_level
            && current.capture_level.as_ref().map(|level| level.value) != Some(value)
        {
            mixer.set_capture_level(name, value)?;
        }
        for (channel, value) in &control.playback_channels {
            mixer.set_playback_channel_level(name, channel, *value)?;
        }
        for (channel, value) in &control.capture_channels {
            mixer.set_capture_channel_level(name, channel, *value)?;
        }
    }
    Ok(())
}

impl fmt::Display for ProfileError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(output, "{error}"),
            Self::Json(error) => write!(output, "invalid profile JSON: {error}"),
            Self::Control(error) => write!(output, "{error}"),
            Self::Invalid(message) => output.write_str(message),
            Self::Apply {
                failure,
                rollback_failure: None,
            } => write!(
                output,
                "profile apply failed and was rolled back: {failure}"
            ),
            Self::Apply {
                failure,
                rollback_failure: Some(rollback),
            } => write!(
                output,
                "profile apply failed: {failure}; rollback also failed: {rollback}"
            ),
        }
    }
}

impl Error for ProfileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Control(error) => Some(error),
            _ => None,
        }
    }
}

impl From<io::Error> for ProfileError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for ProfileError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<ControlError> for ProfileError {
    fn from(error: ControlError) -> Self {
        Self::Control(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    fn sample_profile() -> Profile {
        Profile {
            format_version: FORMAT_VERSION,
            name: "Headphones".to_owned(),
            target: TARGET.to_owned(),
            controls: BTreeMap::from([(
                "Output Select".to_owned(),
                ProfileControl {
                    choice: Some("Headphone".to_owned()),
                    ..ProfileControl::default()
                },
            )]),
        }
    }

    #[test]
    fn native_profile_round_trips_as_json() {
        let mut profile = sample_profile();
        profile.controls.insert(
            "Front".to_owned(),
            ProfileControl {
                playback_level: Some(90),
                playback_channels: BTreeMap::from([
                    ("Front Left".to_owned(), 90),
                    ("Front Right".to_owned(), 82),
                ]),
                ..ProfileControl::default()
            },
        );
        let json = serde_json::to_string(&profile).unwrap();
        assert_eq!(serde_json::from_str::<Profile>(&json).unwrap(), profile);
    }

    #[test]
    fn accepts_legacy_scalar_profile_without_channel_maps() {
        let json = r#"{
            "format_version": 1,
            "name": "Legacy headphones",
            "target": "1102:0012/1102:0051",
            "controls": {
                "Front": {
                    "playback_level": 90
                }
            }
        }"#;

        let profile: Profile = serde_json::from_str(json).unwrap();
        let front = &profile.controls["Front"];
        assert_eq!(front.playback_level, Some(90));
        assert!(front.playback_channels.is_empty());
        assert!(front.capture_channels.is_empty());
    }

    #[test]
    fn captures_stereo_balance_without_changing_legacy_level() {
        let control = ControlSnapshot {
            name: "Front".to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: Some(true),
            capture_switch: None,
            playback_level: Some(crate::Level {
                value: 90,
                min: 0,
                max: 99,
            }),
            capture_level: None,
            playback_channels: vec![
                crate::ChannelLevel {
                    name: "Front Left".to_owned(),
                    value: 90,
                },
                crate::ChannelLevel {
                    name: "Front Right".to_owned(),
                    value: 82,
                },
            ],
            capture_channels: Vec::new(),
        };

        let profile = ProfileControl::from(control);
        assert_eq!(profile.playback_level, Some(90));
        assert_eq!(profile.playback_channels["Front Left"], 90);
        assert_eq!(profile.playback_channels["Front Right"], 82);
    }

    #[test]
    fn rejects_unknown_versions_and_empty_controls() {
        let mut profile = sample_profile();
        profile.format_version = 2;
        assert!(profile.validate_structure().is_err());

        profile = sample_profile();
        profile.controls.clear();
        assert!(profile.validate_structure().is_err());
    }

    #[test]
    fn rejects_malformed_oversized_and_overwritten_files() {
        let malformed = test_path();
        fs::write(&malformed, b"{").unwrap();
        assert!(matches!(
            Profile::load(&malformed),
            Err(ProfileError::Json(_))
        ));
        fs::remove_file(malformed).unwrap();

        let oversized = test_path();
        fs::File::create(&oversized)
            .unwrap()
            .set_len(MAX_PROFILE_BYTES + 1)
            .unwrap();
        assert!(matches!(
            Profile::load(&oversized),
            Err(ProfileError::Invalid(_))
        ));
        fs::remove_file(oversized).unwrap();

        let profile_path = test_path();
        let profile = sample_profile();
        profile.save_new(&profile_path).unwrap();
        assert!(matches!(
            profile.save_new(&profile_path),
            Err(ProfileError::Io(error)) if error.kind() == io::ErrorKind::AlreadyExists
        ));
        fs::remove_file(profile_path).unwrap();
    }

    fn test_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "ae5-profile-test-{}-{}.json",
            std::process::id(),
            NEXT_FILE.fetch_add(1, Ordering::Relaxed)
        ))
    }
}
