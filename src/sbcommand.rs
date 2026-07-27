use crate::{EQ_FREQUENCIES, Profile, ProfileControl, ProfileError};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::str::FromStr;

const MAX_SOURCE_BYTES: u64 = 1024 * 1024;
const MAX_DISCOVERY_ENTRIES: usize = 512;
const MAX_DRIVERSTORE_ENTRIES: usize = 16_384;
const MAX_DRIVER_PACKAGES: usize = 16;
const SOURCE_METADATA_FIELDS: &[&str] = &[
    "CreatorName",
    "CreatorProfession",
    "DescriptionLong",
    "DescriptionShort",
    "FilePath",
    "Id",
    "ImageLarge",
    "ImageSmall",
    "Name",
    "Order",
    "Type",
    "Version",
];

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

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SbCommandImportReport {
    pub exact: Vec<String>,
    pub approximate: Vec<String>,
    pub unsupported: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SbCommandImport {
    pub profile: Profile,
    pub report: SbCommandImportReport,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SbCommandInstallation {
    pub user_config: PathBuf,
    pub product_dir: PathBuf,
    pub driver_version: Option<String>,
}

pub fn import_profile(
    name: &str,
    profile_path: &Path,
    eq_path: &Path,
    target: SbCommandTarget,
) -> Result<Profile, SbCommandError> {
    Ok(import_profile_with_report(name, profile_path, eq_path, target)?.profile)
}

pub fn import_profile_with_report(
    name: &str,
    profile_path: &Path,
    eq_path: &Path,
    target: SbCommandTarget,
) -> Result<SbCommandImport, SbCommandError> {
    let source: SourceProfile = load_json(profile_path)?;
    validate_product(&source.product, profile_path)?;
    let settings = select_settings(&source.settings, target)?;
    let mut report = SbCommandImportReport::default();
    report_unknown_fields(&mut report, "Profile", &source.extra, true);
    report_ae5_default(
        &mut report,
        "Profile.SpeakerMethod",
        source.speaker_method,
        "AE-5 profile routing metadata",
    );
    let mut controls = effect_controls(settings, &mut report)?;

    let eq: SourceEq = load_json(eq_path)?;
    validate_product(&eq.product, eq_path)?;
    report_unknown_fields(&mut report, "Equalizer", &eq.extra, true);
    report_ae5_default(
        &mut report,
        "Equalizer.SpeakerMethod",
        eq.speaker_method,
        "AE-5 equalizer routing metadata",
    );
    add_eq_controls(&mut controls, &eq, target, &mut report)?;
    controls.insert(
        "FX: Equalizer".to_owned(),
        ProfileControl {
            playback_switch: Some(true),
            ..ProfileControl::default()
        },
    );
    report.approximate.push(
        "selected EQ preset → FX: Equalizer (playback on; source has no enable flag)".to_owned(),
    );
    Ok(SbCommandImport {
        profile: Profile::new(name, controls)?,
        report,
    })
}

pub fn import_active_profile_with_report(
    name: &str,
    user_config_path: &Path,
    product_dir: &Path,
    target: SbCommandTarget,
) -> Result<SbCommandImport, SbCommandError> {
    let config = load_text(user_config_path)?;
    let (profile_setting, eq_setting) = match target {
        SbCommandTarget::Speaker => ("SPSelectedProfileId", "SPSelectedPresetId"),
        SbCommandTarget::Headphone => ("HPSelectedProfileId", "HPSelectedPresetId"),
    };
    let profile_id = required_user_setting(&config, profile_setting)?;
    let eq_id = required_user_setting(&config, eq_setting)?;
    validate_identifier(profile_setting, &profile_id)?;
    validate_identifier(eq_setting, &eq_id)?;

    let profile_file = format!("{profile_id}.json");
    let eq_file = format!("{eq_id}.json");
    let import = import_profile_with_report(
        name,
        &product_dir.join("Profiles").join(&profile_file),
        &product_dir.join("Presets").join("EQ").join(&eq_file),
        target,
    )?;
    let mut controls = import.profile.controls;
    let mut report = import.report;
    if let Some(version) = active_command_version(user_config_path) {
        report.exact.insert(
            0,
            format!("Sound Blaster Command {version} → active configuration"),
        );
    }
    report.exact.push(format!(
        "{profile_setting} → Profiles/{profile_file} (active {target} profile)"
    ));
    report.exact.push(format!(
        "{eq_setting} → Presets/EQ/{eq_file} (active {target} EQ)"
    ));
    controls.insert(
        "Output Select".to_owned(),
        ProfileControl {
            choice: Some(target.output_choice().to_owned()),
            ..ProfileControl::default()
        },
    );
    report.exact.push(format!(
        "active {target} target → Output Select ({})",
        target.output_choice()
    ));

    match target {
        SbCommandTarget::Speaker => {
            add_speaker_layout(&config, &mut controls, &mut report)?;
            map_lfe_bass_management(&mut controls, &mut report);
        }
        SbCommandTarget::Headphone => {
            report_headphone_tuning(&config, user_config_path, product_dir, &mut report)?
        }
    }

    Ok(SbCommandImport {
        profile: Profile::new(name, controls)?,
        report,
    })
}

pub fn import_installation_profile_with_report(
    name: &str,
    installation: &SbCommandInstallation,
    target: SbCommandTarget,
) -> Result<SbCommandImport, SbCommandError> {
    let mut import = import_active_profile_with_report(
        name,
        &installation.user_config,
        &installation.product_dir,
        target,
    )?;
    if let Some(version) = &installation.driver_version {
        let position = usize::from(
            import
                .report
                .exact
                .first()
                .is_some_and(|item| item.starts_with("Sound Blaster Command ")),
        );
        import.report.exact.insert(
            position,
            format!("Creative AE-5 driver {version} → active Windows driver package"),
        );
    }
    Ok(import)
}

pub fn discover_installation(
    windows_user_dir: &Path,
) -> Result<SbCommandInstallation, SbCommandError> {
    let local = windows_user_dir.join("AppData").join("Local");
    let config_root = local.join("Creative_Technology_Ltd");
    let mut configs = Vec::new();
    for application in subdirectories(&config_root)? {
        if !application
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("Creative.SBCommand"))
        {
            continue;
        }
        for version_dir in subdirectories(&application)? {
            let Some(version) = command_version(&version_dir) else {
                continue;
            };
            let user_config = version_dir.join("user.config");
            if is_regular_file(&user_config) {
                configs.push((version, user_config));
            }
        }
    }
    configs.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
    let newest_version = configs
        .last()
        .map(|candidate| candidate.0.clone())
        .ok_or_else(|| {
            invalid(format!(
                "no Sound Blaster Command user.config was found under '{}'",
                config_root.display()
            ))
        })?;
    let newest = configs
        .iter()
        .filter(|candidate| candidate.0 == newest_version)
        .collect::<Vec<_>>();
    if newest.len() != 1 {
        return Err(invalid(format!(
            "multiple Sound Blaster Command configs have the newest version under '{}'; choose user.config manually",
            config_root.display()
        )));
    }

    let product_root = local.join("Creative");
    let driver_version = discover_driver_version(windows_user_dir)?;
    let mut product_dirs = subdirectories(&product_root)?
        .into_iter()
        .map(|directory| directory.join("Product").join("AE5"))
        .filter(|directory| is_directory(directory))
        .collect::<Vec<_>>();
    product_dirs.sort();
    match product_dirs.as_slice() {
        [product_dir] => Ok(SbCommandInstallation {
            user_config: newest[0].1.clone(),
            product_dir: product_dir.clone(),
            driver_version,
        }),
        [] => Err(invalid(format!(
            "no AE5 product directory was found under '{}'",
            product_root.display()
        ))),
        _ => Err(invalid(format!(
            "multiple AE5 product directories were found under '{}'; choose one manually",
            product_root.display()
        ))),
    }
}

fn effect_controls(
    settings: &SourceSettings,
    report: &mut SbCommandImportReport,
) -> Result<BTreeMap<String, ProfileControl>, SbCommandError> {
    let mut controls = BTreeMap::new();
    let master = settings
        .sbx_master
        .as_ref()
        .ok_or_else(|| invalid("profile is missing SBXMaster settings"))?;
    report_unknown_fields(report, "Settings", &settings.extra, false);
    if let Some(scout) = &settings.scout {
        let configured = [
            scout.enable,
            scout.surround_enable,
            scout.crystalizer_enable,
            scout.x_bass_enable,
            scout.svm_enable,
            scout.dialog_plus_enable,
            scout.graphics_eq_enable,
        ]
        .into_iter()
        .any(|enabled| enabled != Some(false));
        if configured {
            report.unsupported.push(
                "Settings.Scout enabled or configured (no mapped AE-5 ALSA control)".to_owned(),
            );
        } else {
            report
                .exact
                .push("Settings.Scout disabled → no Linux control required".to_owned());
        }
        report_unknown_fields(report, "Settings.Scout", &scout.extra, false);
    }
    report_unknown_fields(report, "SBXMaster", &master.extra, false);
    let master_enabled = master.enable.unwrap_or(true);
    controls.insert("Enable OutFX".to_owned(), switch(master_enabled));
    report.exact.push(format!(
        "SBXMaster.Enable → Enable OutFX (playback {})",
        on_off(master_enabled)
    ));
    add_effect(
        &mut controls,
        "FX: Surround",
        "Surround",
        settings.surround.as_ref(),
        master.surround_enable,
        report,
    )?;
    add_effect(
        &mut controls,
        "FX: Crystalizer",
        "Crystalizer",
        settings.crystalizer.as_ref(),
        master.crystalizer_enable,
        report,
    )?;
    if let Some(bass) = &settings.bass {
        let enabled = master.x_bass_enable.unwrap_or(bass.enable);
        let level = percent("Bass.Level", bass.level)?;
        controls.insert(
            "FX: X-Bass".to_owned(),
            ProfileControl {
                playback_switch: Some(enabled),
                playback_level: Some(level),
                ..ProfileControl::default()
            },
        );
        report_mapping(
            report,
            is_exact_step(bass.level * 100.0),
            format!(
                "Bass → FX: X-Bass (playback {}, level {level})",
                on_off(enabled)
            ),
        );
        report_unknown_fields(report, "Bass", &bass.extra, false);
        match bass.sub_woofer_gain {
            Some(false) => report
                .exact
                .push("Bass.SubWooferGain off → no gain adjustment".to_owned()),
            Some(true) => report
                .unsupported
                .push("Bass.SubWooferGain on (no mapped AE-5 ALSA control)".to_owned()),
            None => {}
        }
        if bass.x_over != 0.0 || enabled {
            let level = crossover(bass.x_over)?;
            controls.insert(
                "FX: X-Bass Crossover".to_owned(),
                ProfileControl {
                    playback_level: Some(level),
                    ..ProfileControl::default()
                },
            );
            report_mapping(
                report,
                is_exact_step(bass.x_over / 10.0),
                format!(
                    "Bass.XOver {} Hz → FX: X-Bass Crossover ({} Hz)",
                    source_number(bass.x_over),
                    level * 10
                ),
            );
        }
    }
    if let Some(svm) = &settings.svm {
        let enabled = master.svm_enable.unwrap_or(svm.enable);
        let level = percent("SVM.Level", svm.level)?;
        controls.insert(
            "FX: Smart Volume".to_owned(),
            ProfileControl {
                playback_switch: Some(enabled),
                playback_level: Some(level),
                ..ProfileControl::default()
            },
        );
        report_mapping(
            report,
            is_exact_step(svm.level * 100.0),
            format!(
                "SVM → FX: Smart Volume (playback {}, level {level})",
                on_off(enabled)
            ),
        );
        let mode = svm_mode(svm.mode)?;
        controls.insert(
            "FX: Smart Volume Setting".to_owned(),
            ProfileControl {
                choice: Some(mode.to_owned()),
                ..ProfileControl::default()
            },
        );
        report.exact.push(format!(
            "SVM.Mode {} → FX: Smart Volume Setting ({mode})",
            svm.mode
        ));
        report_ae5_default(report, "SVM.PlusMode", svm.plus_mode, "Katana-only mode");
        report_unknown_fields(report, "SVM", &svm.extra, false);
    }
    add_effect(
        &mut controls,
        "FX: Dialog Plus",
        "DialogPlus",
        settings.dialog_plus.as_ref(),
        master.dialog_plus_enable,
        report,
    )?;
    Ok(controls)
}

fn add_effect(
    controls: &mut BTreeMap<String, ProfileControl>,
    control_name: &str,
    source_name: &str,
    effect: Option<&SourceEffect>,
    master_enabled: Option<bool>,
    report: &mut SbCommandImportReport,
) -> Result<(), SbCommandError> {
    if let Some(effect) = effect {
        let enabled = master_enabled.unwrap_or(effect.enable);
        let level = percent(&format!("{source_name}.Level"), effect.level)?;
        controls.insert(
            control_name.to_owned(),
            ProfileControl {
                playback_switch: Some(enabled),
                playback_level: Some(level),
                ..ProfileControl::default()
            },
        );
        report_mapping(
            report,
            is_exact_step(effect.level * 100.0),
            format!(
                "{source_name} → {control_name} (playback {}, level {level})",
                on_off(enabled)
            ),
        );
        if matches!(source_name, "Surround" | "DialogPlus") {
            report_ae5_default(
                report,
                &format!("{source_name}.Mode"),
                effect.mode,
                "Katana-only mode",
            );
        } else if let Some(mode) = effect.mode {
            report.unsupported.push(format!(
                "{source_name}.Mode {mode} (no mapped AE-5 ALSA control)"
            ));
        }
        report_unknown_fields(report, source_name, &effect.extra, false);
    }
    Ok(())
}

fn add_eq_controls(
    controls: &mut BTreeMap<String, ProfileControl>,
    eq: &SourceEq,
    target: SbCommandTarget,
    report: &mut SbCommandImportReport,
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
    report_unknown_fields(report, "Equalizer.Settings", &settings.extra, false);
    if settings.pre_amp.abs() > 0.01 {
        report.unsupported.push(format!(
            "Equalizer.PreAmp {} dB (no equivalent AE-5 ALSA control)",
            source_number(settings.pre_amp)
        ));
    } else {
        report
            .exact
            .push("Equalizer.PreAmp 0 dB → no gain adjustment".to_owned());
    }
    controls.insert(
        "FX: Equalizer Preset".to_owned(),
        ProfileControl {
            choice: Some("Flat".to_owned()),
            ..ProfileControl::default()
        },
    );
    report
        .exact
        .push("Equalizer custom bands → FX: Equalizer Preset Flat".to_owned());
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
        let control_name = format!("EQ Band{index}");
        let level = eq_level(index, band.value)?;
        let target_db = level - 24;
        controls.insert(
            control_name.clone(),
            ProfileControl {
                playback_level: Some(level),
                ..ProfileControl::default()
            },
        );
        report_mapping(
            report,
            is_exact_step(band.value),
            format!(
                "EQ {} Hz {} dB → {control_name} ({target_db:+} dB)",
                band.frequency,
                source_number(band.value)
            ),
        );
        report_unknown_fields(
            report,
            &format!("Equalizer.Bands[{index}]"),
            &band.extra,
            false,
        );
    }
    Ok(())
}

fn report_mapping(report: &mut SbCommandImportReport, exact: bool, message: String) {
    if exact {
        report.exact.push(message);
    } else {
        report
            .approximate
            .push(format!("{message}; rounded to ALSA step"));
    }
}

fn report_ae5_default(
    report: &mut SbCommandImportReport,
    field: &str,
    value: Option<u8>,
    meaning: &str,
) {
    match value {
        Some(0) => report
            .exact
            .push(format!("{field} 0 → {meaning}; no Linux control required")),
        Some(value) => report.unsupported.push(format!(
            "{field} {value} (unexpected non-default value; no mapped AE-5 ALSA control)"
        )),
        None => {}
    }
}

fn report_unknown_fields(
    report: &mut SbCommandImportReport,
    prefix: &str,
    fields: &BTreeMap<String, serde_json::Value>,
    ignore_metadata: bool,
) {
    report.unsupported.extend(
        fields
            .iter()
            .filter(|(_, value)| !value.is_null())
            .filter(|(name, _)| {
                !ignore_metadata || !SOURCE_METADATA_FIELDS.contains(&name.as_str())
            })
            .map(|(name, _)| format!("{prefix}.{name} (no mapped AE-5 ALSA control)")),
    );
}

fn is_exact_step(value: f64) -> bool {
    (value - value.round()).abs() <= 0.0001
}

fn on_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

fn source_number(value: f64) -> String {
    let value = format!("{value:.6}");
    value.trim_end_matches('0').trim_end_matches('.').to_owned()
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

fn add_speaker_layout(
    config: &str,
    controls: &mut BTreeMap<String, ProfileControl>,
    report: &mut SbCommandImportReport,
) -> Result<(), SbCommandError> {
    let mask = required_user_setting(config, "SelectedSpeakerChannelMask")?;
    let mask = mask
        .parse::<u32>()
        .map_err(|_| invalid("SelectedSpeakerChannelMask is not an unsigned integer"))?;
    if let Some(layout) = speaker_layout(mask) {
        controls.insert(
            "Surround Channel Config".to_owned(),
            ProfileControl {
                choice: Some(layout.to_owned()),
                ..ProfileControl::default()
            },
        );
        report.exact.push(format!(
            "SelectedSpeakerChannelMask {mask} → Surround Channel Config ({layout})"
        ));
    } else {
        report.unsupported.push(format!(
            "SelectedSpeakerChannelMask {mask} (unknown AE-5 speaker layout)"
        ));
    }
    if let Some(speaker_type) = user_setting(config, "SelectedSpeakerType")?
        && !speaker_type.is_empty()
    {
        if speaker_type.eq_ignore_ascii_case("Desktop") {
            report.exact.push(
                "SelectedSpeakerType Desktop → crossover semantics represented by Bass.XOver"
                    .to_owned(),
            );
        } else {
            report.unsupported.push(format!(
                "SelectedSpeakerType {speaker_type} (no mapped AE-5 ALSA control)"
            ));
        }
    }
    Ok(())
}

fn report_headphone_tuning(
    config: &str,
    user_config_path: &Path,
    product_dir: &Path,
    report: &mut SbCommandImportReport,
) -> Result<(), SbCommandError> {
    if let Some(tuning) = user_setting(config, "SelectedHpEq")? {
        if tuning.eq_ignore_ascii_case("HPNONE") || tuning.is_empty() {
            report
                .exact
                .push("SelectedHpEq → no Creative headphone tuning".to_owned());
        } else {
            validate_identifier("SelectedHpEq", &tuning)?;
            let file_name = format!("{tuning}.cfg");
            let user_config = product_dir.join("SpeakerEqConfigs").join(&file_name);
            let installed_config = user_config_path
                .ancestors()
                .find(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.eq_ignore_ascii_case("Users"))
                })
                .and_then(Path::parent)
                .map(|windows_root| {
                    windows_root
                        .join("ProgramData/Creative/SBCommand/Product/AE5/SpeakerEqConfigs")
                        .join(file_name)
                });
            let config_path = std::iter::once(user_config)
                .chain(installed_config)
                .find(|path| is_regular_file(path));
            let model = if let Some(config_path) = config_path {
                load_text(&config_path)?
                    .lines()
                    .find_map(|line| line.strip_prefix("model "))
                    .filter(|model| {
                        !model.is_empty()
                            && model.len() <= 120
                            && !model.chars().any(char::is_control)
                    })
                    .map(str::to_owned)
            } else {
                None
            };
            report.unsupported.push(format!(
                "SelectedHpEq {tuning}{} → Creative driver/APO tuning (no mapped AE-5 ALSA control)",
                model.map_or_else(String::new, |model| format!(" ({model})"))
            ));
        }
    }
    Ok(())
}

fn map_lfe_bass_management(
    controls: &mut BTreeMap<String, ProfileControl>,
    report: &mut SbCommandImportReport,
) {
    let layout = controls
        .get("Surround Channel Config")
        .and_then(|control| control.choice.clone());
    let Some(x_bass_enabled) = map_lfe_bass_controls(controls, layout.as_deref()) else {
        return;
    };

    for mappings in [&mut report.exact, &mut report.approximate] {
        mappings.retain(|item| !item.starts_with("Bass → FX: X-Bass"));
        for item in mappings
            .iter_mut()
            .filter(|item| item.starts_with("Bass.XOver "))
        {
            *item = item.replace("FX: X-Bass Crossover", "Bass Redirection Crossover");
        }
    }
    report.exact.push(format!(
        "Bass → Bass Redirection (playback {}; X-Bass off and strength inactive for an LFE speaker layout)",
        on_off(x_bass_enabled)
    ));
}

pub(crate) fn map_lfe_bass_controls(
    controls: &mut BTreeMap<String, ProfileControl>,
    layout: Option<&str>,
) -> Option<bool> {
    if !layout.is_some_and(|layout| layout.ends_with(".1")) {
        return None;
    }
    let x_bass = controls.get("FX: X-Bass")?;
    let x_bass_enabled = x_bass.playback_switch.unwrap_or(false);

    controls.insert(
        "FX: X-Bass".to_owned(),
        ProfileControl {
            playback_switch: Some(false),
            ..ProfileControl::default()
        },
    );
    controls.insert(
        "Bass Redirection".to_owned(),
        ProfileControl {
            playback_switch: Some(x_bass_enabled),
            ..ProfileControl::default()
        },
    );
    if let Some(crossover) = controls.remove("FX: X-Bass Crossover") {
        controls.insert("Bass Redirection Crossover".to_owned(), crossover);
    }
    Some(x_bass_enabled)
}

fn speaker_layout(mask: u32) -> Option<&'static str> {
    match mask {
        3 => Some("2.0"),
        11 => Some("2.1"),
        51 => Some("4.0"),
        59 => Some("4.1"),
        63 => Some("5.1"),
        _ => None,
    }
}

fn required_user_setting(config: &str, name: &str) -> Result<String, SbCommandError> {
    user_setting(config, name)?
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid(format!("user.config is missing string setting '{name}'")))
}

fn user_setting(config: &str, name: &str) -> Result<Option<String>, SbCommandError> {
    let marker = format!(r#"<setting name="{name}""#);
    let Some(start) = config.find(&marker) else {
        return Ok(None);
    };
    if config[start + marker.len()..].contains(&marker) {
        return Err(invalid(format!(
            "user.config contains duplicate setting '{name}'"
        )));
    }
    let setting = &config[start..];
    let tag_end = setting
        .find('>')
        .ok_or_else(|| invalid(format!("setting '{name}' has no closing start tag")))?;
    if !setting[..tag_end].contains(r#"serializeAs="String""#) {
        return Err(invalid(format!("setting '{name}' is not a plain string")));
    }
    let body = &setting[tag_end + 1..];
    let body_end = body
        .find("</setting>")
        .ok_or_else(|| invalid(format!("setting '{name}' has no closing tag")))?;
    let body = &body[..body_end];
    let value_start = body
        .find("<value>")
        .ok_or_else(|| invalid(format!("setting '{name}' has no value")))?;
    let value = &body[value_start + "<value>".len()..];
    let value_end = value
        .find("</value>")
        .ok_or_else(|| invalid(format!("setting '{name}' has no closing value tag")))?;
    let value = value[..value_end].trim();
    if value.contains(['&', '<', '>']) {
        return Err(invalid(format!(
            "setting '{name}' contains unsupported XML markup"
        )));
    }
    Ok(Some(value.to_owned()))
}

fn validate_identifier(name: &str, value: &str) -> Result<(), SbCommandError> {
    if value.len() > 80
        || value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(invalid(format!(
            "setting '{name}' is not a safe identifier"
        )));
    }
    Ok(())
}

fn load_text(path: &Path) -> Result<String, SbCommandError> {
    let contents = load_bytes(path)?;
    String::from_utf8(contents)
        .map_err(|_| invalid(format!("'{}' is not valid UTF-8", path.display())))
}

fn load_json<T: DeserializeOwned>(path: &Path) -> Result<T, SbCommandError> {
    serde_json::from_slice(&load_bytes(path)?).map_err(Into::into)
}

fn load_bytes(path: &Path) -> Result<Vec<u8>, SbCommandError> {
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
    Ok(contents)
}

fn subdirectories(path: &Path) -> Result<Vec<PathBuf>, SbCommandError> {
    let mut directories = Vec::new();
    for (index, entry) in fs::read_dir(path)?.enumerate() {
        if index >= MAX_DISCOVERY_ENTRIES {
            return Err(invalid(format!(
                "'{}' contains too many entries to scan safely",
                path.display()
            )));
        }
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            directories.push(entry.path());
        }
    }
    directories.sort();
    Ok(directories)
}

fn command_version(path: &Path) -> Option<Vec<u64>> {
    let components = path
        .file_name()?
        .to_str()?
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<u64>, _>>()
        .ok()?;
    (components.len() >= 2).then_some(components)
}

fn active_command_version(user_config_path: &Path) -> Option<&str> {
    let version_dir = user_config_path.parent()?;
    command_version(version_dir)?;
    version_dir.file_name()?.to_str()
}

fn discover_driver_version(windows_user_dir: &Path) -> Result<Option<String>, SbCommandError> {
    let Some(users_dir) = windows_user_dir.parent() else {
        return Ok(None);
    };
    if !users_dir
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("Users"))
    {
        return Ok(None);
    }
    let Some(windows_root) = users_dir.parent() else {
        return Ok(None);
    };
    let active_driver = windows_root.join("Windows/System32/drivers/CtxHda.sys");
    let repository = windows_root.join("Windows/System32/DriverStore/FileRepository");
    if !is_regular_file(&active_driver) || !is_directory(&repository) {
        return Ok(None);
    }

    let mut versions = Vec::new();
    for package in driver_packages(&repository)? {
        let packaged_driver = package.join("AMD64/CtxHda.sys");
        let inf_path = package.join("ctxhda.inf");
        if !is_regular_file(&packaged_driver)
            || !is_regular_file(&inf_path)
            || !files_equal(&active_driver, &packaged_driver)?
        {
            continue;
        }
        let inf = load_text(&inf_path)?;
        if !inf.lines().any(|line| {
            line.to_ascii_uppercase()
                .contains("PCI\\VEN_1102&DEV_0012&SUBSYS_00511102")
        }) {
            continue;
        }
        if let Some(version) = driver_version_from_inf(&inf) {
            versions.push(version.to_owned());
        }
    }
    versions.sort();
    versions.dedup();
    match versions.as_slice() {
        [] => Ok(None),
        [version] => Ok(Some(version.clone())),
        _ => Err(invalid(
            "multiple installed Creative packages match the active AE-5 driver",
        )),
    }
}

fn driver_packages(repository: &Path) -> Result<Vec<PathBuf>, SbCommandError> {
    let mut packages = Vec::new();
    for (index, entry) in fs::read_dir(repository)?.enumerate() {
        if index >= MAX_DRIVERSTORE_ENTRIES {
            return Err(invalid(format!(
                "'{}' contains too many entries to scan safely",
                repository.display()
            )));
        }
        let entry = entry?;
        if !entry.file_type()?.is_dir()
            || !entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.to_ascii_lowercase().starts_with("ctxhda.inf_amd64_"))
        {
            continue;
        }
        if packages.len() >= MAX_DRIVER_PACKAGES {
            return Err(invalid(format!(
                "'{}' contains too many Creative driver packages",
                repository.display()
            )));
        }
        packages.push(entry.path());
    }
    packages.sort();
    Ok(packages)
}

fn driver_version_from_inf(inf: &str) -> Option<&str> {
    let value = inf.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        key.trim()
            .eq_ignore_ascii_case("DriverVer")
            .then_some(value.trim())
    })?;
    let (_, version) = value.split_once(',')?;
    let version = version.trim();
    let components = version.split('.').collect::<Vec<_>>();
    (components.len() >= 2
        && components.iter().all(|component| {
            !component.is_empty() && component.bytes().all(|byte| byte.is_ascii_digit())
        }))
    .then_some(version)
}

fn files_equal(left: &Path, right: &Path) -> Result<bool, io::Error> {
    let mut left = fs::File::open(left)?;
    let mut right = fs::File::open(right)?;
    let length = left.metadata()?.len();
    if length != right.metadata()?.len() {
        return Ok(false);
    }

    let mut left_buffer = [0_u8; 64 * 1024];
    let mut right_buffer = [0_u8; 64 * 1024];
    let mut remaining = length;
    while remaining > 0 {
        let amount = usize::try_from(remaining.min(left_buffer.len() as u64))
            .expect("bounded comparison chunk fits usize");
        left.read_exact(&mut left_buffer[..amount])?;
        right.read_exact(&mut right_buffer[..amount])?;
        if left_buffer[..amount] != right_buffer[..amount] {
            return Ok(false);
        }
        remaining -= amount as u64;
    }
    Ok(true)
}

fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_file())
}

fn is_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.file_type().is_dir())
}

fn invalid(message: impl Into<String>) -> SbCommandError {
    SbCommandError::Invalid(message.into())
}

impl SbCommandTarget {
    fn profile_type(self) -> u8 {
        match self {
            Self::Speaker => 1,
            Self::Headphone => 0,
        }
    }

    fn eq_type(self) -> &'static str {
        match self {
            Self::Speaker => "Speaker",
            Self::Headphone => "Headphone",
        }
    }

    fn output_choice(self) -> &'static str {
        match self {
            Self::Speaker => "Speakers",
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
    speaker_method: Option<u8>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceSettings {
    #[serde(rename = "Type")]
    kind: u8,
    scout: Option<SourceScout>,
    surround: Option<SourceEffect>,
    crystalizer: Option<SourceEffect>,
    bass: Option<SourceBass>,
    #[serde(rename = "SVM")]
    svm: Option<SourceSvm>,
    dialog_plus: Option<SourceEffect>,
    #[serde(rename = "SBXMaster")]
    sbx_master: Option<SourceMaster>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceEffect {
    enable: bool,
    level: f64,
    mode: Option<u8>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceBass {
    enable: bool,
    level: f64,
    #[serde(rename = "XOver")]
    x_over: f64,
    sub_woofer_gain: Option<bool>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceScout {
    enable: Option<bool>,
    surround_enable: Option<bool>,
    crystalizer_enable: Option<bool>,
    #[serde(rename = "XBassEnable")]
    x_bass_enable: Option<bool>,
    #[serde(rename = "SVMEnable")]
    svm_enable: Option<bool>,
    dialog_plus_enable: Option<bool>,
    #[serde(rename = "GraphicsEQEnable")]
    graphics_eq_enable: Option<bool>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceSvm {
    enable: bool,
    level: f64,
    mode: u8,
    plus_mode: Option<u8>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
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
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceEq {
    product: String,
    settings: Vec<SourceEqSettings>,
    speaker_method: Option<u8>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceEqSettings {
    #[serde(rename = "Type")]
    kind: String,
    unit: String,
    pre_amp: f64,
    bands: Vec<SourceBand>,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SourceBand {
    frequency: u32,
    value: f64,
    #[serde(flatten)]
    extra: BTreeMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

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
                    "Type":0,
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
        let controls = effect_controls(settings, &mut SbCommandImportReport::default()).unwrap();

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

    #[test]
    fn uses_creatives_reflected_profile_output_type_values() {
        assert_eq!(SbCommandTarget::Headphone.profile_type(), 0);
        assert_eq!(SbCommandTarget::Speaker.profile_type(), 1);

        let profile: SourceProfile = serde_json::from_str(
            r#"{
                "Product":"AE5",
                "Settings":[
                    {"Type":0,"SBXMaster":{"Enable":true}},
                    {"Type":1,"SBXMaster":{"Enable":false}}
                ]
            }"#,
        )
        .unwrap();
        assert_eq!(
            select_settings(&profile.settings, SbCommandTarget::Headphone)
                .unwrap()
                .kind,
            0
        );
        assert_eq!(
            select_settings(&profile.settings, SbCommandTarget::Speaker)
                .unwrap()
                .kind,
            1
        );
    }

    #[test]
    fn separates_rounded_and_inactive_windows_settings() {
        let profile: SourceProfile = serde_json::from_str(
            r#"{
                "Product":"AE5",
                "SpeakerMethod":0,
                "Settings":[{
                    "Type":0,
                    "Scout":{
                        "Enable":false,
                        "SurroundEnable":false,
                        "CrystalizerEnable":false,
                        "XBassEnable":false,
                        "SVMEnable":false,
                        "DialogPlusEnable":false,
                        "GraphicsEQEnable":false
                    },
                    "Surround":{"Enable":true,"Level":0.675,"Mode":0},
                    "Bass":{
                        "Enable":false,
                        "Level":0.0,
                        "XOver":0.0,
                        "SubWooferGain":false
                    },
                    "SVM":{"Enable":true,"Level":0.5,"Mode":0,"PlusMode":0},
                    "DialogPlus":{"Enable":true,"Level":0.5,"Mode":0},
                    "SBXMaster":{"Enable":true,"SurroundEnable":false}
                }]
            }"#,
        )
        .unwrap();
        let settings = select_settings(&profile.settings, SbCommandTarget::Headphone).unwrap();
        let mut report = SbCommandImportReport::default();
        report_unknown_fields(&mut report, "Profile", &profile.extra, true);
        report_ae5_default(
            &mut report,
            "Profile.SpeakerMethod",
            profile.speaker_method,
            "AE-5 profile routing metadata",
        );
        effect_controls(settings, &mut report).unwrap();

        assert!(
            report
                .approximate
                .iter()
                .any(|item| item.contains("FX: Surround") && item.contains("rounded"))
        );
        assert!(
            report
                .exact
                .iter()
                .any(|item| item.contains("Profile.SpeakerMethod"))
        );
        assert!(
            report
                .exact
                .iter()
                .any(|item| item.contains("Surround.Mode"))
        );
        assert!(
            report
                .exact
                .iter()
                .any(|item| item.contains("DialogPlus.Mode"))
        );
        assert!(
            report
                .exact
                .iter()
                .any(|item| item.contains("SVM.PlusMode"))
        );
        assert!(
            report
                .unsupported
                .iter()
                .all(|item| !item.contains("Scout") && !item.contains("Bass.SubWooferGain"))
        );
        assert!(
            report
                .exact
                .iter()
                .any(|item| item.contains("Scout disabled"))
        );
        assert!(
            report
                .exact
                .iter()
                .any(|item| item.contains("Bass.SubWooferGain off"))
        );
    }

    #[test]
    fn retains_nondefault_metadata_and_product_specific_modes_as_unsupported() {
        let profile: SourceProfile = serde_json::from_str(
            r#"{
                "Product":"AE5",
                "SpeakerMethod":1,
                "Settings":[{
                    "Type":0,
                    "Surround":{"Enable":true,"Level":0.5,"Mode":1},
                    "Crystalizer":{"Enable":true,"Level":0.5,"Mode":1},
                    "SVM":{"Enable":true,"Level":0.5,"Mode":0,"PlusMode":1},
                    "DialogPlus":{"Enable":true,"Level":0.5,"Mode":1},
                    "SBXMaster":{"Enable":true}
                }]
            }"#,
        )
        .unwrap();
        let settings = select_settings(&profile.settings, SbCommandTarget::Headphone).unwrap();
        let eq: SourceEq =
            serde_json::from_str(r#"{"Product":"AE5","SpeakerMethod":1,"Settings":[]}"#).unwrap();
        let mut report = SbCommandImportReport::default();
        report_ae5_default(
            &mut report,
            "Profile.SpeakerMethod",
            profile.speaker_method,
            "AE-5 profile routing metadata",
        );
        report_ae5_default(
            &mut report,
            "Equalizer.SpeakerMethod",
            eq.speaker_method,
            "AE-5 equalizer routing metadata",
        );

        effect_controls(settings, &mut report).unwrap();

        for field in [
            "Profile.SpeakerMethod 1",
            "Equalizer.SpeakerMethod 1",
            "Surround.Mode 1",
            "Crystalizer.Mode 1",
            "SVM.PlusMode 1",
            "DialogPlus.Mode 1",
        ] {
            assert!(
                report.unsupported.iter().any(|item| item.contains(field)),
                "missing unsupported report for {field}"
            );
        }
    }

    #[test]
    fn retains_configured_scout_and_subwoofer_gain_as_unsupported() {
        let profile: SourceProfile = serde_json::from_str(
            r#"{
                "Product":"AE5",
                "Settings":[{
                    "Type":0,
                    "Scout":{"Enable":true},
                    "Bass":{
                        "Enable":false,
                        "Level":0.0,
                        "XOver":0.0,
                        "SubWooferGain":true
                    },
                    "SBXMaster":{"Enable":true}
                }]
            }"#,
        )
        .unwrap();
        let settings = select_settings(&profile.settings, SbCommandTarget::Headphone).unwrap();
        let mut report = SbCommandImportReport::default();

        effect_controls(settings, &mut report).unwrap();

        assert!(
            report
                .unsupported
                .iter()
                .any(|item| item.contains("Settings.Scout enabled or configured"))
        );
        assert!(
            report
                .unsupported
                .iter()
                .any(|item| item.contains("Bass.SubWooferGain on"))
        );
    }

    #[test]
    fn preserves_nonzero_eq_preamp_as_an_unsupported_report_item() {
        let eq: SourceEq = serde_json::from_str(
            r#"{
                "Product":"AE5",
                "Settings":[{
                    "Type":"Headphone",
                    "Unit":"db",
                    "PreAmp":2.5,
                    "Bands":[
                        {"Frequency":31,"Value":8.9},
                        {"Frequency":62,"Value":0.0},
                        {"Frequency":125,"Value":0.0},
                        {"Frequency":250,"Value":0.0},
                        {"Frequency":500,"Value":0.0},
                        {"Frequency":1000,"Value":0.0},
                        {"Frequency":2000,"Value":0.0},
                        {"Frequency":4000,"Value":0.0},
                        {"Frequency":8000,"Value":0.0},
                        {"Frequency":16000,"Value":0.0}
                    ]
                }]
            }"#,
        )
        .unwrap();
        let mut controls = BTreeMap::new();
        let mut report = SbCommandImportReport::default();

        add_eq_controls(&mut controls, &eq, SbCommandTarget::Headphone, &mut report).unwrap();

        assert_eq!(
            controls["FX: Equalizer Preset"].choice.as_deref(),
            Some("Flat")
        );
        assert_eq!(controls["EQ Band0"].playback_level, Some(33));
        assert!(
            report
                .unsupported
                .iter()
                .any(|item| item.contains("PreAmp 2.5 dB"))
        );
        assert!(
            report
                .approximate
                .iter()
                .any(|item| item.contains("EQ 31 Hz 8.9 dB"))
        );
    }

    #[test]
    fn reads_only_plain_unique_user_settings() {
        let config = r#"
            <setting name="SelectedHpEq" serializeAs="String">
                <value>ATHM50</value>
            </setting>
            <setting name="LastAEStates" serializeAs="Binary">
                <value>AAECAw==</value>
            </setting>
        "#;

        assert_eq!(
            user_setting(config, "SelectedHpEq").unwrap().as_deref(),
            Some("ATHM50")
        );
        assert!(user_setting(config, "LastAEStates").is_err());
        assert_eq!(user_setting(config, "Missing").unwrap(), None);
    }

    #[test]
    fn rejects_unsafe_or_ambiguous_active_profile_identifiers() {
        assert!(validate_identifier("profile", "My_Profile-1").is_ok());
        assert!(validate_identifier("profile", "../Profiles/other").is_err());
        assert!(validate_identifier("profile", "profile.json").is_err());

        let duplicate = r#"
            <setting name="SelectedHpEq" serializeAs="String"><value>A</value></setting>
            <setting name="SelectedHpEq" serializeAs="String"><value>B</value></setting>
        "#;
        assert!(user_setting(duplicate, "SelectedHpEq").is_err());
    }

    #[test]
    fn maps_known_windows_speaker_masks_without_guessing_unknown_masks() {
        assert_eq!(speaker_layout(3), Some("2.0"));
        assert_eq!(speaker_layout(11), Some("2.1"));
        assert_eq!(speaker_layout(51), Some("4.0"));
        assert_eq!(speaker_layout(59), Some("4.1"));
        assert_eq!(speaker_layout(63), Some("5.1"));
        assert_eq!(speaker_layout(7), None);
    }

    #[test]
    fn separates_speaker_type_metadata_from_the_imported_crossover() {
        let desktop = r#"
            <setting name="SelectedSpeakerChannelMask" serializeAs="String">
                <value>63</value>
            </setting>
            <setting name="SelectedSpeakerType" serializeAs="String">
                <value>Desktop</value>
            </setting>
        "#;
        let mut controls = BTreeMap::new();
        let mut report = SbCommandImportReport::default();

        add_speaker_layout(desktop, &mut controls, &mut report).unwrap();

        assert_eq!(
            controls["Surround Channel Config"].choice.as_deref(),
            Some("5.1")
        );
        assert!(
            report
                .exact
                .iter()
                .any(|item| item.contains("crossover semantics represented by Bass.XOver"))
        );
        assert!(
            report
                .unsupported
                .iter()
                .all(|item| !item.contains("SelectedSpeakerType"))
        );

        let mut report = SbCommandImportReport::default();
        add_speaker_layout(
            &desktop.replace("Desktop", "Tower"),
            &mut BTreeMap::new(),
            &mut report,
        )
        .unwrap();
        assert!(
            report
                .unsupported
                .iter()
                .any(|item| item.contains("SelectedSpeakerType Tower"))
        );
    }

    #[test]
    fn resolves_headphone_tuning_display_metadata_without_mapping_the_effect() {
        let root = std::env::temp_dir().join(format!(
            "ae5-speaker-eq-config-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let user_config = root.join("Users/Max/AppData/Local/Creative/user.config");
        let product = root.join("Users/Max/AppData/Local/Creative/Product/AE5");
        let configs = root.join("ProgramData/Creative/SBCommand/Product/AE5/SpeakerEqConfigs");
        fs::create_dir_all(&configs).unwrap();
        fs::write(
            configs.join("EXAMPLE.cfg"),
            "model Example Headphones\r\norder 50\r\n",
        )
        .unwrap();
        let config = r#"
            <setting name="SelectedHpEq" serializeAs="String">
                <value>EXAMPLE</value>
            </setting>
        "#;
        let mut report = SbCommandImportReport::default();

        report_headphone_tuning(config, &user_config, &product, &mut report).unwrap();

        assert!(report.unsupported.iter().any(|item| {
            item.contains("SelectedHpEq EXAMPLE (Example Headphones) → Creative driver/APO tuning")
        }));
        assert!(
            report_headphone_tuning(
                &config.replace("EXAMPLE", "../escape"),
                &user_config,
                &product,
                &mut SbCommandImportReport::default()
            )
            .is_err()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn maps_enabled_lfe_speaker_bass_to_bass_management() {
        let mut controls = BTreeMap::from([
            (
                "Surround Channel Config".to_owned(),
                ProfileControl {
                    choice: Some("5.1".to_owned()),
                    ..ProfileControl::default()
                },
            ),
            (
                "FX: X-Bass".to_owned(),
                ProfileControl {
                    playback_switch: Some(true),
                    playback_level: Some(53),
                    ..ProfileControl::default()
                },
            ),
            (
                "FX: X-Bass Crossover".to_owned(),
                ProfileControl {
                    playback_level: Some(8),
                    ..ProfileControl::default()
                },
            ),
        ]);
        let mut report = SbCommandImportReport {
            exact: vec!["Bass.XOver 80 Hz → FX: X-Bass Crossover (80 Hz)".to_owned()],
            approximate: vec![
                "Bass → FX: X-Bass (playback on, level 53); rounded to ALSA step".to_owned(),
            ],
            ..SbCommandImportReport::default()
        };

        map_lfe_bass_management(&mut controls, &mut report);

        assert_eq!(
            controls["FX: X-Bass"],
            ProfileControl {
                playback_switch: Some(false),
                ..ProfileControl::default()
            }
        );
        assert_eq!(
            controls["Bass Redirection"],
            ProfileControl {
                playback_switch: Some(true),
                ..ProfileControl::default()
            }
        );
        assert_eq!(
            controls["Bass Redirection Crossover"],
            ProfileControl {
                playback_level: Some(8),
                ..ProfileControl::default()
            }
        );
        assert!(!controls.contains_key("FX: X-Bass Crossover"));
        assert!(
            report
                .exact
                .iter()
                .any(|item| item.contains("Bass.XOver 80 Hz → Bass Redirection Crossover"))
        );
        assert!(report.exact.iter().any(|item| {
            item.contains("Bass → Bass Redirection (playback on; X-Bass off and strength inactive")
        }));
        assert!(report.approximate.is_empty());
        assert!(report.unsupported.is_empty());
    }

    #[test]
    fn discovers_newest_command_config_and_rejects_ambiguous_ae5_products() {
        let user = std::env::temp_dir().join(format!(
            "ae5-sbcommand-discovery-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let application = user
            .join("AppData/Local/Creative_Technology_Ltd")
            .join("Creative.SBCommand.exe_Url_test");
        let old_config = application.join("3.9.99.0/user.config");
        let newest_config = application.join("3.10.0.0/user.config");
        fs::create_dir_all(old_config.parent().unwrap()).unwrap();
        fs::create_dir_all(newest_config.parent().unwrap()).unwrap();
        fs::write(&old_config, "").unwrap();
        fs::write(&newest_config, "").unwrap();

        let product = user
            .join("AppData/Local/Creative/installation-one")
            .join("Product/AE5");
        fs::create_dir_all(&product).unwrap();
        assert_eq!(
            discover_installation(&user).unwrap(),
            SbCommandInstallation {
                user_config: newest_config.clone(),
                product_dir: product,
                driver_version: None,
            }
        );
        assert_eq!(active_command_version(&newest_config), Some("3.10.0.0"));
        assert_eq!(
            active_command_version(Path::new("/tmp/manual/user.config")),
            None
        );

        let duplicate_application = user
            .join("AppData/Local/Creative_Technology_Ltd")
            .join("Creative.SBCommand.exe_Url_duplicate")
            .join("3.10.0.0");
        fs::create_dir_all(&duplicate_application).unwrap();
        fs::write(duplicate_application.join("user.config"), "").unwrap();
        assert!(discover_installation(&user).is_err());
        fs::remove_dir_all(
            duplicate_application
                .parent()
                .expect("duplicate application has a parent"),
        )
        .unwrap();

        fs::create_dir_all(
            user.join("AppData/Local/Creative/installation-two")
                .join("Product/AE5"),
        )
        .unwrap();
        assert!(discover_installation(&user).is_err());
        fs::remove_dir_all(user).unwrap();
    }

    #[test]
    fn discovers_the_active_ae5_driver_from_the_installed_binary() {
        let root = std::env::temp_dir().join(format!(
            "ae5-driver-discovery-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        let user = root.join("Users/Max");
        let active_driver = root.join("Windows/System32/drivers/CtxHda.sys");
        let active_package =
            root.join("Windows/System32/DriverStore/FileRepository/ctxhda.inf_amd64_active");
        let old_package =
            root.join("Windows/System32/DriverStore/FileRepository/ctxhda.inf_amd64_old");
        fs::create_dir_all(&user).unwrap();
        fs::create_dir_all(active_driver.parent().unwrap()).unwrap();
        fs::create_dir_all(active_package.join("AMD64")).unwrap();
        fs::create_dir_all(old_package.join("AMD64")).unwrap();
        fs::write(&active_driver, b"active driver").unwrap();
        fs::write(active_package.join("AMD64/CtxHda.sys"), b"active driver").unwrap();
        fs::write(old_package.join("AMD64/CtxHda.sys"), b"older driver").unwrap();
        fs::write(
            active_package.join("ctxhda.inf"),
            "DriverVer=11/24/2022, 6.0.105.0065\nPCI\\VEN_1102&DEV_0012&SUBSYS_00511102\n",
        )
        .unwrap();
        fs::write(
            old_package.join("ctxhda.inf"),
            "DriverVer=02/24/2022, 6.0.105.0064\nPCI\\VEN_1102&DEV_0012&SUBSYS_00511102\n",
        )
        .unwrap();

        assert_eq!(
            discover_driver_version(&user).unwrap(),
            Some("6.0.105.0065".to_owned())
        );
        assert_eq!(driver_version_from_inf("DriverVer=invalid"), None);
        fs::remove_dir_all(root).unwrap();
    }
}
