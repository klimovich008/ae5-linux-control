use crate::controls::{
    EQUALIZER_PRESET_CONTROL, capture_control_block_reason, equalizer_band_block_reason,
    invalid_bass_state_reason, is_equalizer_band,
};
use crate::{Ae5Mixer, ControlError, ControlSnapshot};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

const FORMAT_VERSION: u32 = 1;
const TARGET: &str = "1102:0012/1102:0051";
const MAX_PROFILE_BYTES: u64 = 1024 * 1024;
const MAX_CONTROLS: usize = 128;
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

pub const LINUX_DRIVER_DEFAULTS_PRESERVED: &[&str] = &[
    "output selection and headphone auto-detect",
    "input selection and microphone boost",
    "speaker layout, full-range flags, and bass redirection",
    "playback and capture volumes, balances, and mutes",
    "PipeWire routing and sample-rate configuration",
];

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

trait ProfileMixer {
    fn snapshots(&self) -> Result<Vec<ControlSnapshot>, ControlError>;
    fn snapshot(&self, name: &str) -> Result<ControlSnapshot, ControlError>;
    fn set_choice(
        &self,
        name: &str,
        choice: &str,
        allow_high_gain: bool,
    ) -> Result<(), ControlError>;
    fn set_playback_switch(&self, name: &str, enabled: bool) -> Result<(), ControlError>;
    fn set_capture_switch(&self, name: &str, enabled: bool) -> Result<(), ControlError>;
    fn set_playback_level(&self, name: &str, value: i64) -> Result<(), ControlError>;
    fn set_capture_level(&self, name: &str, value: i64) -> Result<(), ControlError>;
    fn set_playback_channel_level(
        &self,
        name: &str,
        channel: &str,
        value: i64,
    ) -> Result<(), ControlError>;
    fn set_capture_channel_level(
        &self,
        name: &str,
        channel: &str,
        value: i64,
    ) -> Result<(), ControlError>;
}

impl ProfileMixer for Ae5Mixer {
    fn snapshots(&self) -> Result<Vec<ControlSnapshot>, ControlError> {
        Ae5Mixer::snapshots(self).map_err(Into::into)
    }

    fn snapshot(&self, name: &str) -> Result<ControlSnapshot, ControlError> {
        Ae5Mixer::snapshot(self, name)
    }

    fn set_choice(
        &self,
        name: &str,
        choice: &str,
        allow_high_gain: bool,
    ) -> Result<(), ControlError> {
        Ae5Mixer::set_choice_checked(self, name, choice, allow_high_gain).map(drop)
    }

    fn set_playback_switch(&self, name: &str, enabled: bool) -> Result<(), ControlError> {
        Ae5Mixer::set_playback_switch(self, name, enabled).map(drop)
    }

    fn set_capture_switch(&self, name: &str, enabled: bool) -> Result<(), ControlError> {
        Ae5Mixer::set_capture_switch(self, name, enabled).map(drop)
    }

    fn set_playback_level(&self, name: &str, value: i64) -> Result<(), ControlError> {
        Ae5Mixer::set_playback_level(self, name, value).map(drop)
    }

    fn set_capture_level(&self, name: &str, value: i64) -> Result<(), ControlError> {
        Ae5Mixer::set_capture_level(self, name, value).map(drop)
    }

    fn set_playback_channel_level(
        &self,
        name: &str,
        channel: &str,
        value: i64,
    ) -> Result<(), ControlError> {
        Ae5Mixer::set_playback_channel_level(self, name, channel, value).map(drop)
    }

    fn set_capture_channel_level(
        &self,
        name: &str,
        channel: &str,
        value: i64,
    ) -> Result<(), ControlError> {
        Ae5Mixer::set_capture_channel_level(self, name, channel, value).map(drop)
    }
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
        let omit_equalizer_bands = equalizer_band_block_reason("EQ Band0", &controls).is_some();
        Self::new(
            name,
            controls
                .into_iter()
                .filter_map(|control| {
                    let name = control.name.clone();
                    if capture_control_block_reason(&name).is_some()
                        || omit_equalizer_bands && is_equalizer_band(&name)
                    {
                        return None;
                    }
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

    pub(crate) fn save_replace(&self, path: &Path) -> Result<(), ProfileError> {
        self.validate_structure()?;
        let contents = serde_json::to_vec_pretty(self)?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let file_name = path
            .file_name()
            .ok_or_else(|| ProfileError::Invalid("profile path has no file name".to_owned()))?
            .to_string_lossy();

        let (temporary, mut file) = loop {
            let candidate = parent.join(format!(
                ".{file_name}.{}.{}.tmp",
                std::process::id(),
                NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(file) => break (candidate, file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        };

        let result = (|| -> io::Result<()> {
            if let Ok(metadata) = fs::metadata(path) {
                file.set_permissions(metadata.permissions())?;
            }
            file.write_all(&contents)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result.map_err(Into::into)
    }

    pub fn check(&self, mixer: &Ae5Mixer, allow_high_gain: bool) -> Result<(), ProfileError> {
        self.validate_against(mixer, allow_high_gain).map(|_| ())
    }

    pub fn apply(
        &self,
        mixer: &Ae5Mixer,
        allow_high_gain: bool,
    ) -> Result<ApplyReport, ProfileError> {
        self.apply_to(mixer, allow_high_gain)
    }

    fn apply_to(
        &self,
        mixer: &impl ProfileMixer,
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
            controls_applied: effective_control_count(&self.controls),
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
        mixer: &impl ProfileMixer,
        allow_high_gain: bool,
    ) -> Result<BTreeMap<String, ProfileControl>, ProfileError> {
        self.validate_structure()?;
        let current_controls = mixer.snapshots()?;
        let mut before = BTreeMap::new();
        for (name, requested) in &self.controls {
            if capture_control_block_reason(name).is_some() {
                continue;
            }
            let current = current_controls
                .iter()
                .find(|control| control.name == *name)
                .cloned()
                .ok_or_else(|| ControlError::Missing(name.clone()))?;
            validate_control(name, requested, &current, allow_high_gain)?;
            before.insert(name.clone(), ProfileControl::from(current));
        }
        let mut projected = current_controls;
        project_controls(&mut projected, &self.controls);
        if self.controls.keys().any(|name| is_equalizer_band(name))
            && !profile_uses_factory_equalizer_preset(&self.controls)
            && let Some(reason) = equalizer_band_block_reason("EQ Band0", &projected)
        {
            return Err(ProfileError::Invalid(reason.to_owned()));
        }
        if self.controls.keys().any(|name| {
            matches!(
                name.as_str(),
                "Output Select" | "Surround Channel Config" | "Bass Redirection" | "FX: X-Bass"
            )
        }) && let Some(reason) = invalid_bass_state_reason(&projected)
        {
            return Err(ProfileError::Invalid(reason.to_owned()));
        }
        Ok(before)
    }
}

pub fn linux_driver_defaults() -> Result<Profile, ProfileError> {
    let mut controls = BTreeMap::from([
        (
            "AE-5: Headphone Gain".to_owned(),
            choice("Low (16-31  Ohms)"),
        ),
        ("AE-5: Sound Filter".to_owned(), choice("Slow Roll Off")),
        ("Enable InFX".to_owned(), capture_effect(false, None)),
        ("Enable OutFX".to_owned(), playback_effect(true, None)),
        (
            "FX: Crystalizer".to_owned(),
            playback_effect(true, Some(65)),
        ),
        (
            "FX: Dialog Plus".to_owned(),
            playback_effect(false, Some(50)),
        ),
        ("FX: Equalizer".to_owned(), playback_effect(false, None)),
        (EQUALIZER_PRESET_CONTROL.to_owned(), choice("Flat")),
        ("FX: Mic SVM".to_owned(), capture_effect(false, None)),
        ("FX: Noise Reduction".to_owned(), capture_effect(true, None)),
        (
            "FX: Smart Volume".to_owned(),
            playback_effect(true, Some(74)),
        ),
        ("FX: Smart Volume Setting".to_owned(), choice("Normal")),
        ("FX: Surround".to_owned(), playback_effect(true, Some(67))),
        ("FX: Voice Focus".to_owned(), capture_effect(true, None)),
        ("FX: X-Bass".to_owned(), playback_effect(true, Some(50))),
        ("FX: X-Bass Crossover".to_owned(), playback_level(8)),
        ("SVM Level".to_owned(), capture_level(74)),
        ("VoiceFX".to_owned(), choice("Neutral")),
        ("Wedge Angle".to_owned(), capture_level(30)),
    ]);
    for band in 0..10 {
        controls.insert(format!("EQ Band{band}"), playback_level(24));
    }
    Profile::new("AE-5 Linux driver defaults", controls)
}

pub fn linux_driver_defaults_for(
    current_controls: &[ControlSnapshot],
) -> Result<Profile, ProfileError> {
    let mut profile = linux_driver_defaults()?;
    let speakers_with_lfe = current_controls.iter().any(|control| {
        control.name == "Output Select" && control.selected.as_deref() == Some("Speakers")
    }) && current_controls.iter().any(|control| {
        control.name == "Surround Channel Config"
            && control
                .selected
                .as_deref()
                .is_some_and(|layout| layout.ends_with(".1"))
    });
    if speakers_with_lfe {
        profile
            .controls
            .get_mut("FX: X-Bass")
            .expect("the built-in baseline always contains X-Bass")
            .playback_switch = Some(false);
    }
    Ok(profile)
}

pub fn apply_linux_driver_defaults(
    mixer: &Ae5Mixer,
    backup_path: &Path,
) -> Result<ApplyReport, ProfileError> {
    let defaults = linux_driver_defaults_for(&mixer.snapshots().map_err(ControlError::from)?)?;
    apply_with_backup(
        &defaults,
        mixer,
        "Before AE-5 Linux driver defaults",
        backup_path,
        false,
    )
}

fn apply_with_backup(
    profile: &Profile,
    mixer: &impl ProfileMixer,
    backup_name: &str,
    backup_path: &Path,
    allow_high_gain: bool,
) -> Result<ApplyReport, ProfileError> {
    profile.validate_against(mixer, allow_high_gain)?;
    Profile::capture(backup_name, mixer.snapshots()?)?.save_new(backup_path)?;
    profile.apply_to(mixer, allow_high_gain)
}

fn choice(value: &str) -> ProfileControl {
    ProfileControl {
        choice: Some(value.to_owned()),
        ..ProfileControl::default()
    }
}

fn playback_effect(enabled: bool, level: Option<i64>) -> ProfileControl {
    ProfileControl {
        playback_switch: Some(enabled),
        playback_level: level,
        ..ProfileControl::default()
    }
}

fn capture_effect(enabled: bool, level: Option<i64>) -> ProfileControl {
    ProfileControl {
        capture_switch: Some(enabled),
        capture_level: level,
        ..ProfileControl::default()
    }
}

fn playback_level(value: i64) -> ProfileControl {
    ProfileControl {
        playback_level: Some(value),
        ..ProfileControl::default()
    }
}

fn capture_level(value: i64) -> ProfileControl {
    ProfileControl {
        capture_level: Some(value),
        ..ProfileControl::default()
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
    mixer: &impl ProfileMixer,
    controls: &BTreeMap<String, ProfileControl>,
    allow_high_gain: bool,
) -> Result<(), ControlError> {
    // Route changes are safe only after conflicting effects are off; target effects come last.
    apply_switches(mixer, controls, false)?;
    let skip_equalizer_bands = profile_uses_factory_equalizer_preset(controls);

    for (name, control) in controls {
        if capture_control_block_reason(name).is_some() {
            continue;
        }
        if let Some(choice) = &control.choice
            && (skip_equalizer_bands && name == EQUALIZER_PRESET_CONTROL
                || mixer
                    .snapshot(name)?
                    .selected
                    .as_ref()
                    .is_none_or(|value| !value.eq_ignore_ascii_case(choice)))
        {
            mixer.set_choice(name, choice, allow_high_gain)?;
        }
    }

    apply_switches(mixer, controls, true)?;

    for (name, control) in controls {
        if capture_control_block_reason(name).is_some()
            || skip_equalizer_bands && is_equalizer_band(name)
        {
            continue;
        }
        let current = mixer.snapshot(name)?;
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

fn profile_uses_factory_equalizer_preset(controls: &BTreeMap<String, ProfileControl>) -> bool {
    controls
        .get(EQUALIZER_PRESET_CONTROL)
        .and_then(|control| control.choice.as_deref())
        .is_some_and(|preset| !preset.eq_ignore_ascii_case("Flat"))
}

fn effective_control_count(controls: &BTreeMap<String, ProfileControl>) -> usize {
    let skip_equalizer_bands = profile_uses_factory_equalizer_preset(controls);
    controls
        .keys()
        .filter(|name| {
            capture_control_block_reason(name).is_none()
                && !(skip_equalizer_bands && is_equalizer_band(name))
        })
        .count()
}

fn apply_switches(
    mixer: &impl ProfileMixer,
    controls: &BTreeMap<String, ProfileControl>,
    enabled: bool,
) -> Result<(), ControlError> {
    for (name, control) in controls {
        if capture_control_block_reason(name).is_some() {
            continue;
        }
        let apply_playback = control.playback_switch == Some(enabled);
        let apply_capture = control.capture_switch == Some(enabled);
        if !apply_playback && !apply_capture {
            continue;
        }
        let current = mixer.snapshot(name)?;
        if apply_playback && current.playback_switch != Some(enabled) {
            mixer.set_playback_switch(name, enabled)?;
        }
        if apply_capture && current.capture_switch != Some(enabled) {
            mixer.set_capture_switch(name, enabled)?;
        }
    }
    Ok(())
}

fn project_controls(current: &mut [ControlSnapshot], requested: &BTreeMap<String, ProfileControl>) {
    for control in current {
        let Some(target) = requested.get(&control.name) else {
            continue;
        };
        if let Some(choice) = &target.choice {
            control.selected = Some(choice.clone());
        }
        if let Some(enabled) = target.playback_switch {
            control.playback_switch = Some(enabled);
        }
    }
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
    use std::cell::{Cell, RefCell};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);

    struct FakeMixer {
        surround: Cell<bool>,
        front: Cell<i64>,
        eq_preset: RefCell<String>,
        eq_band: Cell<i64>,
        writes: RefCell<Vec<String>>,
    }

    impl FakeMixer {
        fn new() -> Self {
            Self {
                surround: Cell::new(false),
                front: Cell::new(20),
                eq_preset: RefCell::new("Flat".to_owned()),
                eq_band: Cell::new(31),
                writes: RefCell::new(Vec::new()),
            }
        }

        fn record(&self, action: String) -> usize {
            let mut writes = self.writes.borrow_mut();
            writes.push(action);
            writes.len()
        }
    }

    impl ProfileMixer for FakeMixer {
        fn snapshots(&self) -> Result<Vec<ControlSnapshot>, ControlError> {
            Ok(vec![
                switch_snapshot("FX: Surround", self.surround.get()),
                level_snapshot("Front", self.front.get()),
                eq_preset_snapshot(&self.eq_preset.borrow()),
                eq_band_snapshot(self.eq_band.get()),
            ])
        }

        fn snapshot(&self, name: &str) -> Result<ControlSnapshot, ControlError> {
            match name {
                "FX: Surround" => Ok(switch_snapshot(name, self.surround.get())),
                "Front" => Ok(level_snapshot(name, self.front.get())),
                EQUALIZER_PRESET_CONTROL => Ok(eq_preset_snapshot(&self.eq_preset.borrow())),
                "EQ Band0" => Ok(eq_band_snapshot(self.eq_band.get())),
                _ => Err(ControlError::Missing(name.to_owned())),
            }
        }

        fn set_choice(
            &self,
            name: &str,
            choice: &str,
            _allow_high_gain: bool,
        ) -> Result<(), ControlError> {
            if name != EQUALIZER_PRESET_CONTROL {
                return Err(ControlError::Missing(name.to_owned()));
            }
            self.record(format!("{name}={choice}"));
            *self.eq_preset.borrow_mut() = choice.to_owned();
            Ok(())
        }

        fn set_playback_switch(&self, name: &str, enabled: bool) -> Result<(), ControlError> {
            if name != "FX: Surround" {
                return Err(ControlError::Missing(name.to_owned()));
            }
            self.record(format!("{name} playback switch={enabled}"));
            self.surround.set(enabled);
            Ok(())
        }

        fn set_capture_switch(&self, _name: &str, _enabled: bool) -> Result<(), ControlError> {
            unreachable!("this rollback scenario has no capture-switch writes")
        }

        fn set_playback_level(&self, name: &str, value: i64) -> Result<(), ControlError> {
            match name {
                "Front" => self.front.set(value),
                "EQ Band0" => self.eq_band.set(value),
                _ => return Err(ControlError::Missing(name.to_owned())),
            }
            let write_number = self.record(format!("{name} playback level={value}"));
            if name == "Front" && write_number == 2 {
                return Err(ControlError::Verification(
                    "injected write failure".to_owned(),
                ));
            }
            Ok(())
        }

        fn set_capture_level(&self, _name: &str, _value: i64) -> Result<(), ControlError> {
            unreachable!("this rollback scenario has no capture-level writes")
        }

        fn set_playback_channel_level(
            &self,
            _name: &str,
            _channel: &str,
            _value: i64,
        ) -> Result<(), ControlError> {
            unreachable!("this rollback scenario has no channel writes")
        }

        fn set_capture_channel_level(
            &self,
            _name: &str,
            _channel: &str,
            _value: i64,
        ) -> Result<(), ControlError> {
            unreachable!("this rollback scenario has no channel writes")
        }
    }

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

    fn switch_snapshot(name: &str, enabled: bool) -> ControlSnapshot {
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

    fn choice_snapshot(name: &str, selected: &str) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: Some(selected.to_owned()),
            choices: Vec::new(),
            playback_switch: None,
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        }
    }

    fn level_snapshot(name: &str, value: i64) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: None,
            capture_switch: None,
            playback_level: Some(crate::Level {
                value,
                min: 0,
                max: 100,
            }),
            capture_level: None,
            playback_channels: vec![crate::ChannelLevel {
                name: "Front Left".to_owned(),
                value,
            }],
            capture_channels: Vec::new(),
        }
    }

    fn eq_preset_snapshot(selected: &str) -> ControlSnapshot {
        let mut snapshot = choice_snapshot(EQUALIZER_PRESET_CONTROL, selected);
        snapshot.choices = vec!["Flat".to_owned(), "Acoustic".to_owned()];
        snapshot
    }

    fn eq_band_snapshot(value: i64) -> ControlSnapshot {
        let mut snapshot = level_snapshot("EQ Band0", value);
        snapshot.playback_level.as_mut().unwrap().min = 0;
        snapshot.playback_level.as_mut().unwrap().max = 48;
        snapshot
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
    fn linux_driver_defaults_match_ca0132_and_preserve_user_routing() {
        let profile = linux_driver_defaults().unwrap();

        assert_eq!(profile.controls.len(), 29);
        assert_eq!(
            profile.controls["AE-5: Headphone Gain"].choice.as_deref(),
            Some("Low (16-31  Ohms)")
        );
        assert_eq!(
            profile.controls["AE-5: Sound Filter"].choice.as_deref(),
            Some("Slow Roll Off")
        );
        assert_eq!(profile.controls["Enable OutFX"].playback_switch, Some(true));
        assert_eq!(profile.controls["Enable InFX"].capture_switch, Some(false));
        for (name, enabled, level) in [
            ("FX: Surround", true, 67),
            ("FX: Crystalizer", true, 65),
            ("FX: Dialog Plus", false, 50),
            ("FX: Smart Volume", true, 74),
            ("FX: X-Bass", true, 50),
        ] {
            assert_eq!(
                profile.controls[name].playback_switch,
                Some(enabled),
                "{name}"
            );
            assert_eq!(profile.controls[name].playback_level, Some(level), "{name}");
        }
        assert_eq!(
            profile.controls["FX: Equalizer"].playback_switch,
            Some(false)
        );
        assert_eq!(
            profile.controls[EQUALIZER_PRESET_CONTROL].choice.as_deref(),
            Some("Flat")
        );
        for band in 0..10 {
            assert_eq!(
                profile.controls[&format!("EQ Band{band}")].playback_level,
                Some(24)
            );
        }
        assert_eq!(
            profile.controls["FX: Smart Volume Setting"]
                .choice
                .as_deref(),
            Some("Normal")
        );
        assert_eq!(
            profile.controls["FX: X-Bass Crossover"].playback_level,
            Some(8)
        );
        for (name, enabled) in [
            ("FX: Voice Focus", true),
            ("FX: Mic SVM", false),
            ("FX: Noise Reduction", true),
        ] {
            assert_eq!(
                profile.controls[name].capture_switch,
                Some(enabled),
                "{name}"
            );
        }
        assert_eq!(profile.controls["SVM Level"].capture_level, Some(74));
        assert_eq!(profile.controls["Wedge Angle"].capture_level, Some(30));
        assert_eq!(
            profile.controls["VoiceFX"].choice.as_deref(),
            Some("Neutral")
        );

        for preserved in [
            "Output Select",
            "HP/Speaker Auto Detect",
            "Input Source",
            "Mic Boost",
            "Surround Channel Config",
            "Full-Range Front Speakers",
            "Full-Range Rear Speakers",
            "Bass Redirection",
            "Bass Redirection Crossover",
            "Master",
            "Front",
            "Capture",
            "What U Hear",
        ] {
            assert!(!profile.controls.contains_key(preserved), "{preserved}");
        }
    }

    #[test]
    fn linux_driver_defaults_disable_xbass_for_preserved_lfe_layouts() {
        let lfe_speakers = [
            choice_snapshot("Output Select", "Speakers"),
            choice_snapshot("Surround Channel Config", "5.1"),
        ];
        let profile = linux_driver_defaults_for(&lfe_speakers).unwrap();

        assert_eq!(profile.controls["FX: X-Bass"].playback_switch, Some(false));
        assert!(!profile.controls.contains_key("Output Select"));
        assert!(!profile.controls.contains_key("Surround Channel Config"));

        let headphones = [
            choice_snapshot("Output Select", "Headphone"),
            choice_snapshot("Surround Channel Config", "5.1"),
        ];
        assert_eq!(
            linux_driver_defaults_for(&headphones).unwrap().controls["FX: X-Bass"].playback_switch,
            Some(true)
        );
    }

    #[test]
    fn backup_failure_prevents_profile_writes() {
        let mixer = FakeMixer::new();
        let profile = Profile::new(
            "Reset target",
            BTreeMap::from([(
                "FX: Surround".to_owned(),
                ProfileControl {
                    playback_switch: Some(true),
                    ..ProfileControl::default()
                },
            )]),
        )
        .unwrap();
        let backup = test_path();
        fs::write(&backup, b"do not overwrite").unwrap();

        assert!(matches!(
            apply_with_backup(&profile, &mixer, "Before reset", &backup, false),
            Err(ProfileError::Io(error)) if error.kind() == io::ErrorKind::AlreadyExists
        ));
        assert!(!mixer.surround.get());
        assert!(mixer.writes.borrow().is_empty());
        assert_eq!(fs::read(&backup).unwrap(), b"do not overwrite");
        fs::remove_file(backup).unwrap();
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
    fn omits_ineffective_what_u_hear_from_new_profiles() {
        let what_u_hear = ControlSnapshot {
            name: "What U Hear".to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: None,
            capture_switch: Some(true),
            playback_level: None,
            capture_level: Some(crate::Level {
                value: 90,
                min: 0,
                max: 99,
            }),
            playback_channels: Vec::new(),
            capture_channels: vec![
                crate::ChannelLevel {
                    name: "Front Left".to_owned(),
                    value: 90,
                },
                crate::ChannelLevel {
                    name: "Front Right".to_owned(),
                    value: 90,
                },
            ],
        };

        let profile =
            Profile::capture("Compatible", vec![level_snapshot("Front", 90), what_u_hear]).unwrap();

        assert!(profile.controls.contains_key("Front"));
        assert!(!profile.controls.contains_key("What U Hear"));
    }

    #[test]
    fn legacy_what_u_hear_entries_are_ignored_when_the_kernel_hides_them() {
        let mixer = FakeMixer::new();
        let profile = Profile::new(
            "Legacy loopback",
            BTreeMap::from([(
                "What U Hear".to_owned(),
                ProfileControl {
                    capture_switch: Some(false),
                    capture_level: Some(0),
                    capture_channels: BTreeMap::from([
                        ("Front Left".to_owned(), 0),
                        ("Front Right".to_owned(), 0),
                    ]),
                    ..ProfileControl::default()
                },
            )]),
        )
        .unwrap();

        let report = profile.apply_to(&mixer, false).unwrap();

        assert_eq!(report.controls_applied, 0);
        assert!(mixer.writes.borrow().is_empty());
    }

    #[test]
    fn omits_stale_eq_bands_when_capturing_a_factory_preset() {
        let mixer = FakeMixer::new();
        *mixer.eq_preset.borrow_mut() = "Acoustic".to_owned();

        let profile = Profile::capture("Acoustic", mixer.snapshots().unwrap()).unwrap();

        assert_eq!(
            profile.controls[EQUALIZER_PRESET_CONTROL].choice.as_deref(),
            Some("Acoustic")
        );
        assert!(!profile.controls.contains_key("EQ Band0"));
    }

    #[test]
    fn legacy_factory_preset_profiles_ignore_stale_eq_bands() {
        let mixer = FakeMixer::new();
        *mixer.eq_preset.borrow_mut() = "Acoustic".to_owned();
        let profile = Profile::new(
            "Acoustic",
            BTreeMap::from([
                (
                    EQUALIZER_PRESET_CONTROL.to_owned(),
                    ProfileControl {
                        choice: Some("Acoustic".to_owned()),
                        ..ProfileControl::default()
                    },
                ),
                (
                    "EQ Band0".to_owned(),
                    ProfileControl {
                        playback_level: Some(24),
                        ..ProfileControl::default()
                    },
                ),
            ]),
        )
        .unwrap();

        let report = profile.apply_to(&mixer, false).unwrap();

        assert_eq!(report.controls_applied, 1);
        assert_eq!(*mixer.writes.borrow(), ["FX: Equalizer Preset=Acoustic"]);
        assert_eq!(mixer.eq_band.get(), 31);
    }

    #[test]
    fn rejects_band_only_profiles_while_a_factory_preset_is_active() {
        let mixer = FakeMixer::new();
        *mixer.eq_preset.borrow_mut() = "Acoustic".to_owned();
        let profile = Profile::new(
            "Unsafe partial EQ",
            BTreeMap::from([(
                "EQ Band0".to_owned(),
                ProfileControl {
                    playback_level: Some(24),
                    ..ProfileControl::default()
                },
            )]),
        )
        .unwrap();

        assert!(matches!(
            profile.apply_to(&mixer, false),
            Err(ProfileError::Invalid(message))
                if message.contains("Select Flat before editing custom bands")
        ));
        assert!(mixer.writes.borrow().is_empty());
    }

    #[test]
    fn projects_a_safe_final_bass_state_before_profile_writes() {
        let current = vec![
            switch_snapshot("FX: X-Bass", true),
            switch_snapshot("Bass Redirection", false),
            choice_snapshot("Surround Channel Config", "2.0"),
            choice_snapshot("Output Select", "Headphone"),
        ];
        let route = BTreeMap::from([
            (
                "Output Select".to_owned(),
                ProfileControl {
                    choice: Some("Speakers".to_owned()),
                    ..ProfileControl::default()
                },
            ),
            (
                "Surround Channel Config".to_owned(),
                ProfileControl {
                    choice: Some("5.1".to_owned()),
                    ..ProfileControl::default()
                },
            ),
        ]);

        let mut projected = current.clone();
        project_controls(&mut projected, &route);
        assert_eq!(
            invalid_bass_state_reason(&projected),
            Some("X-Bass is unavailable for speaker layouts with an LFE channel.")
        );

        let mut safe_route = route;
        safe_route.insert(
            "FX: X-Bass".to_owned(),
            ProfileControl {
                playback_switch: Some(false),
                ..ProfileControl::default()
            },
        );
        safe_route.insert(
            "Bass Redirection".to_owned(),
            ProfileControl {
                playback_switch: Some(true),
                ..ProfileControl::default()
            },
        );
        let mut projected = current;
        project_controls(&mut projected, &safe_route);
        assert_eq!(invalid_bass_state_reason(&projected), None);
    }

    #[test]
    fn failed_profile_write_rolls_back_prior_changes_in_order() {
        let mixer = FakeMixer::new();
        let initial = mixer.snapshots().unwrap();
        let profile = Profile::new(
            "Rollback",
            BTreeMap::from([
                (
                    "FX: Surround".to_owned(),
                    ProfileControl {
                        playback_switch: Some(true),
                        ..ProfileControl::default()
                    },
                ),
                (
                    "Front".to_owned(),
                    ProfileControl {
                        playback_level: Some(75),
                        ..ProfileControl::default()
                    },
                ),
            ]),
        )
        .unwrap();

        let error = profile.apply_to(&mixer, false).unwrap_err();
        match error {
            ProfileError::Apply {
                failure,
                rollback_failure,
            } => {
                assert_eq!(failure, "injected write failure");
                assert_eq!(rollback_failure, None);
            }
            other => panic!("expected an apply failure, got {other}"),
        }
        assert_eq!(mixer.snapshots().unwrap(), initial);
        assert_eq!(
            *mixer.writes.borrow(),
            [
                "FX: Surround playback switch=true",
                "Front playback level=75",
                "FX: Surround playback switch=false",
                "Front playback level=20",
            ]
        );
    }

    #[test]
    fn omits_a_driver_level_outside_its_declared_range() {
        let control = ControlSnapshot {
            name: "Wedge Angle".to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: None,
            capture_switch: None,
            playback_level: None,
            capture_level: Some(crate::Level {
                value: 10,
                min: 20,
                max: 180,
            }),
            playback_channels: Vec::new(),
            capture_channels: vec![crate::ChannelLevel {
                name: "Mono".to_owned(),
                value: 10,
            }],
        };

        assert!(ProfileControl::from(control).is_empty());
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
