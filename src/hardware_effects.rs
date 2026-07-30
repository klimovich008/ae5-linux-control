use std::collections::BTreeMap;
use std::env;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::pipewire::suspend_ae5_output;
use crate::{
    Ae5Mixer, ControlError, ControlSnapshot, DIRECT_MODE_CONTROL, EffectsProfileEntry,
    HARDWARE_OUTFX_CONTROL, ProfileControl, hardware_outfx_lab_active,
};

const CONFIG_FILE: &str = "hardware-effects.json";
const CONFIG_SCHEMA_VERSION: u32 = 1;
type ProfileAvailability = fn(&EffectsProfileEntry) -> bool;
const CHILD_CONTROLS: [(&str, ProfileAvailability); 5] = [
    ("FX: Surround", |profile| profile.surround_available),
    ("FX: Crystalizer", |profile| profile.crystalizer_available),
    ("FX: X-Bass", |profile| profile.bass_available),
    ("FX: Smart Volume", |profile| profile.smart_volume_available),
    ("FX: Dialog Plus", |profile| profile.dialog_available),
];
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardwareEffectsConfig {
    pub path: PathBuf,
    pub profile: Option<EffectsProfileEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HardwareEffectsChange {
    pub config: HardwareEffectsConfig,
    pub changed: bool,
}

#[derive(Debug)]
pub enum HardwareEffectsError {
    Control(ControlError),
    Io(io::Error),
    Invalid(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredHardwareEffects {
    schema_version: u32,
    profile: EffectsProfileEntry,
}

pub fn hardware_effects_config() -> Result<HardwareEffectsConfig, HardwareEffectsError> {
    hardware_effects_config_at(&hardware_effects_path()?)
}

pub fn apply_hardware_effects(
    card_index: i32,
    profile: &EffectsProfileEntry,
) -> Result<HardwareEffectsChange, HardwareEffectsError> {
    profile.validate().map_err(HardwareEffectsError::Invalid)?;
    if !profile.outfx_enabled {
        return Err(HardwareEffectsError::Invalid(
            "Enable the Effects master before applying this hardware profile.".to_owned(),
        ));
    }
    require_hardware_effects_gate()?;

    let path = hardware_effects_path()?;
    let previous_config = hardware_effects_config_at(&path)?;
    let mixer = Ae5Mixer::open(card_index).map_err(ControlError::from)?;
    let snapshots = mixer.snapshots().map_err(ControlError::from)?;
    require_direct_mode_off(&snapshots)?;
    let target = profile_targets(profile);
    validate_targets(&target, &snapshots)?;
    let previous = capture_targets(&target, &snapshots)?;

    let suspended = suspend_ae5_output(card_index)?;
    if let Err(error) = apply_targets(&mixer, &target) {
        let rollback = apply_targets(&mixer, &previous);
        let resume = suspended.resume();
        return Err(transaction_failure(error, rollback, resume));
    }
    let verified = mixer
        .snapshots()
        .map_err(ControlError::from)
        .and_then(|actual| verify_targets(&target, &actual).map_err(ControlError::Verification));
    if let Err(error) = verified {
        let rollback = apply_targets(&mixer, &previous);
        let resume = suspended.resume();
        return Err(transaction_failure(error, rollback, resume));
    }
    if let Err(error) = suspended.resume() {
        let rollback = apply_targets_with_output_pause(card_index, &mixer, &previous);
        return Err(transaction_failure(
            ControlError::from(error),
            rollback,
            Ok(()),
        ));
    }

    match save_hardware_effects_config_at(&path, profile) {
        Ok(change) => Ok(change),
        Err(error) => {
            let rollback = apply_targets_with_output_pause(card_index, &mixer, &previous);
            let config = restore_hardware_effects_config_at(&path, &previous_config);
            Err(HardwareEffectsError::Invalid(format!(
                "Hardware Effects applied but their managed state could not be saved: {error}; \
                 hardware rollback: {}; configuration rollback: {}",
                result_detail(rollback),
                result_detail(
                    config
                        .map(|_| ())
                        .map_err(|error| { ControlError::Verification(error.to_string()) })
                )
            )))
        }
    }
}

pub fn disable_hardware_effects(
    card_index: i32,
) -> Result<HardwareEffectsChange, HardwareEffectsError> {
    require_hardware_effects_gate()?;
    let mixer = Ae5Mixer::open(card_index).map_err(ControlError::from)?;
    let current = mixer.snapshot(HARDWARE_OUTFX_CONTROL)?;
    let previous = current.playback_switch.ok_or_else(|| {
        HardwareEffectsError::Invalid(format!("{HARDWARE_OUTFX_CONTROL} has no playback switch"))
    })?;
    let previous_target = master_target(previous);
    let disabled_target = master_target(false);
    let suspended = suspend_ae5_output(card_index)?;
    if let Err(error) = mixer.set_playback_switch(HARDWARE_OUTFX_CONTROL, false) {
        let resume = suspended.resume();
        return Err(transaction_failure(error, Ok(()), resume));
    }
    let verified = mixer
        .snapshots()
        .map_err(ControlError::from)
        .and_then(|actual| {
            verify_targets(&disabled_target, &actual).map_err(ControlError::Verification)
        });
    if let Err(error) = verified {
        let rollback = apply_targets(&mixer, &previous_target);
        let resume = suspended.resume();
        return Err(transaction_failure(error, rollback, resume));
    }
    if let Err(error) = suspended.resume() {
        let rollback = apply_targets_with_output_pause(card_index, &mixer, &previous_target);
        return Err(transaction_failure(
            ControlError::from(error),
            rollback,
            Ok(()),
        ));
    }
    let actual = mixer.snapshots().map_err(ControlError::from)?;
    if let Err(error) = verify_targets(&disabled_target, &actual) {
        let rollback = apply_targets_with_output_pause(card_index, &mixer, &previous_target);
        return Err(transaction_failure(
            ControlError::Verification(error),
            rollback,
            Ok(()),
        ));
    }
    let config = hardware_effects_config()?;
    Ok(HardwareEffectsChange {
        config,
        changed: previous,
    })
}

pub fn hardware_effects_profile_matches(
    profile: &EffectsProfileEntry,
    controls: &[ControlSnapshot],
    include_master: bool,
) -> bool {
    let targets = profile_targets(profile);
    let targets = if include_master {
        targets
    } else {
        targets
            .into_iter()
            .filter(|(name, _)| name != HARDWARE_OUTFX_CONTROL)
            .collect()
    };
    verify_targets(&targets, controls).is_ok()
}

pub fn require_hardware_effects_gate() -> Result<(), HardwareEffectsError> {
    if hardware_outfx_lab_active() {
        Ok(())
    } else {
        Err(HardwareEffectsError::Invalid(
            "Hardware OutFX needs the exact AE-5 OutFX lab kernel, its boot-scoped module gate, \
             an outfx-lab build, and explicit process confirmation."
                .to_owned(),
        ))
    }
}

fn profile_targets(profile: &EffectsProfileEntry) -> BTreeMap<String, ProfileControl> {
    let mut controls = BTreeMap::from([(
        HARDWARE_OUTFX_CONTROL.to_owned(),
        ProfileControl {
            playback_switch: Some(profile.outfx_enabled),
            ..ProfileControl::default()
        },
    )]);
    for (name, available) in CHILD_CONTROLS {
        if !available(profile) {
            continue;
        }
        let (enabled, level) = match name {
            "FX: Surround" => (profile.surround_enabled, profile.surround_level),
            "FX: Crystalizer" => (profile.crystalizer_enabled, profile.crystalizer_level),
            "FX: X-Bass" => (profile.bass_enabled, profile.bass_level),
            "FX: Smart Volume" => (profile.smart_volume_enabled, profile.smart_volume_level),
            "FX: Dialog Plus" => (profile.dialog_enabled, profile.dialog_level),
            _ => unreachable!("child control list is exhaustive"),
        };
        controls.insert(
            name.to_owned(),
            ProfileControl {
                playback_switch: Some(enabled),
                playback_level: Some(i64::from(level)),
                ..ProfileControl::default()
            },
        );
    }
    if profile.smart_volume_available {
        controls.insert(
            "FX: Smart Volume Setting".to_owned(),
            ProfileControl {
                choice: Some(profile.smart_volume_mode.clone()),
                ..ProfileControl::default()
            },
        );
    }
    controls
}

fn master_target(enabled: bool) -> BTreeMap<String, ProfileControl> {
    BTreeMap::from([(
        HARDWARE_OUTFX_CONTROL.to_owned(),
        ProfileControl {
            playback_switch: Some(enabled),
            ..ProfileControl::default()
        },
    )])
}

fn validate_targets(
    targets: &BTreeMap<String, ProfileControl>,
    controls: &[ControlSnapshot],
) -> Result<(), HardwareEffectsError> {
    for (name, target) in targets {
        let current = controls
            .iter()
            .find(|control| control.name == *name)
            .ok_or_else(|| HardwareEffectsError::Invalid(format!("{name} is unavailable")))?;
        if target.playback_switch.is_some() && current.playback_switch.is_none() {
            return Err(HardwareEffectsError::Invalid(format!(
                "{name} has no playback switch"
            )));
        }
        if let Some(value) = target.playback_level {
            let level = current.playback_level.as_ref().ok_or_else(|| {
                HardwareEffectsError::Invalid(format!("{name} has no playback level"))
            })?;
            if !(level.min..=level.max).contains(&value) {
                return Err(HardwareEffectsError::Invalid(format!(
                    "{name} level {value} is outside {}..{}",
                    level.min, level.max
                )));
            }
        }
        if let Some(choice) = &target.choice
            && current
                .choices
                .iter()
                .all(|candidate| !candidate.eq_ignore_ascii_case(choice))
        {
            return Err(HardwareEffectsError::Invalid(format!(
                "{name} does not support {choice}"
            )));
        }
    }
    Ok(())
}

fn capture_targets(
    targets: &BTreeMap<String, ProfileControl>,
    controls: &[ControlSnapshot],
) -> Result<BTreeMap<String, ProfileControl>, HardwareEffectsError> {
    targets
        .iter()
        .map(|(name, target)| {
            let current = controls
                .iter()
                .find(|control| control.name == *name)
                .ok_or_else(|| HardwareEffectsError::Invalid(format!("{name} is unavailable")))?;
            Ok((
                name.clone(),
                ProfileControl {
                    choice: target.choice.as_ref().and(current.selected.clone()),
                    playback_switch: target.playback_switch.and(current.playback_switch),
                    playback_level: target
                        .playback_level
                        .and(current.playback_level.as_ref().map(|level| level.value)),
                    ..ProfileControl::default()
                },
            ))
        })
        .collect()
}

fn apply_targets(
    mixer: &Ae5Mixer,
    targets: &BTreeMap<String, ProfileControl>,
) -> Result<(), ControlError> {
    if targets.contains_key(HARDWARE_OUTFX_CONTROL)
        && mixer.snapshot(HARDWARE_OUTFX_CONTROL)?.playback_switch != Some(false)
    {
        mixer.set_playback_switch(HARDWARE_OUTFX_CONTROL, false)?;
    }
    for (name, target) in targets {
        if name.starts_with("FX: ")
            && name != "FX: Smart Volume Setting"
            && target.playback_switch.is_some()
            && mixer.snapshot(name)?.playback_switch != Some(false)
        {
            mixer.set_playback_switch(name, false)?;
        }
    }
    for (name, target) in targets {
        if let Some(choice) = &target.choice
            && mixer
                .snapshot(name)?
                .selected
                .as_deref()
                .is_none_or(|current| !current.eq_ignore_ascii_case(choice))
        {
            mixer.set_choice(name, choice)?;
        }
    }
    for (name, target) in targets {
        if let Some(level) = target.playback_level
            && mixer
                .snapshot(name)?
                .playback_level
                .as_ref()
                .map(|current| current.value)
                != Some(level)
        {
            mixer.set_playback_level(name, level)?;
        }
    }
    for (name, target) in targets {
        if name == HARDWARE_OUTFX_CONTROL {
            continue;
        }
        if let Some(enabled) = target.playback_switch
            && mixer.snapshot(name)?.playback_switch != Some(enabled)
        {
            mixer.set_playback_switch(name, enabled)?;
        }
    }
    if let Some(enabled) = targets
        .get(HARDWARE_OUTFX_CONTROL)
        .and_then(|target| target.playback_switch)
        && mixer.snapshot(HARDWARE_OUTFX_CONTROL)?.playback_switch != Some(enabled)
    {
        mixer.set_playback_switch(HARDWARE_OUTFX_CONTROL, enabled)?;
    }
    Ok(())
}

fn apply_targets_with_output_pause(
    card_index: i32,
    mixer: &Ae5Mixer,
    targets: &BTreeMap<String, ProfileControl>,
) -> Result<(), ControlError> {
    let suspended = suspend_ae5_output(card_index).map_err(ControlError::from)?;
    let applied = apply_targets(mixer, targets).and_then(|_| {
        let actual = mixer.snapshots().map_err(ControlError::from)?;
        verify_targets(targets, &actual).map_err(ControlError::Verification)
    });
    let resume = suspended.resume().map_err(ControlError::from);
    match (applied, resume) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(resume_error)) => Err(ControlError::Verification(format!(
            "{error}; output resume also failed: {resume_error}"
        ))),
    }
}

fn verify_targets(
    targets: &BTreeMap<String, ProfileControl>,
    controls: &[ControlSnapshot],
) -> Result<(), String> {
    for (name, target) in targets {
        let current = controls
            .iter()
            .find(|control| control.name == *name)
            .ok_or_else(|| format!("{name} is unavailable"))?;
        if let Some(expected) = target.playback_switch
            && current.playback_switch != Some(expected)
        {
            return Err(format!(
                "{name} playback switch read back as {:?}, expected {expected}",
                current.playback_switch
            ));
        }
        if let Some(expected) = target.playback_level
            && current.playback_level.as_ref().map(|level| level.value) != Some(expected)
        {
            return Err(format!(
                "{name} playback level read back as {:?}, expected {expected}",
                current.playback_level.as_ref().map(|level| level.value)
            ));
        }
        if let Some(expected) = &target.choice
            && current
                .selected
                .as_deref()
                .is_none_or(|actual| !actual.eq_ignore_ascii_case(expected))
        {
            return Err(format!(
                "{name} read back as {:?}, expected {expected}",
                current.selected
            ));
        }
    }
    Ok(())
}

fn require_direct_mode_off(controls: &[ControlSnapshot]) -> Result<(), HardwareEffectsError> {
    if controls
        .iter()
        .find(|control| control.name == DIRECT_MODE_CONTROL)
        .and_then(|control| control.playback_switch)
        == Some(true)
    {
        Err(HardwareEffectsError::Invalid(
            "Turn Direct Mode off before applying hardware Effects.".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn transaction_failure(
    failure: ControlError,
    rollback: Result<(), ControlError>,
    resume: io::Result<()>,
) -> HardwareEffectsError {
    HardwareEffectsError::Invalid(format!(
        "Hardware Effects transaction failed: {failure}; rollback: {}; output resume: {}",
        result_detail(rollback),
        resume
            .err()
            .map(|error| error.to_string())
            .unwrap_or_else(|| "verified".to_owned())
    ))
}

fn result_detail(result: Result<(), ControlError>) -> String {
    result
        .err()
        .map(|error| error.to_string())
        .unwrap_or_else(|| "verified".to_owned())
}

fn hardware_effects_config_at(path: &Path) -> Result<HardwareEffectsConfig, HardwareEffectsError> {
    match fs::read(path) {
        Ok(contents) => {
            let stored: StoredHardwareEffects =
                serde_json::from_slice(&contents).map_err(|_| foreign_config(path))?;
            if stored.schema_version != CONFIG_SCHEMA_VERSION {
                return Err(foreign_config(path));
            }
            stored
                .profile
                .validate()
                .map_err(HardwareEffectsError::Invalid)?;
            Ok(HardwareEffectsConfig {
                path: path.to_owned(),
                profile: Some(stored.profile),
            })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(HardwareEffectsConfig {
            path: path.to_owned(),
            profile: None,
        }),
        Err(error) => Err(error.into()),
    }
}

fn save_hardware_effects_config_at(
    path: &Path,
    profile: &EffectsProfileEntry,
) -> Result<HardwareEffectsChange, HardwareEffectsError> {
    let current = hardware_effects_config_at(path)?;
    if current.profile.as_ref() == Some(profile) {
        return Ok(HardwareEffectsChange {
            config: current,
            changed: false,
        });
    }
    let stored = StoredHardwareEffects {
        schema_version: CONFIG_SCHEMA_VERSION,
        profile: profile.clone(),
    };
    let mut contents = serde_json::to_vec_pretty(&stored)
        .map_err(|error| HardwareEffectsError::Invalid(error.to_string()))?;
    contents.push(b'\n');
    replace_file(path, &contents)?;
    Ok(HardwareEffectsChange {
        config: hardware_effects_config_at(path)?,
        changed: true,
    })
}

fn restore_hardware_effects_config_at(
    path: &Path,
    config: &HardwareEffectsConfig,
) -> Result<HardwareEffectsChange, HardwareEffectsError> {
    if config.path != path {
        return Err(HardwareEffectsError::Invalid(format!(
            "cannot restore hardware Effects state from {} into {}",
            config.path.display(),
            path.display()
        )));
    }
    match config.profile.as_ref() {
        Some(profile) => save_hardware_effects_config_at(path, profile),
        None => match fs::remove_file(path) {
            Ok(()) => Ok(HardwareEffectsChange {
                config: hardware_effects_config_at(path)?,
                changed: true,
            }),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(HardwareEffectsChange {
                config: config.clone(),
                changed: false,
            }),
            Err(error) => Err(error.into()),
        },
    }
}

fn hardware_effects_path() -> Result<PathBuf, HardwareEffectsError> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return Ok(path.join("ae5-control").join(CONFIG_FILE));
    }
    env::var_os("HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(".config/ae5-control").join(CONFIG_FILE))
        .ok_or_else(|| {
            HardwareEffectsError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "neither XDG_CONFIG_HOME nor HOME is available",
            ))
        })
}

fn replace_file(path: &Path, contents: &[u8]) -> Result<(), HardwareEffectsError> {
    let parent = path.parent().ok_or_else(|| {
        HardwareEffectsError::Invalid(format!("{} has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    let temporary = parent.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("hardware-effects"),
        std::process::id(),
        sequence
    ));
    let result = (|| -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(contents)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(Into::into)
}

fn foreign_config(path: &Path) -> HardwareEffectsError {
    HardwareEffectsError::Io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "{} exists but is not managed by AE-5 Control",
            path.display()
        ),
    ))
}

impl fmt::Display for HardwareEffectsError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Control(error) => error.fmt(output),
            Self::Io(error) => error.fmt(output),
            Self::Invalid(message) => output.write_str(message),
        }
    }
}

impl std::error::Error for HardwareEffectsError {}

impl From<ControlError> for HardwareEffectsError {
    fn from(error: ControlError) -> Self {
        Self::Control(error)
    }
}

impl From<io::Error> for HardwareEffectsError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChannelLevel, Level};

    #[test]
    fn profile_targets_keep_master_last_and_children_independent() {
        let targets = profile_targets(&profile());

        assert_eq!(targets[HARDWARE_OUTFX_CONTROL].playback_switch, Some(true));
        assert_eq!(targets["FX: Surround"].playback_level, Some(35));
        assert_eq!(targets["FX: Crystalizer"].playback_switch, Some(true));
        assert_eq!(
            targets["FX: Smart Volume Setting"].choice.as_deref(),
            Some("Normal")
        );
    }

    #[test]
    fn matching_detects_external_hardware_changes() {
        let profile = profile();
        let controls = snapshots(&profile);

        assert!(hardware_effects_profile_matches(&profile, &controls, true));
        let mut changed = controls.clone();
        changed
            .iter_mut()
            .find(|control| control.name == "FX: X-Bass")
            .unwrap()
            .playback_level
            .as_mut()
            .unwrap()
            .value += 1;
        assert!(!hardware_effects_profile_matches(&profile, &changed, true));
    }

    #[test]
    fn matching_can_ignore_only_the_global_bypass() {
        let profile = profile();
        let mut controls = snapshots(&profile);
        controls
            .iter_mut()
            .find(|control| control.name == HARDWARE_OUTFX_CONTROL)
            .unwrap()
            .playback_switch = Some(false);

        assert!(!hardware_effects_profile_matches(&profile, &controls, true));
        assert!(hardware_effects_profile_matches(&profile, &controls, false));
    }

    fn profile() -> EffectsProfileEntry {
        EffectsProfileEntry {
            id: "effects:test".to_owned(),
            name: "Test".to_owned(),
            source: "Test".to_owned(),
            read_only: false,
            outfx_enabled: true,
            surround_available: true,
            surround_enabled: true,
            surround_level: 35,
            crystalizer_available: true,
            crystalizer_enabled: true,
            crystalizer_level: 50,
            bass_available: true,
            bass_enabled: true,
            bass_level: 53,
            smart_volume_available: true,
            smart_volume_enabled: true,
            smart_volume_level: 15,
            smart_volume_mode: "Normal".to_owned(),
            dialog_available: true,
            dialog_enabled: true,
            dialog_level: 20,
        }
    }

    fn snapshots(profile: &EffectsProfileEntry) -> Vec<ControlSnapshot> {
        profile_targets(profile)
            .into_iter()
            .map(|(name, target)| ControlSnapshot {
                name,
                selected: target.choice.clone(),
                choices: target.choice.into_iter().collect(),
                playback_switch: target.playback_switch,
                capture_switch: None,
                playback_level: target.playback_level.map(|value| Level {
                    value,
                    min: 0,
                    max: 100,
                    db: None,
                }),
                capture_level: None,
                playback_channels: target
                    .playback_level
                    .map(|value| {
                        vec![ChannelLevel {
                            name: "Front Left".to_owned(),
                            value,
                        }]
                    })
                    .unwrap_or_default(),
                capture_channels: Vec::new(),
            })
            .collect()
    }
}
