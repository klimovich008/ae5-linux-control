use std::env;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::EffectsProfileEntry;

const EFFECTS_FILTER_SLOT: u8 = 1;
const CONFIG_FILE: &str = "software-effects.json";
const CONFIG_SCHEMA_VERSION: u32 = 1;
const REQUIRED_LADSPA_PLUGINS: [&str; 4] = [
    "matrix_spatialiser_1422.so",
    "transient_1206.so",
    "sc4_1882.so",
    "fast_lookahead_limiter_1913.so",
];
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectsChainConfig {
    pub path: PathBuf,
    pub enabled: bool,
    pub target_node: Option<String>,
    pub profile: Option<EffectsProfileEntry>,
}

impl EffectsChainConfig {
    pub fn signature(&self) -> Option<String> {
        match (
            self.enabled,
            self.target_node.as_deref(),
            self.profile.as_ref(),
        ) {
            (true, Some(target), Some(profile)) => Some(effects_chain_signature(profile, target)),
            _ => None,
        }
    }

    pub fn filter_graph(&self) -> Result<Option<String>, EffectsChainError> {
        match (self.enabled, self.profile.as_ref()) {
            (false, _) => Ok(None),
            (true, Some(profile)) => render_effects_filter_graph(profile).map(Some),
            (true, None) => Err(EffectsChainError::Invalid(
                "enabled software Effects state has no profile".to_owned(),
            )),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectsChainChange {
    pub config: EffectsChainConfig,
    pub changed: bool,
}

#[derive(Debug)]
pub enum EffectsChainError {
    Io(io::Error),
    Invalid(String),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredEffectsChain {
    schema_version: u32,
    target_node: String,
    profile: EffectsProfileEntry,
}

#[derive(Clone)]
struct StereoPorts {
    left: String,
    right: String,
}

struct GraphBuilder {
    nodes: Vec<String>,
    links: Vec<String>,
    inputs: StereoPorts,
    current: StereoPorts,
}

impl GraphBuilder {
    fn new() -> Self {
        let nodes = ['L', 'R']
            .map(|channel| {
                format!(
                    "  {{ type = builtin name = input{channel} label = linear \
                     control = {{ Mult = 1.0 Add = 0.0 }} }}"
                )
            })
            .to_vec();
        let inputs = StereoPorts {
            left: "inputL:In".to_owned(),
            right: "inputR:In".to_owned(),
        };
        let current = StereoPorts {
            left: "inputL:Out".to_owned(),
            right: "inputR:Out".to_owned(),
        };
        Self {
            nodes,
            links: Vec::new(),
            inputs,
            current,
        }
    }

    fn add_mono_pair(&mut self, name: &str, node: impl Fn(char) -> String) {
        for channel in ['L', 'R'] {
            self.nodes.push(node(channel));
        }
        self.links.push(format!(
            "  {{ output = \"{}\" input = \"{name}L:Input\" }}",
            self.current.left
        ));
        self.links.push(format!(
            "  {{ output = \"{}\" input = \"{name}R:Input\" }}",
            self.current.right
        ));
        self.current = StereoPorts {
            left: format!("{name}L:Output"),
            right: format!("{name}R:Output"),
        };
    }

    fn add_builtin_biquad_pair(&mut self, name: &str, label: &str, controls: &str) {
        for channel in ['L', 'R'] {
            self.nodes.push(format!(
                "  {{ type = builtin name = {name}{channel} label = {label} \
                 control = {{ {controls} }} }}"
            ));
        }
        self.links.push(format!(
            "  {{ output = \"{}\" input = \"{name}L:In\" }}",
            self.current.left
        ));
        self.links.push(format!(
            "  {{ output = \"{}\" input = \"{name}R:In\" }}",
            self.current.right
        ));
        self.current = StereoPorts {
            left: format!("{name}L:Out"),
            right: format!("{name}R:Out"),
        };
    }

    fn add_stereo_ladspa(
        &mut self,
        name: &str,
        plugin: &str,
        label: &str,
        controls: &str,
        input_ports: (&str, &str),
        output_ports: (&str, &str),
    ) {
        self.nodes.push(format!(
            "  {{ type = ladspa name = {name} plugin = {plugin} label = {label} \
             control = {{ {controls} }} }}"
        ));
        self.links.push(format!(
            "  {{ output = \"{}\" input = \"{name}:{}\" }}",
            self.current.left, input_ports.0
        ));
        self.links.push(format!(
            "  {{ output = \"{}\" input = \"{name}:{}\" }}",
            self.current.right, input_ports.1
        ));
        self.current = StereoPorts {
            left: format!("{name}:{}", output_ports.0),
            right: format!("{name}:{}", output_ports.1),
        };
    }

    fn finish(self) -> String {
        format!(
            "{{ nodes = [\n{}\n] links = [\n{}\n] \
             inputs = [ \"{}\" \"{}\" ] outputs = [ \"{}\" \"{}\" ] }}\n",
            self.nodes.join("\n"),
            self.links.join("\n"),
            self.inputs.left,
            self.inputs.right,
            self.current.left,
            self.current.right
        )
    }
}

pub const fn effects_filter_slot() -> u8 {
    EFFECTS_FILTER_SLOT
}

pub fn effects_chain_config() -> Result<EffectsChainConfig, EffectsChainError> {
    effects_chain_config_at(&effects_chain_path()?)
}

pub fn enable_effects_chain(
    profile: &EffectsProfileEntry,
    target_node: &str,
) -> Result<EffectsChainChange, EffectsChainError> {
    set_effects_chain_at(&effects_chain_path()?, Some((profile, target_node)))
}

pub fn disable_effects_chain() -> Result<EffectsChainChange, EffectsChainError> {
    set_effects_chain_at(&effects_chain_path()?, None)
}

pub fn restore_effects_chain_config(
    config: &EffectsChainConfig,
) -> Result<EffectsChainChange, EffectsChainError> {
    let path = effects_chain_path()?;
    if config.path != path {
        return Err(EffectsChainError::Invalid(format!(
            "cannot restore software Effects state from {} into {}",
            config.path.display(),
            path.display()
        )));
    }
    restore_effects_chain_config_at(&path, config)
}

pub fn validate_effects_runtime_support() -> Result<(), EffectsChainError> {
    let search_paths = ladspa_search_paths();
    let missing = REQUIRED_LADSPA_PLUGINS
        .iter()
        .filter(|plugin| {
            !search_paths
                .iter()
                .any(|directory| directory.join(plugin).is_file())
        })
        .map(|plugin| plugin.trim_end_matches(".so"))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(EffectsChainError::Invalid(format!(
            "Linux software Effects need the ladspa-swh-plugins package; missing {}",
            missing.join(", ")
        )))
    }
}

pub fn validate_effects_chain_activation(
    config: &EffectsChainConfig,
    target_node: &str,
) -> Result<(), EffectsChainError> {
    if !config.enabled {
        return Err(EffectsChainError::Invalid(
            "apply an Effects profile before activating it".to_owned(),
        ));
    }
    if config.target_node.as_deref() != Some(target_node) {
        return Err(EffectsChainError::Invalid(format!(
            "the software Effects target is {}, but the current AE-5 output is {target_node}",
            config.target_node.as_deref().unwrap_or("unavailable")
        )));
    }
    validate_effects_runtime_support()?;
    config.filter_graph()?;
    Ok(())
}

pub fn render_effects_filter_graph(
    profile: &EffectsProfileEntry,
) -> Result<String, EffectsChainError> {
    validate_profile(profile)?;
    if !profile.outfx_enabled {
        return Err(EffectsChainError::Invalid(
            "the Effects master must be enabled before rendering an active graph".to_owned(),
        ));
    }

    let mut graph = GraphBuilder::new();
    if profile.crystalizer_available && profile.crystalizer_enabled {
        let amount = f64::from(profile.crystalizer_level) / 100.0;
        let attack = format_number(amount * 0.65);
        let sustain = format_number(amount * -0.15);
        graph.add_mono_pair("crystal", |channel| {
            format!(
                "  {{ type = ladspa name = crystal{channel} plugin = transient_1206 \
                 label = transient control = {{ \"Attack speed\" = {attack} \
                 \"Sustain time\" = {sustain} }} }}"
            )
        });
    }
    if profile.bass_available && profile.bass_enabled {
        let gain = format_number(f64::from(profile.bass_level) * 9.0 / 100.0);
        graph.add_builtin_biquad_pair(
            "bass",
            "bq_lowshelf",
            &format!("Freq = 110.0 Q = 0.7 Gain = {gain}"),
        );
    }
    if profile.dialog_available && profile.dialog_enabled {
        let gain = format_number(f64::from(profile.dialog_level) * 6.0 / 100.0);
        graph.add_builtin_biquad_pair(
            "dialog",
            "bq_peaking",
            &format!("Freq = 2500.0 Q = 0.8 Gain = {gain}"),
        );
    }
    if profile.surround_available && profile.surround_enabled {
        let width = u32::from(profile.surround_level) * 6 / 5;
        graph.add_stereo_ladspa(
            "surround",
            "matrix_spatialiser_1422",
            "matrixSpatialiser",
            &format!("\"Width\" = {width}"),
            ("Input L", "Input R"),
            ("Output L", "Output R"),
        );
    }
    if profile.smart_volume_available && profile.smart_volume_enabled {
        let controls =
            smart_volume_controls(&profile.smart_volume_mode, profile.smart_volume_level);
        graph.add_stereo_ladspa(
            "volume",
            "sc4_1882",
            "sc4",
            &controls,
            ("Left input", "Right input"),
            ("Left output", "Right output"),
        );
    }

    if any_child_enabled(profile) {
        graph.add_stereo_ladspa(
            "limiter",
            "fast_lookahead_limiter_1913",
            "fastLookaheadLimiter",
            "\"Input gain (dB)\" = 0.0 \"Limit (dB)\" = -1.0 \
             \"Release time (s)\" = 0.1",
            ("Input 1", "Input 2"),
            ("Output 1", "Output 2"),
        );
    }
    Ok(graph.finish())
}

fn smart_volume_controls(mode: &str, level: u16) -> String {
    let amount = f64::from(level) / 100.0;
    let (threshold, ratio, makeup, attack, release) = match mode {
        "Night" => (
            -18.0 - amount * 10.0,
            5.0 + amount * 5.0,
            2.0 + amount * 2.0,
            20.0,
            300.0,
        ),
        "Loud" => (
            -12.0 - amount * 12.0,
            3.0 + amount * 5.0,
            4.0 + amount * 4.0,
            10.0,
            180.0,
        ),
        _ => (
            -6.0 - amount * 18.0,
            1.0 + amount * 5.0,
            amount * 6.0,
            15.0,
            250.0,
        ),
    };
    format!(
        "\"RMS/peak\" = 0.5 \"Attack time (ms)\" = {} \
         \"Release time (ms)\" = {} \"Threshold level (dB)\" = {} \
         \"Ratio (1:n)\" = {} \"Knee radius (dB)\" = 3.0 \
         \"Makeup gain (dB)\" = {}",
        format_number(attack),
        format_number(release),
        format_number(threshold),
        format_number(ratio),
        format_number(makeup)
    )
}

fn effects_chain_signature(profile: &EffectsProfileEntry, target_node: &str) -> String {
    format!(
        "effects-v1|{target_node}|{}:{}|{}:{}|{}:{}|{}:{}:{}|{}:{}",
        profile.surround_enabled,
        profile.surround_level,
        profile.crystalizer_enabled,
        profile.crystalizer_level,
        profile.bass_enabled,
        profile.bass_level,
        profile.smart_volume_enabled,
        profile.smart_volume_level,
        profile.smart_volume_mode,
        profile.dialog_enabled,
        profile.dialog_level
    )
}

fn any_child_enabled(profile: &EffectsProfileEntry) -> bool {
    (profile.surround_available && profile.surround_enabled)
        || (profile.crystalizer_available && profile.crystalizer_enabled)
        || (profile.bass_available && profile.bass_enabled)
        || (profile.smart_volume_available && profile.smart_volume_enabled)
        || (profile.dialog_available && profile.dialog_enabled)
}

fn validate_profile(profile: &EffectsProfileEntry) -> Result<(), EffectsChainError> {
    profile.validate().map_err(EffectsChainError::Invalid)?;
    for (name, available, enabled) in [
        (
            "Surround",
            profile.surround_available,
            profile.surround_enabled,
        ),
        (
            "Crystalizer",
            profile.crystalizer_available,
            profile.crystalizer_enabled,
        ),
        ("Bass", profile.bass_available, profile.bass_enabled),
        (
            "Smart Volume",
            profile.smart_volume_available,
            profile.smart_volume_enabled,
        ),
        ("Dialog+", profile.dialog_available, profile.dialog_enabled),
    ] {
        if enabled && !available {
            return Err(EffectsChainError::Invalid(format!(
                "{name} is enabled but unavailable in this Effects profile"
            )));
        }
    }
    Ok(())
}

fn set_effects_chain_at(
    path: &Path,
    request: Option<(&EffectsProfileEntry, &str)>,
) -> Result<EffectsChainChange, EffectsChainError> {
    let current = effects_chain_config_at(path)?;
    let Some((profile, target_node)) = request else {
        if !current.enabled {
            return Ok(EffectsChainChange {
                config: current,
                changed: false,
            });
        }
        fs::remove_file(path)?;
        return Ok(EffectsChainChange {
            config: effects_chain_config_at(path)?,
            changed: true,
        });
    };

    validate_profile(profile)?;
    if !profile.outfx_enabled {
        return Err(EffectsChainError::Invalid(
            "the Effects master is disabled; disable the live chain instead".to_owned(),
        ));
    }
    validate_target_node(target_node)?;
    let stored = StoredEffectsChain {
        schema_version: CONFIG_SCHEMA_VERSION,
        target_node: target_node.to_owned(),
        profile: profile.clone(),
    };
    let mut contents = serde_json::to_vec_pretty(&stored)
        .map_err(|error| EffectsChainError::Invalid(error.to_string()))?;
    contents.push(b'\n');
    if current.enabled
        && current.target_node.as_deref() == Some(target_node)
        && current.profile.as_ref() == Some(profile)
    {
        return Ok(EffectsChainChange {
            config: current,
            changed: false,
        });
    }

    let parent = path.parent().ok_or_else(|| {
        EffectsChainError::Invalid(format!("{} has no parent directory", path.display()))
    })?;
    fs::create_dir_all(parent)?;
    if current.enabled {
        replace_file(path, &contents)?;
    } else {
        let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
        file.write_all(&contents)?;
        file.sync_all()?;
    }
    Ok(EffectsChainChange {
        config: effects_chain_config_at(path)?,
        changed: true,
    })
}

fn restore_effects_chain_config_at(
    path: &Path,
    config: &EffectsChainConfig,
) -> Result<EffectsChainChange, EffectsChainError> {
    match (
        config.enabled,
        config.profile.as_ref(),
        config.target_node.as_deref(),
    ) {
        (false, _, _) => set_effects_chain_at(path, None),
        (true, Some(profile), Some(target)) => set_effects_chain_at(path, Some((profile, target))),
        (true, _, _) => Err(EffectsChainError::Invalid(
            "enabled software Effects state is incomplete".to_owned(),
        )),
    }
}

fn effects_chain_config_at(path: &Path) -> Result<EffectsChainConfig, EffectsChainError> {
    match fs::read(path) {
        Ok(contents) => {
            let stored: StoredEffectsChain =
                serde_json::from_slice(&contents).map_err(|_| foreign_config(path))?;
            if stored.schema_version != CONFIG_SCHEMA_VERSION {
                return Err(foreign_config(path));
            }
            validate_target_node(&stored.target_node)?;
            validate_profile(&stored.profile)?;
            if !stored.profile.outfx_enabled {
                return Err(foreign_config(path));
            }
            Ok(EffectsChainConfig {
                path: path.to_owned(),
                enabled: true,
                target_node: Some(stored.target_node),
                profile: Some(stored.profile),
            })
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(EffectsChainConfig {
            path: path.to_owned(),
            enabled: false,
            target_node: None,
            profile: None,
        }),
        Err(error) => Err(error.into()),
    }
}

fn effects_chain_path() -> Result<PathBuf, EffectsChainError> {
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
            EffectsChainError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "neither XDG_CONFIG_HOME nor HOME is available",
            ))
        })
}

fn ladspa_search_paths() -> Vec<PathBuf> {
    let mut paths = env::var_os("LADSPA_PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    for path in [
        "/usr/lib64/ladspa",
        "/usr/lib/ladspa",
        "/usr/local/lib/ladspa",
    ] {
        let path = PathBuf::from(path);
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

fn validate_target_node(target_node: &str) -> Result<(), EffectsChainError> {
    if target_node.is_empty()
        || !target_node
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
    {
        return Err(EffectsChainError::Invalid(format!(
            "PipeWire target node '{target_node}' contains unsupported characters"
        )));
    }
    Ok(())
}

fn format_number(value: f64) -> String {
    let formatted = format!("{value:.3}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed.contains('.') {
        trimmed.to_owned()
    } else {
        format!("{trimmed}.0")
    }
}

fn foreign_config(path: &Path) -> EffectsChainError {
    EffectsChainError::Io(io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!(
            "{} exists but is not managed by AE-5 Control",
            path.display()
        ),
    ))
}

fn replace_file(path: &Path, contents: &[u8]) -> Result<(), EffectsChainError> {
    let parent = path.parent().ok_or_else(|| {
        EffectsChainError::Invalid(format!("{} has no parent directory", path.display()))
    })?;
    let file_name = path
        .file_name()
        .ok_or_else(|| EffectsChainError::Invalid(format!("{} has no file name", path.display())))?
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

impl fmt::Display for EffectsChainError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(output),
            Self::Invalid(message) => output.write_str(message),
        }
    }
}

impl std::error::Error for EffectsChainError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Invalid(_) => None,
        }
    }
}

impl From<io::Error> for EffectsChainError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    fn effects() -> EffectsProfileEntry {
        EffectsProfileEntry {
            id: "effects:test".to_owned(),
            name: "Test Effects".to_owned(),
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

    fn test_path() -> PathBuf {
        std::env::temp_dir()
            .join(format!(
                "ae5-effects-chain-test-{}-{}",
                std::process::id(),
                NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
            ))
            .join(CONFIG_FILE)
    }

    #[test]
    fn effects_use_a_filter_slot_independent_from_equalizer_slot_zero() {
        assert_eq!(effects_filter_slot(), 1);
    }

    #[test]
    fn complete_profile_renders_every_substitute_and_safety_limiter() {
        let graph = render_effects_filter_graph(&effects()).unwrap();

        for expected in [
            "matrixSpatialiser",
            "transient",
            "bq_lowshelf",
            "sc4",
            "bq_peaking",
            "fastLookaheadLimiter",
        ] {
            assert!(
                graph.contains(expected),
                "generated graph does not contain {expected}"
            );
        }
    }

    #[test]
    fn disabled_children_are_absent_from_the_runtime_graph() {
        let mut draft = effects();
        draft.surround_enabled = false;
        draft.crystalizer_enabled = false;
        draft.bass_enabled = false;
        draft.smart_volume_enabled = false;
        draft.dialog_enabled = false;

        let graph = render_effects_filter_graph(&draft).unwrap();

        assert!(!graph.contains("matrixSpatialiser"));
        assert!(!graph.contains("transient"));
        assert!(!graph.contains("bq_lowshelf"));
        assert!(!graph.contains("sc4"));
        assert!(!graph.contains("bq_peaking"));
        assert!(!graph.contains("fastLookaheadLimiter"));
        assert!(graph.contains("linear"));
    }

    #[test]
    fn disabled_master_cannot_render_an_active_graph() {
        let mut draft = effects();
        draft.outfx_enabled = false;

        let error = render_effects_filter_graph(&draft).unwrap_err();

        assert!(error.to_string().contains("master"));
    }

    #[test]
    fn enable_disable_round_trip_removes_the_managed_state() {
        let path = test_path();
        let first = set_effects_chain_at(
            &path,
            Some((&effects(), "alsa_output.pci-ae5.analog-stereo")),
        )
        .unwrap();
        let second = set_effects_chain_at(
            &path,
            Some((&effects(), "alsa_output.pci-ae5.analog-stereo")),
        )
        .unwrap();
        let disabled = set_effects_chain_at(&path, None).unwrap();

        assert!(first.changed);
        assert!(!second.changed);
        assert!(disabled.changed);
        assert!(!path.exists());

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn restoring_previous_state_is_byte_identical() {
        let path = test_path();
        let previous = effects();
        set_effects_chain_at(
            &path,
            Some((&previous, "alsa_output.pci-ae5.analog-stereo")),
        )
        .unwrap();
        let previous_config = effects_chain_config_at(&path).unwrap();
        let previous_bytes = fs::read(&path).unwrap();

        let mut replacement = effects();
        replacement.bass_level = 70;
        set_effects_chain_at(
            &path,
            Some((&replacement, "alsa_output.pci-ae5.analog-stereo")),
        )
        .unwrap();
        restore_effects_chain_config_at(&path, &previous_config).unwrap();

        assert_eq!(fs::read(&path).unwrap(), previous_bytes);
        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn generated_graph_is_accepted_by_pipewire_parser_when_available() {
        let path = test_path().with_file_name("effects-graph.conf");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let graph = render_effects_filter_graph(&effects()).unwrap();
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
                "pw-config rejected generated graph: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                eprintln!("pw-config unavailable; syntax parser check skipped")
            }
            Err(error) => panic!("failed to run pw-config: {error}"),
        }

        fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
