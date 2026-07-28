use crate::sbcommand::map_lfe_bass_controls;
use crate::{Profile, ProfileError};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

const COMMAND_DEFAULTS: &str = include_str!("../data/sbcommand-3.5.10-default-profiles.json");
pub const COMMAND_DEFAULT_PROFILE_COUNT: usize = 33;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
pub struct BuiltinProfile {
    pub source_id: String,
    pub name: String,
    speaker: Profile,
    headphone: Profile,
}

impl BuiltinProfile {
    pub fn profile_for(
        &self,
        output_choice: &str,
        speaker_layout: Option<&str>,
    ) -> Result<Profile, ProfileError> {
        let mut profile = match output_choice {
            "Headphone" => self.headphone.clone(),
            "Speakers" => self.speaker.clone(),
            value => {
                return Err(ProfileError::Invalid(format!(
                    "cannot choose a built-in profile variant for output '{value}'"
                )));
            }
        };
        if output_choice == "Speakers" {
            map_lfe_bass_controls(&mut profile.controls, speaker_layout);
        }
        Ok(profile)
    }
}

pub fn builtin_profiles() -> Result<&'static [BuiltinProfile], &'static str> {
    static PROFILES: OnceLock<Result<Vec<BuiltinProfile>, String>> = OnceLock::new();
    match PROFILES.get_or_init(load_builtin_profiles) {
        Ok(profiles) => Ok(profiles),
        Err(error) => Err(error),
    }
}

fn load_builtin_profiles() -> Result<Vec<BuiltinProfile>, String> {
    let profiles: Vec<BuiltinProfile> =
        serde_json::from_str(COMMAND_DEFAULTS).map_err(|error| error.to_string())?;
    if profiles.len() != COMMAND_DEFAULT_PROFILE_COUNT {
        return Err(format!(
            "embedded Command profile count is {}, expected {COMMAND_DEFAULT_PROFILE_COUNT}",
            profiles.len()
        ));
    }
    let mut names = BTreeSet::new();
    let mut ids = BTreeSet::new();
    for preset in &profiles {
        if !names.insert(preset.name.to_lowercase()) {
            return Err(format!("duplicate embedded profile name '{}'", preset.name));
        }
        if !ids.insert(&preset.source_id) {
            return Err(format!(
                "duplicate embedded profile source id '{}'",
                preset.source_id
            ));
        }
        for profile in [&preset.speaker, &preset.headphone] {
            profile
                .validate_structure()
                .map_err(|error| error.to_string())?;
            if profile.name != preset.name {
                return Err(format!(
                    "embedded variant '{}' does not match profile '{}'",
                    profile.name, preset.name
                ));
            }
        }
    }
    Ok(profiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeds_every_command_default_and_selects_the_live_output_variant() {
        let profiles = builtin_profiles().unwrap();
        assert_eq!(profiles.len(), COMMAND_DEFAULT_PROFILE_COUNT);
        assert!(profiles.iter().any(|profile| profile.name == "Gaming"));
        assert!(
            profiles
                .iter()
                .any(|profile| profile.name == "Call of Duty: Infinite Warfare")
        );

        let gaming = profiles
            .iter()
            .find(|profile| profile.source_id == "Gaming")
            .unwrap();
        let headphone = gaming.profile_for("Headphone", None).unwrap();
        let speakers = gaming.profile_for("Speakers", Some("2.0")).unwrap();
        assert_eq!(
            headphone.controls["FX: Smart Volume"].playback_level,
            Some(11)
        );
        assert_eq!(
            speakers.controls["FX: Smart Volume"].playback_level,
            Some(74)
        );
        assert!(!headphone.controls.contains_key("Output Select"));
        assert!(!speakers.controls.contains_key("Output Select"));
    }

    #[test]
    fn adapts_command_bass_to_lfe_speaker_layouts_without_changing_the_route() {
        let preset = builtin_profiles()
            .unwrap()
            .iter()
            .find(|profile| {
                profile
                    .speaker
                    .controls
                    .get("FX: X-Bass")
                    .and_then(|control| control.playback_switch)
                    == Some(true)
            })
            .unwrap();
        let profile = preset.profile_for("Speakers", Some("5.1")).unwrap();

        assert_eq!(profile.controls["FX: X-Bass"].playback_switch, Some(false));
        assert_eq!(
            profile.controls["Bass Redirection"].playback_switch,
            Some(true)
        );
        assert!(profile.controls.contains_key("Bass Redirection Crossover"));
        assert!(!profile.controls.contains_key("FX: X-Bass Crossover"));
        assert!(!profile.controls.contains_key("Surround Channel Config"));
    }
}
