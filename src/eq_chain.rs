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
const CONFIG_FILE: &str = "92-ae5-control-eq.conf";
const MANAGED_HEADER: &str = "# Managed by AE-5 Control.\n";
const GAINS_PREFIX: &str = "# EQ gains (dB): [ ";
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

pub fn eq_chain_config() -> Result<EqChainConfig, EqChainError> {
    eq_chain_config_at(&eq_chain_path()?)
}

pub fn enable_eq_chain(
    profile: &Profile,
    controls: &[ControlSnapshot],
) -> Result<EqChainChange, EqChainError> {
    let bands = bands_from_profile(profile, controls)?;
    set_eq_chain_enabled_at(&eq_chain_path()?, Some(&bands))
}

pub fn disable_eq_chain() -> Result<EqChainChange, EqChainError> {
    set_eq_chain_enabled_at(&eq_chain_path()?, None)
}

fn render_config(bands: &[EqBand]) -> Result<String, EqChainError> {
    validate_bands(bands)?;

    let gains = bands
        .iter()
        .map(|band| format_gain(band.gain_db))
        .collect::<Vec<_>>()
        .join(" ");
    let mut output = format!("{MANAGED_HEADER}{GAINS_PREFIX}{gains} ]\ncontext.modules = [\n");
    output.push_str(
        "  { name = libpipewire-module-filter-chain\n\
         \x20   args = {\n\
         \x20     node.description = \"AE-5 Software Equalizer\"\n\
         \x20     filter.graph = {\n\
         \x20       nodes = [\n",
    );
    for channel in ['L', 'R'] {
        for (index, band) in bands.iter().enumerate() {
            output.push_str(&format!(
                "          {{ type = builtin name = eq{channel}{index} label = bq_peaking\n\
                 \x20           control = {{ Freq = {} Q = {:.1} Gain = {} }} }}\n",
                band.frequency,
                band.q,
                format_gain(band.gain_db)
            ));
        }
    }
    output.push_str("        ]\n        links = [\n");
    for channel in ['L', 'R'] {
        for index in 0..bands.len() - 1 {
            output.push_str(&format!(
                "          {{ output = \"eq{channel}{index}:Out\" input = \"eq{channel}{}:In\" }}\n",
                index + 1
            ));
        }
    }
    output.push_str(
        "        ]\n\
         \x20       inputs  = [ \"eqL0:In\" \"eqR0:In\" ]\n\
         \x20       outputs = [ \"eqL9:Out\" \"eqR9:Out\" ]\n\
         \x20     }\n\
         \x20     capture.props = {\n\
         \x20       node.name = \"ae5_software_equalizer\"\n\
         \x20       media.class = Audio/Sink\n\
         \x20       audio.channels = 2\n\
         \x20       audio.position = [ FL FR ]\n\
         \x20     }\n\
         \x20     playback.props = {\n\
         \x20       node.name = \"ae5_software_equalizer_output\"\n\
         \x20       node.passive = true\n\
         \x20       audio.channels = 2\n\
         \x20       audio.position = [ FL FR ]\n\
         \x20     }\n\
         \x20   }\n\
         \x20 }\n\
         ]\n",
    );
    Ok(output)
}

fn set_eq_chain_enabled_at(
    path: &Path,
    bands: Option<&[EqBand]>,
) -> Result<EqChainChange, EqChainError> {
    let current = eq_chain_config_at(path)?;
    let Some(bands) = bands else {
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

    let contents = render_config(bands)?;
    if current.enabled && current.bands == bands {
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

fn format_gain(gain: f64) -> String {
    let formatted = format!("{gain:.2}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed.contains('.') {
        trimmed.to_owned()
    } else {
        format!("{trimmed}.0")
    }
}

fn eq_chain_path() -> Result<PathBuf, EqChainError> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return Ok(path.join("pipewire/pipewire.conf.d").join(CONFIG_FILE));
    }
    env::var_os("HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| {
            path.join(".config/pipewire/pipewire.conf.d")
                .join(CONFIG_FILE)
        })
        .ok_or_else(|| {
            EqChainError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "neither XDG_CONFIG_HOME nor HOME is available",
            ))
        })
}

fn eq_chain_config_at(path: &Path) -> Result<EqChainConfig, EqChainError> {
    match fs::read_to_string(path) {
        Ok(contents) => Ok(EqChainConfig {
            path: path.to_owned(),
            enabled: true,
            bands: parse_managed_config(path, &contents)?,
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(EqChainConfig {
            path: path.to_owned(),
            enabled: false,
            bands: Vec::new(),
        }),
        Err(error) => Err(error.into()),
    }
}

fn parse_managed_config(path: &Path, contents: &str) -> Result<Vec<EqBand>, EqChainError> {
    let gains = contents
        .lines()
        .nth(1)
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
    if render_config(&bands).ok().as_deref() != Some(contents) {
        return Err(foreign_config(path));
    }
    Ok(bands)
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
        (0..10)
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
            .collect()
    }

    fn flat_bands() -> Vec<EqBand> {
        bands_from_profile(&profile_with([24; 10]), &live_controls()).unwrap()
    }

    fn test_path() -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "ae5-eq-chain-test-{}-{}",
                std::process::id(),
                NEXT_TEST_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ))
            .join("92-ae5-control-eq.conf")
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
            render_config(&bands).unwrap(),
            render_config(&bands).unwrap()
        );
    }

    #[test]
    fn rejects_gains_pipewire_would_silently_clamp() {
        let bands = bands_from_profile(&profile_with([48; 10]), &live_controls()).unwrap();
        let error = render_config(&bands).unwrap_err();
        assert!(error.to_string().contains("supports -120..+20 dB"));
    }

    #[test]
    fn generated_config_is_accepted_by_pipewire_config_parser_when_available() {
        let path = test_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, render_config(&flat_bands()).unwrap()).unwrap();

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

        let first = set_eq_chain_enabled_at(&path, Some(&bands)).unwrap();
        let installed = fs::read(&path).unwrap();
        let second = set_eq_chain_enabled_at(&path, Some(&bands)).unwrap();
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
    fn refuses_to_replace_or_remove_foreign_config() {
        let path = test_path();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "user configuration\n").unwrap();

        let enable_error = set_eq_chain_enabled_at(&path, Some(&flat_bands())).unwrap_err();
        let disable_error = set_eq_chain_enabled_at(&path, None).unwrap_err();

        assert!(enable_error.to_string().contains("not managed"));
        assert!(disable_error.to_string().contains("not managed"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "user configuration\n");

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
