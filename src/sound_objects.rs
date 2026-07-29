use crate::{Profile, ProfileControl, builtin_profiles, profile_library};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
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
    let output_choice = output_choice(output);
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
    let has_effects = controls.keys().any(|name| is_effects_control(name));
    has_effects.then(|| EffectsProfileEntry {
        id: format!("effects:{id}"),
        name: name.to_owned(),
        source: source.to_owned(),
        read_only: read_only || profile_has_eq(profile),
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
        read_only: read_only || profile_has_effects(profile),
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

fn is_effects_control(name: &str) -> bool {
    name == "Enable OutFX" || is_enhancement_control(name)
}

fn profile_has_effects(profile: &Profile) -> bool {
    profile.controls.keys().any(|name| is_effects_control(name))
}

fn profile_has_eq(profile: &Profile) -> bool {
    profile
        .controls
        .keys()
        .any(|name| name.starts_with("EQ Band") || matches!(name.as_str(), "FX: Equalizer"))
}

fn output_choice(output: &str) -> Option<&'static str> {
    match output {
        "Headphones" | "Headphone" => Some("Headphone"),
        "Speakers" => Some("Speakers"),
        _ => None,
    }
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
    path.file_name()
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

pub fn save_effects_profile(
    draft: &EffectsProfileEntry,
    output: &str,
) -> Result<EffectsProfileEntry, String> {
    let directory = profile_library()
        .map_err(|error| error.to_string())?
        .directory;
    let id = save_effects_profile_at(&directory, draft, output)?;
    saved_effects_entry(&id, output)
}

pub fn save_effects_profile_as(
    draft: &EffectsProfileEntry,
    name: &str,
    output: &str,
) -> Result<EffectsProfileEntry, String> {
    let directory = profile_library()
        .map_err(|error| error.to_string())?
        .directory;
    let id = save_effects_profile_as_at(&directory, draft, name, output)?;
    saved_effects_entry(&id, output)
}

pub fn save_eq_preset(draft: &EqPresetEntry, output: &str) -> Result<EqPresetEntry, String> {
    let directory = profile_library()
        .map_err(|error| error.to_string())?
        .directory;
    let id = save_eq_preset_at(&directory, draft, output)?;
    saved_eq_entry(&id, output)
}

pub fn save_eq_preset_as(
    draft: &EqPresetEntry,
    name: &str,
    output: &str,
) -> Result<EqPresetEntry, String> {
    let directory = profile_library()
        .map_err(|error| error.to_string())?
        .directory;
    let id = save_eq_preset_as_at(&directory, draft, name, output)?;
    saved_eq_entry(&id, output)
}

impl EffectsProfileEntry {
    pub fn set_control(&mut self, control: &str, enabled: bool, level: i32) -> Result<(), String> {
        let level = u16::try_from(level)
            .ok()
            .filter(|level| *level <= 100)
            .ok_or_else(|| format!("{control} level must be between 0 and 100 percent."))?;
        let (available, stored_enabled, stored_level) = match control {
            "surround" => (
                self.surround_available,
                &mut self.surround_enabled,
                &mut self.surround_level,
            ),
            "crystalizer" => (
                self.crystalizer_available,
                &mut self.crystalizer_enabled,
                &mut self.crystalizer_level,
            ),
            "bass" => (
                self.bass_available,
                &mut self.bass_enabled,
                &mut self.bass_level,
            ),
            "smart-volume" => (
                self.smart_volume_available,
                &mut self.smart_volume_enabled,
                &mut self.smart_volume_level,
            ),
            "dialog" => (
                self.dialog_available,
                &mut self.dialog_enabled,
                &mut self.dialog_level,
            ),
            _ => return Err(format!("Unknown Effects control '{control}'.")),
        };
        if !available {
            return Err(format!("{control} is unavailable in this Effects profile."));
        }
        *stored_enabled = enabled;
        *stored_level = level;
        Ok(())
    }
}

impl EqPresetEntry {
    pub fn set_band_gain(&mut self, index: i32, gain_tenths_db: i32) -> Result<(), String> {
        let index = usize::try_from(index)
            .ok()
            .filter(|index| *index < EQ_BANDS)
            .ok_or_else(|| "Equalizer band index must be between 0 and 9.".to_owned())?;
        let gain = i16::try_from(gain_tenths_db)
            .ok()
            .filter(|gain| (-120..=120).contains(gain) && *gain % 10 == 0)
            .ok_or_else(|| {
                "Equalizer gain must be a whole decibel between -12 and +12 dB.".to_owned()
            })?;
        let Some(stored_gain) = self.gains_tenths_db.get_mut(index) else {
            return Err("Equalizer draft does not contain ten bands.".to_owned());
        };
        *stored_gain = gain;
        Ok(())
    }
}

fn saved_effects_entry(id: &str, output: &str) -> Result<EffectsProfileEntry, String> {
    sound_object_catalog(output, None)?
        .effects_profiles
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| format!("Saved Effects profile '{id}' was not found after writing it."))
}

fn saved_eq_entry(id: &str, output: &str) -> Result<EqPresetEntry, String> {
    sound_object_catalog(output, None)?
        .eq_presets
        .into_iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| format!("Saved EQ preset '{id}' was not found after writing it."))
}

fn save_effects_profile_at(
    directory: &Path,
    draft: &EffectsProfileEntry,
    output: &str,
) -> Result<String, String> {
    validate_effects_draft(draft)?;
    let output_choice = writable_output_choice(output)?;
    let mut stored = stored_profile_for_id(directory, &draft.id, "effects")?;
    let current = effects_from_profile(
        &draft.id,
        &stored.profile.name,
        "User library",
        false,
        &stored.profile,
    )
    .ok_or_else(|| "The selected file does not contain an Effects profile.".to_owned())?;
    if current.read_only {
        return Err(
            "This Effects object is read-only because it is factory-owned or also contains EQ."
                .to_owned(),
        );
    }
    if !profile_matches_output(&stored.profile, Some(output_choice)) {
        return Err("The Effects profile belongs to a different output.".to_owned());
    }
    update_effects_controls(&mut stored.profile.controls, draft, output_choice);
    stored
        .profile
        .save_replace(&stored.path)
        .map_err(|error| error.to_string())?;
    Ok(draft.id.clone())
}

fn save_effects_profile_as_at(
    directory: &Path,
    draft: &EffectsProfileEntry,
    name: &str,
    output: &str,
) -> Result<String, String> {
    validate_effects_draft(draft)?;
    let output_choice = writable_output_choice(output)?;
    let name = section_profile_name(name, output_choice, false)?;
    let mut controls =
        source_section_controls(directory, &draft.id, "effects", is_effects_control)?;
    update_effects_controls(&mut controls, draft, output_choice);
    let profile = Profile::new(&name, controls).map_err(|error| error.to_string())?;
    save_new_section_profile(directory, "effects", name_without_route(&name), &profile)
}

fn save_eq_preset_at(
    directory: &Path,
    draft: &EqPresetEntry,
    output: &str,
) -> Result<String, String> {
    validate_eq_draft(draft)?;
    let output_choice = writable_output_choice(output)?;
    let mut stored = stored_profile_for_id(directory, &draft.id, "eq")?;
    let current = eq_from_profile(
        &draft.id,
        &stored.profile.name,
        "User library",
        false,
        &stored.profile,
    )?
    .ok_or_else(|| "The selected file does not contain an EQ preset.".to_owned())?;
    if current.read_only {
        return Err(
            "This EQ object is read-only because it is factory-owned or also contains Effects."
                .to_owned(),
        );
    }
    if !profile_matches_output(&stored.profile, Some(output_choice)) {
        return Err("The EQ preset belongs to a different output.".to_owned());
    }
    update_eq_controls(&mut stored.profile.controls, draft, output_choice)?;
    stored
        .profile
        .save_replace(&stored.path)
        .map_err(|error| error.to_string())?;
    Ok(draft.id.clone())
}

fn save_eq_preset_as_at(
    directory: &Path,
    draft: &EqPresetEntry,
    name: &str,
    output: &str,
) -> Result<String, String> {
    validate_eq_draft(draft)?;
    let output_choice = writable_output_choice(output)?;
    let name = section_profile_name(name, output_choice, true)?;
    let mut controls = source_section_controls(directory, &draft.id, "eq", is_eq_control)?;
    update_eq_controls(&mut controls, draft, output_choice)?;
    let profile = Profile::new(&name, controls).map_err(|error| error.to_string())?;
    save_new_section_profile(directory, "eq", name_without_route(&name), &profile)
}

fn source_section_controls(
    directory: &Path,
    id: &str,
    section: &str,
    belongs_to_section: fn(&str) -> bool,
) -> Result<BTreeMap<String, ProfileControl>, String> {
    if !id.starts_with(&format!("{section}:user:")) {
        return Ok(BTreeMap::new());
    }
    let stored = stored_profile_for_id(directory, id, section)?;
    Ok(stored
        .profile
        .controls
        .into_iter()
        .filter(|(name, _)| belongs_to_section(name))
        .collect())
}

fn stored_profile_for_id(
    directory: &Path,
    id: &str,
    section: &str,
) -> Result<profile_library::StoredProfile, String> {
    let prefix = format!("{section}:user:");
    let file_name = id
        .strip_prefix(&prefix)
        .ok_or_else(|| format!("Only user {section} objects can be replaced."))?;
    let path = Path::new(file_name);
    let is_direct_json = path.file_name().and_then(|name| name.to_str()) == Some(file_name)
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
    if !is_direct_json {
        return Err("The profile identifier is not a direct JSON library file.".to_owned());
    }
    profile_library::load_library_profile_at(directory, &directory.join(path))
        .map_err(|error| error.to_string())
}

fn validate_effects_draft(draft: &EffectsProfileEntry) -> Result<(), String> {
    for (name, available, level) in [
        ("Surround", draft.surround_available, draft.surround_level),
        (
            "Crystalizer",
            draft.crystalizer_available,
            draft.crystalizer_level,
        ),
        ("Bass", draft.bass_available, draft.bass_level),
        (
            "Smart Volume",
            draft.smart_volume_available,
            draft.smart_volume_level,
        ),
        ("Dialog+", draft.dialog_available, draft.dialog_level),
    ] {
        if available && level > 100 {
            return Err(format!("{name} must be between 0 and 100 percent."));
        }
    }
    if draft.smart_volume_available
        && !matches!(
            draft.smart_volume_mode.as_str(),
            "Normal" | "Night" | "Loud"
        )
    {
        return Err(format!(
            "Unsupported Smart Volume mode '{}'.",
            draft.smart_volume_mode
        ));
    }
    Ok(())
}

fn validate_eq_draft(draft: &EqPresetEntry) -> Result<(), String> {
    if draft.gains_tenths_db.len() != EQ_BANDS {
        return Err("An EQ preset must contain exactly ten bands.".to_owned());
    }
    if draft
        .gains_tenths_db
        .iter()
        .any(|gain| !(-120..=120).contains(gain) || *gain % 10 != 0)
    {
        return Err("Every EQ gain must be a whole decibel between -12 and +12 dB.".to_owned());
    }
    Ok(())
}

fn update_effects_controls(
    controls: &mut BTreeMap<String, ProfileControl>,
    draft: &EffectsProfileEntry,
    output_choice: &str,
) {
    controls.insert(
        "Output Select".to_owned(),
        ProfileControl {
            choice: Some(output_choice.to_owned()),
            ..ProfileControl::default()
        },
    );
    controls.insert(
        "Enable OutFX".to_owned(),
        ProfileControl {
            playback_switch: Some(draft.outfx_enabled),
            ..ProfileControl::default()
        },
    );
    for (name, available, enabled, level) in [
        (
            "FX: Surround",
            draft.surround_available,
            draft.surround_enabled,
            draft.surround_level,
        ),
        (
            "FX: Crystalizer",
            draft.crystalizer_available,
            draft.crystalizer_enabled,
            draft.crystalizer_level,
        ),
        (
            "FX: X-Bass",
            draft.bass_available,
            draft.bass_enabled,
            draft.bass_level,
        ),
        (
            "FX: Smart Volume",
            draft.smart_volume_available,
            draft.smart_volume_enabled,
            draft.smart_volume_level,
        ),
        (
            "FX: Dialog Plus",
            draft.dialog_available,
            draft.dialog_enabled,
            draft.dialog_level,
        ),
    ] {
        if available {
            controls.insert(
                name.to_owned(),
                ProfileControl {
                    playback_switch: Some(enabled),
                    playback_level: Some(i64::from(level)),
                    ..ProfileControl::default()
                },
            );
        }
    }
    if draft.smart_volume_available {
        controls.insert(
            "FX: Smart Volume Setting".to_owned(),
            ProfileControl {
                choice: Some(draft.smart_volume_mode.clone()),
                ..ProfileControl::default()
            },
        );
    }
}

fn update_eq_controls(
    controls: &mut BTreeMap<String, ProfileControl>,
    draft: &EqPresetEntry,
    output_choice: &str,
) -> Result<(), String> {
    validate_eq_draft(draft)?;
    controls.insert(
        "Output Select".to_owned(),
        ProfileControl {
            choice: Some(output_choice.to_owned()),
            ..ProfileControl::default()
        },
    );
    controls.insert(
        "FX: Equalizer".to_owned(),
        ProfileControl {
            playback_switch: Some(draft.enabled),
            ..ProfileControl::default()
        },
    );
    controls.insert(
        "FX: Equalizer Preset".to_owned(),
        ProfileControl {
            choice: Some("Flat".to_owned()),
            ..ProfileControl::default()
        },
    );
    for (index, gain) in draft.gains_tenths_db.iter().enumerate() {
        controls.insert(
            format!("EQ Band{index}"),
            ProfileControl {
                playback_level: Some(EQ_FLAT_RAW + i64::from(*gain) / 10),
                ..ProfileControl::default()
            },
        );
    }
    Ok(())
}

fn is_eq_control(name: &str) -> bool {
    name.starts_with("EQ Band")
        || matches!(name, "FX: Equalizer" | "FX: Equalizer Preset")
        || name.starts_with("FX: EQ")
}

fn writable_output_choice(output: &str) -> Result<&'static str, String> {
    output_choice(output)
        .ok_or_else(|| format!("Profiles cannot be saved for unsupported output '{output}'."))
}

fn section_profile_name(name: &str, output_choice: &str, eq: bool) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("Profile name cannot be empty.".to_owned());
    }
    let route = if output_choice == "Headphone" {
        "Headphones"
    } else {
        "Speakers"
    };
    Ok(if eq {
        format!("EQ · {name} · {route}")
    } else {
        format!("{name} · {route}")
    })
}

fn name_without_route(name: &str) -> &str {
    let name = name.strip_prefix("EQ · ").unwrap_or(name);
    name.strip_suffix(" · Headphones")
        .or_else(|| name.strip_suffix(" · Speakers"))
        .unwrap_or(name)
}

fn save_new_section_profile(
    directory: &Path,
    section: &str,
    display_name: &str,
    profile: &Profile,
) -> Result<String, String> {
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let slug = file_slug(display_name);
    let file_name = format!("{section}-{slug}.json");
    let path = directory.join(&file_name);
    if path.exists() {
        return Err(format!(
            "A {section} object named '{display_name}' already exists."
        ));
    }
    profile.save_new(&path).map_err(|error| error.to_string())?;
    Ok(format!("{section}:user:{file_name}"))
}

fn file_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut separator = false;
    for character in name.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            if separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(character);
            separator = false;
        } else {
            separator = true;
        }
        if slug.len() >= 48 {
            break;
        }
    }
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        "profile".to_owned()
    } else {
        slug.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

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
        assert!(effects.read_only);
        assert!(eq.read_only);
    }

    #[test]
    fn section_only_user_objects_remain_writable() {
        let mut effects_profile = profile("Effects", Some("Headphone"));
        effects_profile
            .controls
            .retain(|name, _| !name.starts_with("EQ Band"));
        let effects = effects_from_profile(
            "user:effects.json",
            "Effects",
            "User library",
            false,
            &effects_profile,
        )
        .unwrap();

        let mut eq_profile = profile("EQ", Some("Headphone"));
        eq_profile
            .controls
            .retain(|name, _| !is_effects_control(name));
        let eq = eq_from_profile("user:eq.json", "EQ", "User library", false, &eq_profile)
            .unwrap()
            .unwrap();

        assert!(!effects.read_only);
        assert!(!eq.read_only);
    }

    #[test]
    fn effects_save_as_writes_only_effects_for_the_live_output() {
        let directory = test_directory();
        let mut combined = profile("Combined", Some("Headphone"));
        combined.controls.insert(
            "FX: X-Bass Crossover".to_owned(),
            ProfileControl {
                playback_level: Some(8),
                ..ProfileControl::default()
            },
        );
        combined.save_new(&directory.join("combined.json")).unwrap();
        let mut draft = effects_from_profile(
            "user:combined.json",
            "Combined",
            "User library",
            false,
            &combined,
        )
        .unwrap();
        draft.surround_level = 67;

        let saved_id =
            save_effects_profile_as_at(&directory, &draft, "Night mode", "Headphones").unwrap();
        let saved = load_saved_profile(&directory, &saved_id, "effects");

        assert_eq!(
            saved.controls["Output Select"].choice.as_deref(),
            Some("Headphone")
        );
        assert_eq!(saved.controls["FX: Surround"].playback_level, Some(67));
        assert_eq!(
            saved.controls["FX: X-Bass Crossover"].playback_level,
            Some(8)
        );
        assert!(
            !saved
                .controls
                .keys()
                .any(|name| name.starts_with("EQ Band"))
        );
    }

    #[test]
    fn save_as_refuses_to_overwrite_an_existing_section_object() {
        let directory = test_directory();
        let combined = profile("Combined", Some("Headphone"));
        combined.save_new(&directory.join("combined.json")).unwrap();
        let draft = eq_from_profile(
            "user:combined.json",
            "Combined",
            "User library",
            false,
            &combined,
        )
        .unwrap()
        .unwrap();

        save_eq_preset_as_at(&directory, &draft, "My EQ", "Headphones").unwrap();
        let error = save_eq_preset_as_at(&directory, &draft, "My EQ", "Headphones").unwrap_err();

        assert!(
            error.contains("already exists"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn saving_effects_preserves_unrepresented_effects_and_other_files() {
        let directory = test_directory();
        let mut effects_profile = profile("Effects · Headphones", Some("Headphone"));
        effects_profile
            .controls
            .retain(|name, _| !name.starts_with("EQ Band"));
        effects_profile.controls.insert(
            "FX: X-Bass Crossover".to_owned(),
            ProfileControl {
                playback_level: Some(8),
                ..ProfileControl::default()
            },
        );
        let effects_path = directory.join("effects.json");
        effects_profile.save_new(&effects_path).unwrap();

        let mut eq_profile = profile("EQ · Other", Some("Headphone"));
        eq_profile
            .controls
            .retain(|name, _| !is_effects_control(name));
        let eq_path = directory.join("eq.json");
        eq_profile.save_new(&eq_path).unwrap();
        let eq_before = fs::read(&eq_path).unwrap();

        let mut draft = effects_from_profile(
            "user:effects.json",
            "Effects",
            "User library",
            false,
            &effects_profile,
        )
        .unwrap();
        draft.surround_level = 72;
        save_effects_profile_at(&directory, &draft, "Headphones").unwrap();
        let saved = Profile::load(&effects_path).unwrap();

        assert_eq!(
            saved.controls["FX: X-Bass Crossover"].playback_level,
            Some(8)
        );
        assert_eq!(saved.controls["FX: Surround"].playback_level, Some(72));
        assert_eq!(fs::read(&eq_path).unwrap(), eq_before);
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
        assert_eq!(profile_file_id(&PathBuf::from("mine.json")), "mine.json");
    }

    #[test]
    fn incomplete_equalizer_is_rejected_without_affecting_effects() {
        let mut profile = profile("Broken", None);
        profile.controls.remove("EQ Band4");
        assert!(eq_from_profile("broken", "Broken", "User", false, &profile).is_err());
        assert!(effects_from_profile("broken", "Broken", "User", false, &profile).is_some());
    }

    fn test_directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ae5-sound-objects-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        path
    }

    fn load_saved_profile(directory: &Path, id: &str, section: &str) -> Profile {
        let file_name = id
            .strip_prefix(&format!("{section}:user:"))
            .expect("saved object id");
        Profile::load(&directory.join(file_name)).unwrap()
    }
}
