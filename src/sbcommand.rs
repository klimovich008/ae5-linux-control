use crate::{Profile, ProfileControl, ProfileError};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::str::FromStr;

const MAX_SOURCE_BYTES: u64 = 1024 * 1024;
const EQ_FREQUENCIES: [u32; 10] = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum SbCommandTarget {
    Speaker,
    Headphone,
}

#[derive(Debug)]
pub enum SbCommandError {
    Io(io::Error),
    Json(serde_json::Error),
    Profile(ProfileError),
    Invalid(String),
}

pub fn import_profile(
    name: &str,
    profile_path: &Path,
    eq_path: &Path,
    target: SbCommandTarget,
) -> Result<Profile, SbCommandError> {
    let source: SourceProfile = load_json(profile_path)?;
    validate_product(&source.product, profile_path)?;
    let settings = select_settings(&source.settings, target)?;
    let mut controls = effect_controls(settings)?;

    let eq: SourceEq = load_json(eq_path)?;
    validate_product(&eq.product, eq_path)?;
    add_eq_controls(&mut controls, &eq, target)?;
    controls.insert(
        "FX: Equalizer".to_owned(),
        ProfileControl {
            playback_switch: Some(true),
            ..ProfileControl::default()
        },
    );
    Profile::new(name, controls).map_err(Into::into)
}

fn effect_controls(
    settings: &SourceSettings,
) -> Result<BTreeMap<String, ProfileControl>, SbCommandError> {
    let mut controls = BTreeMap::new();
    let master = settings
        .sbx_master
        .as_ref()
        .ok_or_else(|| invalid("profile is missing SBXMaster settings"))?;
    controls.insert(
        "Enable OutFX".to_owned(),
        switch(master.enable.unwrap_or(true)),
    );
    add_effect(
        &mut controls,
        "FX: Surround",
        settings.surround.as_ref(),
        master.surround_enable,
    )?;
    add_effect(
        &mut controls,
        "FX: Crystalizer",
        settings.crystalizer.as_ref(),
        master.crystalizer_enable,
    )?;
    if let Some(bass) = &settings.bass {
        let enabled = master.x_bass_enable.unwrap_or(bass.enable);
        controls.insert(
            "FX: X-Bass".to_owned(),
            ProfileControl {
                playback_switch: Some(enabled),
                playback_level: Some(percent("Bass.Level", bass.level)?),
                ..ProfileControl::default()
            },
        );
        if bass.x_over != 0.0 || enabled {
            controls.insert(
                "FX: X-Bass Crossover".to_owned(),
                ProfileControl {
                    playback_level: Some(crossover(bass.x_over)?),
                    ..ProfileControl::default()
                },
            );
        }
    }
    if let Some(svm) = &settings.svm {
        controls.insert(
            "FX: Smart Volume".to_owned(),
            ProfileControl {
                playback_switch: Some(master.svm_enable.unwrap_or(svm.enable)),
                playback_level: Some(percent("SVM.Level", svm.level)?),
                ..ProfileControl::default()
            },
        );
        controls.insert(
            "FX: Smart Volume Setting".to_owned(),
            ProfileControl {
                choice: Some(svm_mode(svm.mode)?.to_owned()),
                ..ProfileControl::default()
            },
        );
    }
    add_effect(
        &mut controls,
        "FX: Dialog Plus",
        settings.dialog_plus.as_ref(),
        master.dialog_plus_enable,
    )?;
    Ok(controls)
}

fn add_effect(
    controls: &mut BTreeMap<String, ProfileControl>,
    control_name: &str,
    effect: Option<&SourceEffect>,
    master_enabled: Option<bool>,
) -> Result<(), SbCommandError> {
    if let Some(effect) = effect {
        controls.insert(
            control_name.to_owned(),
            ProfileControl {
                playback_switch: Some(master_enabled.unwrap_or(effect.enable)),
                playback_level: Some(percent(&format!("{control_name}.Level"), effect.level)?),
                ..ProfileControl::default()
            },
        );
    }
    Ok(())
}

fn add_eq_controls(
    controls: &mut BTreeMap<String, ProfileControl>,
    eq: &SourceEq,
    target: SbCommandTarget,
) -> Result<(), SbCommandError> {
    let expected_type = target.eq_type();
    let settings = eq
        .settings
        .iter()
        .find(|settings| settings.kind.eq_ignore_ascii_case(expected_type))
        .ok_or_else(|| invalid(format!("EQ preset has no {expected_type} settings")))?;
    if !settings.unit.eq_ignore_ascii_case("db") {
        return Err(invalid(format!(
            "unsupported EQ unit '{}'; expected dB",
            settings.unit
        )));
    }
    if settings.pre_amp.abs() > 0.01 {
        return Err(invalid(format!(
            "EQ preamp {} dB cannot be represented by the AE-5 ALSA controls",
            settings.pre_amp
        )));
    }
    if settings.bands.len() != EQ_FREQUENCIES.len() {
        return Err(invalid(format!(
            "EQ preset has {} bands; expected {}",
            settings.bands.len(),
            EQ_FREQUENCIES.len()
        )));
    }
    for (index, (band, expected_frequency)) in settings.bands.iter().zip(EQ_FREQUENCIES).enumerate()
    {
        if band.frequency != expected_frequency {
            return Err(invalid(format!(
                "EQ band {index} is {} Hz; expected {expected_frequency} Hz",
                band.frequency
            )));
        }
        controls.insert(
            format!("EQ Band{index}"),
            ProfileControl {
                playback_level: Some(eq_level(index, band.value)?),
                ..ProfileControl::default()
            },
        );
    }
    Ok(())
}

fn select_settings(
    settings: &[SourceSettings],
    target: SbCommandTarget,
) -> Result<&SourceSettings, SbCommandError> {
    let wanted = target.profile_type();
    let mut matching = settings.iter().filter(|settings| settings.kind == wanted);
    let selected = matching
        .next()
        .ok_or_else(|| invalid(format!("profile has no {target} settings")))?;
    if matching.next().is_some() {
        return Err(invalid(format!(
            "profile contains duplicate {target} settings"
        )));
    }
    Ok(selected)
}

fn percent(field: &str, value: f64) -> Result<i64, SbCommandError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(invalid(format!(
            "{field} is {value}; expected a value from 0.0 to 1.0"
        )));
    }
    Ok((value * 100.0).round() as i64)
}

fn crossover(value: f64) -> Result<i64, SbCommandError> {
    if !value.is_finite() || !(10.0..=1000.0).contains(&value) {
        return Err(invalid(format!(
            "Bass.XOver is {value}; expected 10 to 1000 Hz"
        )));
    }
    Ok((value / 10.0).round() as i64)
}

fn eq_level(index: usize, value: f64) -> Result<i64, SbCommandError> {
    if !value.is_finite() || !(-12.0..=12.0).contains(&value) {
        return Err(invalid(format!(
            "EQ band {index} is {value} dB; expected -12 to 12 dB"
        )));
    }
    Ok((24.0 + value).round() as i64)
}

fn svm_mode(mode: u8) -> Result<&'static str, SbCommandError> {
    match mode {
        0 => Ok("Normal"),
        1 => Ok("Loud"),
        2 => Ok("Night"),
        _ => Err(invalid(format!("unsupported SVM mode {mode}"))),
    }
}

fn switch(enabled: bool) -> ProfileControl {
    ProfileControl {
        playback_switch: Some(enabled),
        ..ProfileControl::default()
    }
}

fn validate_product(product: &str, path: &Path) -> Result<(), SbCommandError> {
    if product.eq_ignore_ascii_case("AE5") {
        Ok(())
    } else {
        Err(invalid(format!(
            "'{}' targets product '{product}', expected AE5",
            path.display()
        )))
    }
}

fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T, SbCommandError> {
    let file = fs::File::open(path)?;
    if file.metadata()?.len() > MAX_SOURCE_BYTES {
        return Err(invalid(format!(
            "'{}' exceeds the {MAX_SOURCE_BYTES}-byte limit",
            path.display()
        )));
    }
    let mut contents = Vec::new();
    file.take(MAX_SOURCE_BYTES + 1).read_to_end(&mut contents)?;
    if contents.len() as u64 > MAX_SOURCE_BYTES {
        return Err(invalid(format!(
            "'{}' exceeds the {MAX_SOURCE_BYTES}-byte limit",
            path.display()
        )));
    }
    serde_json::from_slice(&contents).map_err(Into::into)
}

fn invalid(message: impl Into<String>) -> SbCommandError {
    SbCommandError::Invalid(message.into())
}

impl SbCommandTarget {
    fn profile_type(self) -> u8 {
        match self {
            Self::Speaker => 0,
            Self::Headphone => 1,
        }
    }

    fn eq_type(self) -> &'static str {
        match self {
            Self::Speaker => "Speaker",
            Self::Headphone => "Headphone",
        }
    }
}

impl fmt::Display for SbCommandTarget {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.write_str(match self {
            Self::Speaker => "speaker",
            Self::Headphone => "headphone",
        })
    }
}

impl FromStr for SbCommandTarget {
    type Err = SbCommandError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "speaker" => Ok(Self::Speaker),
            "headphone" => Ok(Self::Headphone),
            _ => Err(invalid("target must be 'speaker' or 'headphone'")),
        }
    }
}

impl fmt::Display for SbCommandError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(output, "{error}"),
            Self::Json(error) => write!(output, "invalid Sound Blaster Command JSON: {error}"),
            Self::Profile(error) => write!(output, "{error}"),
            Self::Invalid(message) => output.write_str(message),
        }
    }
}

impl Error for SbCommandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Profile(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<io::Error> for SbCommandError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for SbCommandError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<ProfileError> for SbCommandError {
    fn from(error: ProfileError) -> Self {
        Self::Profile(error)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceProfile {
    product: String,
    settings: Vec<SourceSettings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceSettings {
    #[serde(rename = "Type")]
    kind: u8,
    surround: Option<SourceEffect>,
    crystalizer: Option<SourceEffect>,
    bass: Option<SourceBass>,
    #[serde(rename = "SVM")]
    svm: Option<SourceSvm>,
    dialog_plus: Option<SourceEffect>,
    #[serde(rename = "SBXMaster")]
    sbx_master: Option<SourceMaster>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceEffect {
    enable: bool,
    level: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceBass {
    enable: bool,
    level: f64,
    #[serde(rename = "XOver")]
    x_over: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceSvm {
    enable: bool,
    level: f64,
    mode: u8,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceMaster {
    enable: Option<bool>,
    surround_enable: Option<bool>,
    crystalizer_enable: Option<bool>,
    #[serde(rename = "XBassEnable")]
    x_bass_enable: Option<bool>,
    #[serde(rename = "SVMEnable")]
    svm_enable: Option<bool>,
    dialog_plus_enable: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceEq {
    product: String,
    settings: Vec<SourceEqSettings>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceEqSettings {
    #[serde(rename = "Type")]
    kind: String,
    unit: String,
    pre_amp: f64,
    bands: Vec<SourceBand>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceBand {
    frequency: u32,
    value: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_creative_levels_crossovers_modes_and_eq_bands() {
        assert_eq!(percent("level", 0.675).unwrap(), 68);
        assert_eq!(crossover(80.0).unwrap(), 8);
        assert_eq!(eq_level(0, -12.0).unwrap(), 12);
        assert_eq!(eq_level(0, 0.0).unwrap(), 24);
        assert_eq!(eq_level(0, 12.0).unwrap(), 36);
        assert_eq!(svm_mode(2).unwrap(), "Night");
    }

    #[test]
    fn rejects_values_that_the_alsa_controls_cannot_represent() {
        assert!(percent("level", 1.01).is_err());
        assert!(crossover(0.0).is_err());
        assert!(eq_level(0, 12.1).is_err());
        assert!(svm_mode(3).is_err());
        assert!("other".parse::<SbCommandTarget>().is_err());
    }

    #[test]
    fn parses_creative_schema_and_honors_custom_profile_master_flags() {
        let profile: SourceProfile = serde_json::from_str(
            r#"{
                "Product":"AE5",
                "Settings":[{
                    "Type":1,
                    "Surround":{"Enable":true,"Level":0.67},
                    "Bass":{"Enable":true,"Level":0.0,"XOver":0.0},
                    "SBXMaster":{
                        "Enable":true,
                        "SurroundEnable":false,
                        "XBassEnable":false
                    }
                }]
            }"#,
        )
        .unwrap();
        let settings = select_settings(&profile.settings, SbCommandTarget::Headphone).unwrap();
        let controls = effect_controls(settings).unwrap();

        assert_eq!(
            controls["FX: Surround"],
            ProfileControl {
                playback_switch: Some(false),
                playback_level: Some(67),
                ..ProfileControl::default()
            }
        );
        assert_eq!(
            controls["FX: X-Bass"],
            ProfileControl {
                playback_switch: Some(false),
                playback_level: Some(0),
                ..ProfileControl::default()
            }
        );
        assert!(!controls.contains_key("FX: X-Bass Crossover"));
    }
}
