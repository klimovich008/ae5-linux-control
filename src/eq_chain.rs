use crate::{ControlSnapshot, Profile};
use std::env;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const EQ_FREQUENCIES: [u32; 10] = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];
const EQ_Q: f64 = 1.4;
const PIPEWIRE_MAX_GAIN_DB: f64 = 20.0;
const PIPEWIRE_MIN_GAIN_DB: f64 = -120.0;
const PIPEWIRE_RATES: [f64; 3] = [44_100.0, 48_000.0, 96_000.0];
const RESPONSE_STEPS: usize = 65_536;
const LEGACY_HEADROOM_MARGIN_DB: f64 = 0.25;
const CONFIG_FILE: &str = "software-eq.state";
const MANAGED_HEADER: &str = "# Managed by AE-5 Control.\n";
const FORMAT_LINE: &str = "# Format: direct-filter-v2\n";
const LEGACY_FORMAT_LINE: &str = "# Format: direct-filter-v1\n";
const GAINS_PREFIX: &str = "# EQ gains (dB): [ ";
const TARGET_PREFIX: &str = "# Target: ";
const PREAMP_PREFIX: &str = "# Preamp (dB): ";
const LEGACY_PREAMP_PREFIX: &str = "# Automatic preamp (dB): ";
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq)]
pub struct EqBand {
    pub frequency: u32,
    pub q: f64,
    pub gain_db: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EqChainConfig {
    pub path: PathBuf,
    pub enabled: bool,
    pub bands: Vec<EqBand>,
    pub target_node: Option<String>,
    pub preamp_db: f64,
}

impl EqChainConfig {
    pub fn signature(&self) -> Option<String> {
        if !self.enabled {
            return None;
        }
        self.target_node.as_deref().map(|target| {
            if self.preamp_db == 0.0 {
                eq_chain_signature(&self.bands, target)
            } else {
                legacy_eq_chain_signature(&self.bands, target, self.preamp_db)
            }
        })
    }

    pub fn filter_graph(&self) -> Result<Option<String>, EqChainError> {
        if !self.enabled {
            return Ok(None);
        }
        let graph = if self.preamp_db == 0.0 {
            render_filter_graph(&self.bands)?
        } else {
            render_legacy_filter_graph(&self.bands, self.preamp_db)?
        };
        Ok(Some(graph))
    }

    pub fn expected_responses_db(&self, sample_rate: u32) -> Result<[f64; 10], EqChainError> {
        self.filter_graph()?.ok_or_else(|| {
            EqChainError::Invalid(
                "save a software equalizer profile before calculating its response".to_owned(),
            )
        })?;
        if !PIPEWIRE_RATES.contains(&f64::from(sample_rate)) {
            return Err(EqChainError::Invalid(format!(
                "software equalizer response supports 44100, 48000, or 96000 Hz, not {sample_rate}"
            )));
        }
        Ok(EQ_FREQUENCIES.map(|frequency| {
            self.preamp_db
                + cascade_response_db(&self.bands, f64::from(sample_rate), f64::from(frequency))
        }))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EqChainChange {
    pub config: EqChainConfig,
    pub changed: bool,
}

#[derive(Debug)]
pub enum EqChainError {
    Io(io::Error),
    Invalid(String),
}

pub fn bands_from_profile(
    profile: &Profile,
    controls: &[ControlSnapshot],
) -> Result<Vec<EqBand>, EqChainError> {
    EQ_FREQUENCIES
        .into_iter()
        .enumerate()
        .map(|(index, frequency)| {
            let name = format!("EQ Band{index}");
            let value = profile
                .controls
                .get(&name)
                .and_then(|control| control.playback_level)
                .ok_or_else(|| {
                    EqChainError::Invalid(format!(
                        "profile '{}' has no playback level for {name}",
                        profile.name
                    ))
                })?;
            let level = controls
                .iter()
                .find(|control| control.name == name)
                .and_then(|control| control.playback_level.as_ref())
                .ok_or_else(|| {
                    EqChainError::Invalid(format!("live ALSA control {name} has no playback level"))
                })?;
            if !(level.min..=level.max).contains(&value) {
                return Err(EqChainError::Invalid(format!(
                    "profile {} value {value} is outside live ALSA range {}..{}",
                    name, level.min, level.max
                )));
            }
            let db = level.db.as_ref().ok_or_else(|| {
                EqChainError::Invalid(format!(
                    "live ALSA control {name} has no readable dB mapping"
                ))
            })?;
            let raw_span = level.max.checked_sub(level.min).ok_or_else(|| {
                EqChainError::Invalid(format!(
                    "live ALSA control {name} has invalid range {}..{}",
                    level.min, level.max
                ))
            })?;
            let db_span = db.step.checked_mul(raw_span).ok_or_else(|| {
                EqChainError::Invalid(format!("live ALSA dB mapping for {name} overflows"))
            })?;
            if raw_span == 0 || db.step <= 0 || db.min.checked_add(db_span) != Some(db.max) {
                return Err(EqChainError::Invalid(format!(
                    "live ALSA control {name} has a non-linear or invalid dB mapping"
                )));
            }
            let offset = value.checked_sub(level.min).ok_or_else(|| {
                EqChainError::Invalid(format!("live ALSA range for {name} overflows"))
            })?;
            let gain_hundredths = db
                .step
                .checked_mul(offset)
                .and_then(|gain| db.min.checked_add(gain))
                .ok_or_else(|| {
                    EqChainError::Invalid(format!("profile dB mapping for {name} overflows"))
                })?;
            Ok(EqBand {
                frequency,
                q: EQ_Q,
                gain_db: gain_hundredths as f64 / 100.0,
            })
        })
        .collect()
}

pub fn bands_from_gains_tenths_db(gains: &[i16]) -> Result<Vec<EqBand>, EqChainError> {
    if gains.len() != EQ_FREQUENCIES.len() {
        return Err(EqChainError::Invalid(format!(
            "equalizer needs exactly {} gains, found {}",
            EQ_FREQUENCIES.len(),
            gains.len()
        )));
    }
    let bands = EQ_FREQUENCIES
        .into_iter()
        .zip(gains.iter().copied())
        .map(|(frequency, gain)| EqBand {
            frequency,
            q: EQ_Q,
            gain_db: f64::from(gain) / 10.0,
        })
        .collect::<Vec<_>>();
    validate_bands(&bands)?;
    Ok(bands)
}

pub fn eq_chain_config() -> Result<EqChainConfig, EqChainError> {
    eq_chain_config_at(&eq_chain_path()?)
}

pub fn enable_eq_chain(
    profile: &Profile,
    controls: &[ControlSnapshot],
    target_node: &str,
) -> Result<EqChainChange, EqChainError> {
    enable_eq_chain_at(&eq_chain_path()?, profile, controls, target_node)
}

pub fn enable_eq_chain_bands(
    bands: &[EqBand],
    target_node: &str,
) -> Result<EqChainChange, EqChainError> {
    set_eq_chain_enabled_at(&eq_chain_path()?, Some((bands, target_node)))
}

fn enable_eq_chain_at(
    path: &Path,
    profile: &Profile,
    controls: &[ControlSnapshot],
    target_node: &str,
) -> Result<EqChainChange, EqChainError> {
    let bands = bands_from_profile(profile, controls)?;
    set_eq_chain_enabled_at(path, Some((&bands, target_node)))
}

pub fn disable_eq_chain() -> Result<EqChainChange, EqChainError> {
    set_eq_chain_enabled_at(&eq_chain_path()?, None)
}

pub fn restore_eq_chain_config(config: &EqChainConfig) -> Result<EqChainChange, EqChainError> {
    let path = eq_chain_path()?;
    if config.path != path {
        return Err(EqChainError::Invalid(format!(
            "cannot restore software equalizer state from {} into {}",
            config.path.display(),
            path.display()
        )));
    }
    restore_eq_chain_config_at(&path, config)
}

fn restore_eq_chain_config_at(
    path: &Path,
    config: &EqChainConfig,
) -> Result<EqChainChange, EqChainError> {
    match (config.enabled, config.target_node.as_deref()) {
        (false, _) => set_eq_chain_enabled_at(path, None),
        (true, Some(target)) if config.preamp_db == 0.0 => {
            set_eq_chain_enabled_at(path, Some((&config.bands, target)))
        }
        (true, Some(target)) => restore_legacy_eq_chain_at(path, config, target),
        (true, None) => Err(EqChainError::Invalid(
            "enabled software equalizer state has no target node".to_owned(),
        )),
    }
}

fn restore_legacy_eq_chain_at(
    path: &Path,
    config: &EqChainConfig,
    target_node: &str,
) -> Result<EqChainChange, EqChainError> {
    let current = eq_chain_config_at(path)?;
    let expected_preamp = legacy_v1_preamp_db(&config.bands)?;
    if config.preamp_db != expected_preamp {
        return Err(EqChainError::Invalid(format!(
            "legacy software equalizer preamp {:+.2} dB does not match its saved bands",
            config.preamp_db
        )));
    }
    let contents = render_legacy_config(&config.bands, target_node)?;
    if fs::read_to_string(path).ok().as_deref() == Some(contents.as_str()) {
        return Ok(EqChainChange {
            config: current,
            changed: false,
        });
    }

    let parent = path.parent().ok_or_else(|| {
        EqChainError::Invalid(format!("{} has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    if current.enabled {
        replace_file(path, contents.as_bytes())?;
    } else {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
    }
    Ok(EqChainChange {
        config: eq_chain_config_at(path)?,
        changed: true,
    })
}

pub fn validate_eq_chain_activation(
    config: &EqChainConfig,
    target_node: &str,
) -> Result<(), EqChainError> {
    if !config.enabled {
        return Err(EqChainError::Invalid(
            "save a software equalizer profile before applying it".to_owned(),
        ));
    }
    if config.target_node.as_deref() != Some(target_node) {
        return Err(EqChainError::Invalid(format!(
            "the software equalizer targets {}, but the current AE-5 output is {target_node}; reinstall it for the current output",
            config.target_node.as_deref().unwrap_or("no PipeWire node")
        )));
    }
    if config.preamp_db != 0.0 {
        return Err(EqChainError::Invalid(
            "this saved equalizer still uses the retired preamp; apply the EQ preset again to migrate it to unity gain"
                .to_owned(),
        ));
    }
    config.filter_graph()?;
    Ok(())
}

fn render_config(bands: &[EqBand], target_node: &str) -> Result<String, EqChainError> {
    validate_bands(bands)?;
    validate_target_node(target_node)?;

    let gains = bands
        .iter()
        .map(|band| format_gain(band.gain_db))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(format!(
        "{MANAGED_HEADER}{FORMAT_LINE}{GAINS_PREFIX}{gains} ]\n\
         {TARGET_PREFIX}{target_node}\n\
         {PREAMP_PREFIX}0.0\n"
    ))
}

fn render_legacy_config(bands: &[EqBand], target_node: &str) -> Result<String, EqChainError> {
    validate_bands(bands)?;
    validate_target_node(target_node)?;

    let gains = bands
        .iter()
        .map(|band| format_gain(band.gain_db))
        .collect::<Vec<_>>()
        .join(" ");
    let preamp_db = legacy_v1_preamp_db(bands)?;
    Ok(format!(
        "{MANAGED_HEADER}{LEGACY_FORMAT_LINE}{GAINS_PREFIX}{gains} ]\n\
         {TARGET_PREFIX}{target_node}\n\
         {LEGACY_PREAMP_PREFIX}{}\n",
        format_gain(preamp_db)
    ))
}

fn render_filter_graph(bands: &[EqBand]) -> Result<String, EqChainError> {
    render_filter_graph_with_preamp(bands, None)
}

fn render_legacy_filter_graph(bands: &[EqBand], preamp_db: f64) -> Result<String, EqChainError> {
    if !preamp_db.is_finite() || preamp_db > 0.0 {
        return Err(EqChainError::Invalid(format!(
            "legacy software equalizer preamp {preamp_db:+.2} dB is invalid"
        )));
    }
    render_filter_graph_with_preamp(bands, Some(preamp_db))
}

fn render_filter_graph_with_preamp(
    bands: &[EqBand],
    preamp_db: Option<f64>,
) -> Result<String, EqChainError> {
    validate_bands(bands)?;
    let mut output = String::from("{ nodes = [\n");
    if let Some(preamp_db) = preamp_db {
        let multiplier = 10.0_f64.powf(preamp_db / 20.0);
        for channel in ['L', 'R'] {
            output.push_str(&format!(
                "  {{ type = builtin name = pre{channel} label = linear \
                 control = {{ Mult = {} Add = 0.0 }} }}\n",
                format_multiplier(multiplier)
            ));
        }
    }
    for channel in ['L', 'R'] {
        for (index, band) in bands.iter().enumerate() {
            output.push_str(&format!(
                "  {{ type = builtin name = eq{channel}{index} label = bq_peaking \
                 control = {{ Freq = {} Q = {:.1} Gain = {} }} }}\n",
                band.frequency,
                band.q,
                format_gain(band.gain_db)
            ));
        }
    }
    output.push_str("] links = [\n");
    for channel in ['L', 'R'] {
        if preamp_db.is_some() {
            output.push_str(&format!(
                "  {{ output = \"pre{channel}:Out\" input = \"eq{channel}0:In\" }}\n"
            ));
        }
        for index in 0..bands.len() - 1 {
            output.push_str(&format!(
                "  {{ output = \"eq{channel}{index}:Out\" input = \"eq{channel}{}:In\" }}\n",
                index + 1
            ));
        }
    }
    let inputs = if preamp_db.is_some() {
        "\"preL:In\" \"preR:In\""
    } else {
        "\"eqL0:In\" \"eqR0:In\""
    };
    output.push_str(&format!(
        "] inputs = [ {inputs} ] outputs = [ \"eqL9:Out\" \"eqR9:Out\" ] }}\n"
    ));
    Ok(output)
}

fn eq_chain_signature(bands: &[EqBand], target_node: &str) -> String {
    format!(
        "direct-v2|{target_node}|{}",
        bands
            .iter()
            .map(|band| format_gain(band.gain_db))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn legacy_eq_chain_signature(bands: &[EqBand], target_node: &str, preamp_db: f64) -> String {
    format!(
        "direct-v1|{target_node}|{}|{}",
        format_gain(preamp_db),
        bands
            .iter()
            .map(|band| format_gain(band.gain_db))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn set_eq_chain_enabled_at(
    path: &Path,
    request: Option<(&[EqBand], &str)>,
) -> Result<EqChainChange, EqChainError> {
    let current = eq_chain_config_at(path)?;
    let Some((bands, target_node)) = request else {
        if !current.enabled {
            return Ok(EqChainChange {
                config: current,
                changed: false,
            });
        }
        fs::remove_file(path)?;
        return Ok(EqChainChange {
            config: eq_chain_config_at(path)?,
            changed: true,
        });
    };

    let contents = render_config(bands, target_node)?;
    if current.enabled
        && current.bands == bands
        && current.target_node.as_deref() == Some(target_node)
        && current.preamp_db == 0.0
    {
        return Ok(EqChainChange {
            config: current,
            changed: false,
        });
    }

    let parent = path.parent().ok_or_else(|| {
        EqChainError::Invalid(format!("{} has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    if current.enabled {
        replace_file(path, contents.as_bytes())?;
    } else {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(contents.as_bytes())?;
        file.sync_all()?;
    }
    Ok(EqChainChange {
        config: eq_chain_config_at(path)?,
        changed: true,
    })
}

fn validate_bands(bands: &[EqBand]) -> Result<(), EqChainError> {
    if bands.len() != EQ_FREQUENCIES.len() {
        return Err(EqChainError::Invalid(format!(
            "equalizer needs {} bands, found {}",
            EQ_FREQUENCIES.len(),
            bands.len()
        )));
    }
    for (index, (band, frequency)) in bands.iter().zip(EQ_FREQUENCIES).enumerate() {
        if band.frequency != frequency || band.q != EQ_Q || !band.gain_db.is_finite() {
            return Err(EqChainError::Invalid(format!(
                "equalizer band {index} has an invalid frequency, Q, or gain"
            )));
        }
        if !(PIPEWIRE_MIN_GAIN_DB..=PIPEWIRE_MAX_GAIN_DB).contains(&band.gain_db) {
            return Err(EqChainError::Invalid(format!(
                "equalizer band {index} requests {:+.2} dB, but PipeWire bq_peaking supports {:.0}..+{:.0} dB",
                band.gain_db, PIPEWIRE_MIN_GAIN_DB, PIPEWIRE_MAX_GAIN_DB
            )));
        }
    }
    Ok(())
}

fn validate_target_node(target_node: &str) -> Result<(), EqChainError> {
    if target_node.is_empty()
        || !target_node
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err(EqChainError::Invalid(format!(
            "PipeWire target node '{target_node}' contains unsupported characters"
        )));
    }
    Ok(())
}

fn format_gain(gain: f64) -> String {
    let formatted = format!("{gain:.2}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed.contains('.') {
        trimmed.to_owned()
    } else {
        format!("{trimmed}.0")
    }
}

fn format_multiplier(multiplier: f64) -> String {
    let formatted = format!("{multiplier:.9}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed.contains('.') {
        trimmed.to_owned()
    } else {
        format!("{trimmed}.0")
    }
}

fn legacy_v1_preamp_db(bands: &[EqBand]) -> Result<f64, EqChainError> {
    let peak_db = maximum_response_db(bands)?;
    if peak_db <= 0.0 {
        return Ok(0.0);
    }
    Ok(-((peak_db + LEGACY_HEADROOM_MARGIN_DB) * 100.0).ceil() / 100.0)
}

fn maximum_response_db(bands: &[EqBand]) -> Result<f64, EqChainError> {
    validate_bands(bands)?;
    let mut maximum = 0.0_f64;
    for sample_rate in PIPEWIRE_RATES {
        let nyquist = sample_rate / 2.0;
        for step in 0..=RESPONSE_STEPS {
            let frequency = nyquist * step as f64 / RESPONSE_STEPS as f64;
            maximum = maximum.max(cascade_response_db(bands, sample_rate, frequency));
        }
        for band in bands {
            maximum = maximum.max(cascade_response_db(
                bands,
                sample_rate,
                f64::from(band.frequency),
            ));
        }
    }
    if !maximum.is_finite() {
        return Err(EqChainError::Invalid(
            "equalizer response calculation produced a non-finite peak".to_owned(),
        ));
    }
    Ok(maximum)
}

fn cascade_response_db(bands: &[EqBand], sample_rate: f64, frequency: f64) -> f64 {
    bands
        .iter()
        .map(|band| peaking_response_db(band, sample_rate, frequency))
        .sum()
}

fn peaking_response_db(band: &EqBand, sample_rate: f64, frequency: f64) -> f64 {
    let centre = 2.0 * std::f64::consts::PI * f64::from(band.frequency) / sample_rate;
    let alpha = centre.sin() / (2.0 * band.q);
    let amplitude = 10.0_f64.powf(band.gain_db / 40.0);
    let a0 = 1.0 + alpha / amplitude;
    let b0 = (1.0 + alpha * amplitude) / a0;
    let b1 = -2.0 * centre.cos() / a0;
    let b2 = (1.0 - alpha * amplitude) / a0;
    let a1 = b1;
    let a2 = (1.0 - alpha / amplitude) / a0;

    let omega = 2.0 * std::f64::consts::PI * frequency / sample_rate;
    let cosine = omega.cos();
    let sine = omega.sin();
    let cosine2 = (2.0 * omega).cos();
    let sine2 = (2.0 * omega).sin();
    let numerator_real = b0 + b1 * cosine + b2 * cosine2;
    let numerator_imaginary = -b1 * sine - b2 * sine2;
    let denominator_real = 1.0 + a1 * cosine + a2 * cosine2;
    let denominator_imaginary = -a1 * sine - a2 * sine2;
    let numerator_power =
        numerator_real * numerator_real + numerator_imaginary * numerator_imaginary;
    let denominator_power =
        denominator_real * denominator_real + denominator_imaginary * denominator_imaginary;
    10.0 * (numerator_power / denominator_power).log10()
}

fn eq_chain_path() -> Result<PathBuf, EqChainError> {
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
            EqChainError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "neither XDG_CONFIG_HOME nor HOME is available",
            ))
        })
}

fn eq_chain_config_at(path: &Path) -> Result<EqChainConfig, EqChainError> {
    match fs::read_to_string(path) {
        Ok(contents) => {
            let (bands, target_node, preamp_db) = parse_managed_config(path, &contents)?;
            Ok(EqChainConfig {
                path: path.to_owned(),
                enabled: true,
                bands,
                target_node,
                preamp_db,
            })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(EqChainConfig {
            path: path.to_owned(),
            enabled: false,
            bands: Vec::new(),
            target_node: None,
            preamp_db: 0.0,
        }),
        Err(error) => Err(error.into()),
    }
}

fn parse_managed_config(
    path: &Path,
    contents: &str,
) -> Result<(Vec<EqBand>, Option<String>, f64), EqChainError> {
    let format_line = contents
        .lines()
        .nth(1)
        .ok_or_else(|| foreign_config(path))?;
    let gains = contents
        .lines()
        .nth(2)
        .and_then(|line| line.strip_prefix(GAINS_PREFIX))
        .and_then(|line| line.strip_suffix(" ]"))
        .ok_or_else(|| foreign_config(path))?
        .split_whitespace()
        .map(|gain| gain.parse::<f64>().map_err(|_| foreign_config(path)))
        .collect::<Result<Vec<_>, _>>()?;
    let bands = EQ_FREQUENCIES
        .into_iter()
        .zip(gains)
        .map(|(frequency, gain_db)| EqBand {
            frequency,
            q: EQ_Q,
            gain_db,
        })
        .collect::<Vec<_>>();
    let target_node = contents
        .lines()
        .nth(3)
        .and_then(|line| line.strip_prefix(TARGET_PREFIX))
        .ok_or_else(|| foreign_config(path))?;
    let (preamp_prefix, rendered) = if format_line == FORMAT_LINE.trim_end() {
        (PREAMP_PREFIX, render_config(&bands, target_node))
    } else if format_line == LEGACY_FORMAT_LINE.trim_end() {
        (
            LEGACY_PREAMP_PREFIX,
            render_legacy_config(&bands, target_node),
        )
    } else {
        return Err(foreign_config(path));
    };
    let preamp_db = contents
        .lines()
        .nth(4)
        .and_then(|line| line.strip_prefix(preamp_prefix))
        .and_then(|value| value.parse::<f64>().ok())
        .ok_or_else(|| foreign_config(path))?;
    if rendered.ok().as_deref() != Some(contents) {
        return Err(foreign_config(path));
    }
    Ok((bands, Some(target_node.to_owned()), preamp_db))
}

fn foreign_config(path: &Path) -> EqChainError {
    EqChainError::Io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "{} exists but is not managed by AE-5 Control",
            path.display()
        ),
    ))
}

fn replace_file(path: &Path, contents: &[u8]) -> Result<(), EqChainError> {
    let parent = path.parent().ok_or_else(|| {
        EqChainError::Invalid(format!("{} has no parent directory", path.display()))
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| EqChainError::Invalid(format!("{} has no file name", path.display())))?
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

impl fmt::Display for EqChainError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(output),
            Self::Invalid(message) => output.write_str(message),
        }
    }
}

impl std::error::Error for EqChainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<io::Error> for EqChainError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DecibelRange, Level, ProfileControl};
    use std::collections::BTreeMap;
    use std::fs;
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEST_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    fn profile_with(values: [i64; 10]) -> Profile {
        Profile::new(
            "Software EQ",
            values
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    (
                        format!("EQ Band{index}"),
                        ProfileControl {
                            playback_level: Some(value),
                            ..ProfileControl::default()
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        )
        .unwrap()
    }

    fn live_controls() -> Vec<ControlSnapshot> {
        let mut controls = (0..10)
            .map(|index| ControlSnapshot {
                name: format!("EQ Band{index}"),
                selected: None,
                choices: Vec::new(),
                playback_switch: None,
                capture_switch: None,
                playback_level: Some(Level {
                    value: 24,
                    min: 0,
                    max: 48,
                    db: Some(DecibelRange {
                        min: -2400,
                        max: 2400,
                        step: 100,
                    }),
                }),
                capture_level: None,
                playback_channels: Vec::new(),
                capture_channels: Vec::new(),
            })
            .collect::<Vec<_>>();
        controls.push(ControlSnapshot {
            name: "Enable OutFX".to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: Some(false),
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        });
        controls
    }

    fn flat_bands() -> Vec<EqBand> {
        bands_from_profile(&profile_with([24; 10]), &live_controls()).unwrap()
    }

    #[test]
    fn draft_gains_map_to_the_fixed_ten_band_graph() {
        let bands =
            bands_from_gains_tenths_db(&[-120, -60, 0, 10, 20, 30, 40, 50, 60, 120]).unwrap();

        assert_eq!(
            bands
                .iter()
                .map(|band| (band.frequency, band.gain_db))
                .collect::<Vec<_>>(),
            vec![
                (31, -12.0),
                (62, -6.0),
                (125, 0.0),
                (250, 1.0),
                (500, 2.0),
                (1000, 3.0),
                (2000, 4.0),
                (4000, 5.0),
                (8000, 6.0),
                (16000, 12.0),
            ]
        );
    }

    #[test]
    fn draft_gains_reject_an_incomplete_equalizer() {
        let error = bands_from_gains_tenths_db(&[0; 9]).unwrap_err();

        assert!(error.to_string().contains("exactly 10"));
    }

    fn test_path() -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "ae5-eq-chain-test-{}-{}",
                std::process::id(),
                NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ))
            .join(CONFIG_FILE)
    }

    #[test]
    fn maps_the_alsa_minimum_to_minus_24_db() {
        let bands = bands_from_profile(&profile_with([0; 10]), &live_controls()).unwrap();
        assert_eq!(bands[0].gain_db, -24.0);
    }

    #[test]
    fn maps_the_alsa_centre_to_zero_db() {
        let bands = bands_from_profile(&profile_with([24; 10]), &live_controls()).unwrap();
        assert_eq!(bands[0].gain_db, 0.0);
    }

    #[test]
    fn maps_the_alsa_maximum_to_plus_24_db() {
        let bands = bands_from_profile(&profile_with([48; 10]), &live_controls()).unwrap();
        assert_eq!(bands[0].gain_db, 24.0);
    }

    #[test]
    fn maps_flat_profiles_to_unity_gain_on_every_band() {
        let bands = flat_bands();
        assert!(bands.iter().all(|band| band.gain_db == 0.0));
    }

    #[test]
    fn renders_the_same_profile_byte_identically() {
        let bands = flat_bands();
        assert_eq!(
            render_config(&bands, "alsa_output.pci-ae5.analog-stereo").unwrap(),
            render_config(&bands, "alsa_output.pci-ae5.analog-stereo").unwrap()
        );
    }

    #[test]
    fn rejects_gains_pipewire_would_silently_clamp() {
        let bands = bands_from_profile(&profile_with([48; 10]), &live_controls()).unwrap();
        let error = render_config(&bands, "alsa_output.pci-ae5.analog-stereo").unwrap_err();
        assert!(error.to_string().contains("supports -120..+20 dB"));
    }

    #[test]
    fn generated_config_is_accepted_by_pipewire_config_parser_when_available() {
        let path = test_path().with_file_name("graph.conf");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let graph = render_filter_graph(&flat_bands()).unwrap();
        fs::write(
            &path,
            format!("context.properties = {{ ae5.test.graph = {graph} }}\n"),
        )
        .unwrap();

        match Command::new("pw-config")
            .args(["-n", path.to_str().unwrap(), "merge", "context.modules"])
            .output()
        {
            Ok(output) => assert!(
                output.status.success(),
                "pw-config rejected generated config: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                eprintln!("pw-config unavailable; syntax parser check skipped")
            }
            Err(error) => panic!("failed to run pw-config: {error}"),
        }

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn enable_disable_round_trip_leaves_no_config_file() {
        let path = test_path();
        let bands = flat_bands();

        let first =
            set_eq_chain_enabled_at(&path, Some((&bands, "alsa_output.pci-ae5.analog-stereo")))
                .unwrap();
        let installed = fs::read(&path).unwrap();
        let second =
            set_eq_chain_enabled_at(&path, Some((&bands, "alsa_output.pci-ae5.analog-stereo")))
                .unwrap();
        let installed_again = fs::read(&path).unwrap();
        let disabled = set_eq_chain_enabled_at(&path, None).unwrap();

        assert!(first.changed);
        assert!(!second.changed);
        assert_eq!(installed_again, installed);
        assert!(disabled.changed);
        assert!(!path.exists());

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn restoring_a_previous_config_recreates_it_byte_identically() {
        let path = test_path();
        let mut previous_bands = flat_bands();
        previous_bands[2].gain_db = 3.0;
        set_eq_chain_enabled_at(
            &path,
            Some((&previous_bands, "alsa_output.pci-ae5.analog-stereo")),
        )
        .unwrap();
        let previous = eq_chain_config_at(&path).unwrap();
        let previous_bytes = fs::read(&path).unwrap();

        let mut replacement_bands = flat_bands();
        replacement_bands[5].gain_db = -4.0;
        set_eq_chain_enabled_at(
            &path,
            Some((&replacement_bands, "alsa_output.pci-ae5.analog-stereo")),
        )
        .unwrap();
        restore_eq_chain_config_at(&path, &previous).unwrap();

        assert_eq!(fs::read(&path).unwrap(), previous_bytes);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn restoring_a_legacy_config_after_a_failed_migration_is_byte_identical() {
        let path = test_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut previous_bands = flat_bands();
        previous_bands[5].gain_db = 10.0;
        let previous_bytes =
            render_legacy_config(&previous_bands, "alsa_output.pci-ae5.analog-stereo").unwrap();
        fs::write(&path, &previous_bytes).unwrap();
        let previous = eq_chain_config_at(&path).unwrap();

        set_eq_chain_enabled_at(
            &path,
            Some((&previous_bands, "alsa_output.pci-ae5.analog-stereo")),
        )
        .unwrap();
        restore_eq_chain_config_at(&path, &previous).unwrap();

        assert_eq!(fs::read_to_string(&path).unwrap(), previous_bytes);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn refuses_to_replace_or_remove_foreign_config() {
        let path = test_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "user configuration\n").unwrap();

        let bands = flat_bands();
        let enable_error =
            set_eq_chain_enabled_at(&path, Some((&bands, "alsa_output.pci-ae5.analog-stereo")))
                .unwrap_err();
        let disable_error = set_eq_chain_enabled_at(&path, None).unwrap_err();

        assert!(enable_error.to_string().contains("not managed"));
        assert!(disable_error.to_string().contains("not managed"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "user configuration\n");

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn rendered_chain_targets_the_physical_ae5_sink() {
        let config = render_config(&flat_bands(), "alsa_output.pci-ae5.analog-stereo").unwrap();
        assert!(config.contains("# Target: alsa_output.pci-ae5.analog-stereo"));
        assert!(!config.contains("target.object"));
    }

    #[test]
    fn enabling_software_eq_accepts_active_hardware_effects() {
        let profile = profile_with([24; 10]);
        let mut controls = live_controls();
        controls
            .iter_mut()
            .find(|control| control.name == "Enable OutFX")
            .unwrap()
            .playback_switch = Some(true);

        let path = test_path();
        let change = enable_eq_chain_at(
            &path,
            &profile,
            &controls,
            "alsa_output.pci-ae5.analog-stereo",
        )
        .unwrap();

        assert!(change.config.enabled);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn activation_rejects_a_stale_physical_target() {
        let config = EqChainConfig {
            path: test_path(),
            enabled: true,
            bands: flat_bands(),
            target_node: Some("alsa_output.old-profile".to_owned()),
            preamp_db: 0.0,
        };

        let error =
            validate_eq_chain_activation(&config, "alsa_output.current-profile").unwrap_err();

        assert!(error.to_string().contains("reinstall it"));
    }

    #[test]
    fn activation_rejects_a_legacy_attenuated_graph() {
        let config = EqChainConfig {
            path: test_path(),
            enabled: true,
            bands: flat_bands(),
            target_node: Some("alsa_output.current-profile".to_owned()),
            preamp_db: -1.0,
        };

        let error =
            validate_eq_chain_activation(&config, "alsa_output.current-profile").unwrap_err();

        assert!(error.to_string().contains("retired preamp"));
    }

    #[test]
    fn activation_accepts_the_current_graph_with_outfx_off() {
        let config = EqChainConfig {
            path: test_path(),
            enabled: true,
            bands: flat_bands(),
            target_node: Some("alsa_output.current-profile".to_owned()),
            preamp_db: 0.0,
        };

        validate_eq_chain_activation(&config, "alsa_output.current-profile").unwrap();
    }

    #[test]
    fn target_node_rejects_pipewire_config_injection() {
        let error = render_config(
            &flat_bands(),
            "alsa_output.valid\" node.dont-fallback=false",
        )
        .unwrap_err();

        assert!(error.to_string().contains("unsupported characters"));
    }

    #[test]
    fn new_config_records_unity_preamp() {
        let config = render_config(&flat_bands(), "alsa_output.current-profile").unwrap();

        assert!(config.contains("# Format: direct-filter-v2"));
        assert!(config.contains("# Preamp (dB): 0.0"));
        assert!(!config.contains("# Automatic preamp"));
    }

    #[test]
    fn boosted_equalizer_does_not_add_automatic_attenuation() {
        let mut bands = flat_bands();
        bands[5].gain_db = 10.0;

        let config = render_config(&bands, "alsa_output.current-profile").unwrap();
        let graph = render_filter_graph(&bands).unwrap();

        assert!(config.contains("# Preamp (dB): 0.0"));
        assert!(!graph.contains("label = linear"));
        assert!(!graph.contains("name = preL"));
        assert!(!graph.contains("name = preR"));
    }

    #[test]
    fn direct_graph_starts_with_equalizer_nodes_and_has_no_virtual_sink() {
        let mut bands = flat_bands();
        bands[5].gain_db = 10.0;

        let graph = render_filter_graph(&bands).unwrap();

        assert!(!graph.contains("label = linear"));
        assert!(!graph.contains("preL"));
        assert!(!graph.contains("preR"));
        assert!(graph.contains("inputs = [ \"eqL0:In\" \"eqR0:In\" ]"));
        assert!(!graph.contains("libpipewire-module-filter-chain"));
        assert!(!graph.contains("target.object"));
    }

    #[test]
    fn applying_the_same_bands_migrates_a_managed_v1_preamp_config() {
        let path = test_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            "# Managed by AE-5 Control.\n\
             # Format: direct-filter-v1\n\
             # EQ gains (dB): [ 0.0 0.0 0.0 0.0 0.0 10.0 0.0 0.0 0.0 0.0 ]\n\
             # Target: alsa_output.current-profile\n\
             # Automatic preamp (dB): -10.26\n",
        )
        .unwrap();
        let mut bands = flat_bands();
        bands[5].gain_db = 10.0;

        let change =
            set_eq_chain_enabled_at(&path, Some((&bands, "alsa_output.current-profile"))).unwrap();
        let contents = fs::read_to_string(&path).unwrap();

        assert!(change.changed);
        assert_eq!(change.config.preamp_db, 0.0);
        assert!(contents.contains("# Format: direct-filter-v2"));
        assert!(!contents.contains("# Automatic preamp"));
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn reports_the_exact_expected_response_of_the_saved_graph() {
        let config = EqChainConfig {
            path: test_path(),
            enabled: true,
            bands: flat_bands(),
            target_node: Some("alsa_output.current-profile".to_owned()),
            preamp_db: 0.0,
        };

        assert_eq!(config.expected_responses_db(48_000).unwrap(), [0.0; 10]);
        assert!(
            config
                .expected_responses_db(192_000)
                .unwrap_err()
                .to_string()
                .contains("44100, 48000, or 96000")
        );
    }
}
