use crate::{Profile, ProfileControl, builtin_profiles, profile_library};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::Path;

const EQ_BANDS: usize = 10;
const EQ_FLAT_RAW: i64 = 24;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "daemon", derive(zbus::zvariant::Type))]
pub struct EffectsProfileEntry {
    pub id: String,
    pub name: String,
    pub source: String,
    pub read_only: bool,
    pub outfx_enabled: bool,
    pub surround_available: bool,
    pub surround_enabled: bool,
    pub surround_level: u16,
    pub crystalizer_available: bool,
    pub crystalizer_enabled: bool,
    pub crystalizer_level: u16,
    pub bass_available: bool,
    pub bass_enabled: bool,
    pub bass_level: u16,
    pub smart_volume_available: bool,
    pub smart_volume_enabled: bool,
    pub smart_volume_level: u16,
    pub smart_volume_mode: String,
    pub dialog_available: bool,
    pub dialog_enabled: bool,
    pub dialog_level: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "daemon", derive(zbus::zvariant::Type))]
pub struct EqPresetEntry {
    pub id: String,
    pub name: String,
    pub source: String,
    pub read_only: bool,
    pub enabled: bool,
    pub gains_tenths_db: Vec<i16>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "daemon", derive(zbus::zvariant::Type))]
pub struct SoundObjectCatalog {
    pub schema_version: u32,
    pub effects_profiles: Vec<EffectsProfileEntry>,
    pub eq_presets: Vec<EqPresetEntry>,
    pub warnings: Vec<String>,
}

pub fn sound_object_catalog(
    output: &str,
    speaker_layout: Option<&str>,
) -> Result<SoundObjectCatalog, String> {
    let output_choice = match output {
        "Headphones" | "Headphone" => Some("Headphone"),
        "Speakers" => Some("Speakers"),
        _ => None,
    };
    let mut catalog = SoundObjectCatalog {
        schema_version: 1,
        effects_profiles: Vec::new(),
        eq_presets: Vec::new(),
        warnings: Vec::new(),
    };

    if let Some(output_choice) = output_choice {
        for builtin in builtin_profiles()? {
            let profile = builtin
                .profile_for(output_choice, speaker_layout)
                .map_err(|error| error.to_string())?;
            let id = format!("factory:{}", builtin.source_id);
            if let Some(entry) = effects_from_profile(&id, &builtin.name, "Factory", true, &profile)
            {
                catalog.effects_profiles.push(entry);
            }
            match eq_from_profile(&id, &builtin.name, "Factory", true, &profile) {
                Ok(Some(entry)) => catalog.eq_presets.push(entry),
                Ok(None) => {}
                Err(error) => catalog
                    .warnings
                    .push(format!("{} equalizer: {error}", builtin.name)),
            }
        }
    } else {
        catalog.warnings.push(format!(
            "Factory profiles are unavailable for output '{output}'."
        ));
    }

    let library = profile_library().map_err(|error| error.to_string())?;
    catalog.warnings.extend(library.skipped);
    for stored in library.profiles {
        if !profile_matches_output(&stored.profile, output_choice) {
            continue;
        }
        let id = format!("user:{}", profile_file_id(&stored.path));
        let display_name = user_display_name(&stored.profile.name, output_choice);
        if let Some(entry) =
            effects_from_profile(&id, &display_name, "User library", false, &stored.profile)
        {
            catalog.effects_profiles.push(entry);
        }
        match eq_from_profile(&id, &display_name, "User library", false, &stored.profile) {
            Ok(Some(entry)) => catalog.eq_presets.push(entry),
            Ok(None) => {}
            Err(error) => catalog
                .warnings
                .push(format!("{} equalizer: {error}", stored.profile.name)),
        }
    }

    sort_and_disambiguate_effects(&mut catalog.effects_profiles);
    sort_and_disambiguate_eq(&mut catalog.eq_presets);
    catalog.warnings.sort();
    Ok(catalog)
}

fn effects_from_profile(
    id: &str,
    name: &str,
    source: &str,
    read_only: bool,
    profile: &Profile,
) -> Option<EffectsProfileEntry> {
    let controls = &profile.controls;
    let has_effects = controls
        .keys()
        .any(|name| name == "Enable OutFX" || is_enhancement_control(name));
    has_effects.then(|| EffectsProfileEntry {
        id: format!("effects:{id}"),
        name: name.to_owned(),
        source: source.to_owned(),
        read_only,
        outfx_enabled: switch(controls, "Enable OutFX"),
        surround_available: controls.contains_key("FX: Surround"),
        surround_enabled: switch(controls, "FX: Surround"),
        surround_level: level(controls, "FX: Surround"),
        crystalizer_available: controls.contains_key("FX: Crystalizer"),
        crystalizer_enabled: switch(controls, "FX: Crystalizer"),
        crystalizer_level: level(controls, "FX: Crystalizer"),
        bass_available: controls.contains_key("FX: X-Bass"),
        bass_enabled: switch(controls, "FX: X-Bass"),
        bass_level: level(controls, "FX: X-Bass"),
        smart_volume_available: controls.contains_key("FX: Smart Volume"),
        smart_volume_enabled: switch(controls, "FX: Smart Volume"),
        smart_volume_level: level(controls, "FX: Smart Volume"),
        smart_volume_mode: choice(controls, "FX: Smart Volume Setting", "Normal"),
        dialog_available: controls.contains_key("FX: Dialog Plus"),
        dialog_enabled: switch(controls, "FX: Dialog Plus"),
        dialog_level: level(controls, "FX: Dialog Plus"),
    })
}

fn eq_from_profile(
    id: &str,
    name: &str,
    source: &str,
    read_only: bool,
    profile: &Profile,
) -> Result<Option<EqPresetEntry>, String> {
    let has_any_band = (0..EQ_BANDS)
        .map(|index| format!("EQ Band{index}"))
        .any(|name| profile.controls.contains_key(&name));
    if !has_any_band {
        return Ok(None);
    }

    let gains_tenths_db = (0..EQ_BANDS)
        .map(|index| {
            let control_name = format!("EQ Band{index}");
            let raw = profile
                .controls
                .get(&control_name)
                .and_then(|control| control.playback_level)
                .ok_or_else(|| format!("missing {control_name}"))?;
            if !(0..=48).contains(&raw) {
                return Err(format!("{control_name} value {raw} is outside 0..48"));
            }
            i16::try_from((raw - EQ_FLAT_RAW) * 10)
                .map_err(|_| format!("{control_name} cannot be represented"))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(EqPresetEntry {
        id: format!("eq:{id}"),
        name: name.to_owned(),
        source: source.to_owned(),
        read_only,
        enabled: profile
            .controls
            .get("FX: Equalizer")
            .and_then(|control| control.playback_switch)
            .unwrap_or(true),
        gains_tenths_db,
    }))
}

fn is_enhancement_control(name: &str) -> bool {
    name.starts_with("FX:")
        && !matches!(name, "FX: Equalizer" | "FX: Equalizer Preset")
        && !name.starts_with("FX: EQ")
}

fn switch(controls: &BTreeMap<String, ProfileControl>, name: &str) -> bool {
    controls
        .get(name)
        .and_then(|control| control.playback_switch)
        .unwrap_or(false)
}

fn level(controls: &BTreeMap<String, ProfileControl>, name: &str) -> u16 {
    controls
        .get(name)
        .and_then(|control| control.playback_level)
        .and_then(|value| u16::try_from(value.clamp(0, 100)).ok())
        .unwrap_or(0)
}

fn choice(controls: &BTreeMap<String, ProfileControl>, name: &str, fallback: &str) -> String {
    controls
        .get(name)
        .and_then(|control| control.choice.clone())
        .unwrap_or_else(|| fallback.to_owned())
}

fn profile_matches_output(profile: &Profile, output_choice: Option<&str>) -> bool {
    let Some(profile_output) = profile
        .controls
        .get("Output Select")
        .and_then(|control| control.choice.as_deref())
    else {
        return true;
    };
    output_choice.is_some_and(|output| output == profile_output)
}

fn profile_file_id(path: &Path) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("profile")
        .to_owned()
}

fn user_display_name(name: &str, output_choice: Option<&str>) -> String {
    let name = name.strip_prefix("EQ · ").unwrap_or(name);
    let suffix = match output_choice {
        Some("Headphone") => " · Headphones",
        Some("Speakers") => " · Speakers",
        _ => "",
    };
    name.strip_suffix(suffix).unwrap_or(name).to_owned()
}

fn sort_and_disambiguate_effects(entries: &mut [EffectsProfileEntry]) {
    entries
        .sort_by_cached_key(|entry| (entry.read_only, entry.name.to_lowercase(), entry.id.clone()));
    let counts = name_counts(entries.iter().map(|entry| entry.name.as_str()));
    for entry in entries {
        if counts.get(&entry.name.to_lowercase()).copied().unwrap_or(0) > 1 {
            entry.name = format!("{} · {}", entry.name, entry.source);
        }
    }
}

fn sort_and_disambiguate_eq(entries: &mut [EqPresetEntry]) {
    entries
        .sort_by_cached_key(|entry| (entry.read_only, entry.name.to_lowercase(), entry.id.clone()));
    let counts = name_counts(entries.iter().map(|entry| entry.name.as_str()));
    for entry in entries {
        if counts.get(&entry.name.to_lowercase()).copied().unwrap_or(0) > 1 {
            entry.name = format!("{} · {}", entry.name, entry.source);
        }
    }
}

fn name_counts<'a>(names: impl Iterator<Item = &'a str>) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for name in names {
        *counts.entry(name.to_lowercase()).or_default() += 1;
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn profile(name: &str, output: Option<&str>) -> Profile {
        let mut controls = BTreeMap::from([
            (
                "Enable OutFX".to_owned(),
                ProfileControl {
                    playback_switch: Some(true),
                    ..ProfileControl::default()
                },
            ),
            (
                "FX: Surround".to_owned(),
                ProfileControl {
                    playback_switch: Some(true),
                    playback_level: Some(35),
                    ..ProfileControl::default()
                },
            ),
        ]);
        for index in 0..EQ_BANDS {
            controls.insert(
                format!("EQ Band{index}"),
                ProfileControl {
                    playback_level: Some(EQ_FLAT_RAW + index as i64 - 5),
                    ..ProfileControl::default()
                },
            );
        }
        if let Some(output) = output {
            controls.insert(
                "Output Select".to_owned(),
                ProfileControl {
                    choice: Some(output.to_owned()),
                    ..ProfileControl::default()
                },
            );
        }
        Profile {
            format_version: 1,
            name: name.to_owned(),
            target: "1102:0012/1102:0051".to_owned(),
            controls,
        }
    }

    #[test]
    fn every_factory_profile_produces_independent_effects_and_eq_objects() {
        let profiles = builtin_profiles().unwrap();
        let extracted = profiles
            .iter()
            .map(|builtin| {
                let profile = builtin.profile_for("Headphone", None).unwrap();
                (
                    effects_from_profile(
                        &builtin.source_id,
                        &builtin.name,
                        "Factory",
                        true,
                        &profile,
                    ),
                    eq_from_profile(&builtin.source_id, &builtin.name, "Factory", true, &profile)
                        .unwrap(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(extracted.len(), 33);
        assert!(
            extracted
                .iter()
                .all(|(effects, eq)| effects.is_some() && eq.is_some())
        );
        assert!(profiles.iter().any(|profile| profile.name == "Gaming"));
    }

    #[test]
    fn one_profile_becomes_two_independent_objects() {
        let profile = profile("My profile · Headphones", Some("Headphone"));
        let effects =
            effects_from_profile("user:mine", "My profile", "User library", false, &profile)
                .unwrap();
        let mut eq = eq_from_profile("user:mine", "My profile", "User library", false, &profile)
            .unwrap()
            .unwrap();

        eq.gains_tenths_db[0] = 120;
        assert_eq!(effects.surround_level, 35);
        assert_eq!(effects.name, eq.name);
        assert_ne!(effects.id, eq.id);
    }

    #[test]
    fn route_specific_user_profiles_do_not_cross_outputs() {
        let headphones = profile("Headphones", Some("Headphone"));
        let speakers = profile("Speakers", Some("Speakers"));
        assert!(profile_matches_output(&headphones, Some("Headphone")));
        assert!(!profile_matches_output(&speakers, Some("Headphone")));
        assert!(profile_matches_output(&speakers, Some("Speakers")));
    }

    #[test]
    fn user_names_drop_redundant_section_and_output_labels() {
        assert_eq!(
            user_display_name("EQ · SHP Last", Some("Headphone")),
            "SHP Last"
        );
        assert_eq!(
            user_display_name("My profile · Headphones", Some("Headphone")),
            "My profile"
        );
        assert_eq!(profile_file_id(&PathBuf::from("mine.json")), "mine");
    }

    #[test]
    fn incomplete_equalizer_is_rejected_without_affecting_effects() {
        let mut profile = profile("Broken", None);
        profile.controls.remove("EQ Band4");
        assert!(eq_from_profile("broken", "Broken", "User", false, &profile).is_err());
        assert!(effects_from_profile("broken", "Broken", "User", false, &profile).is_some());
    }
}
