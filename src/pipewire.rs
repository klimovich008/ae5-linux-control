use std::collections::BTreeSet;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

const NATIVE_RATES_CONFIG: &str = "\
# Managed by AE-5 Control.
context.properties = {
    default.clock.allowed-rates = [ 44100 48000 96000 ]
}
";
const AE5_PROFILE_SET: &str = "sound-blaster-ae5.conf";
const SOFTWARE_EQ_SIGNATURE_PROPERTY: &str = "ae5.control.eq.signature";
const SOFTWARE_EFFECTS_SIGNATURE_PROPERTY: &str = "ae5.control.effects.signature";
const PIPEWIRE_SETTINGS_METADATA_NAME: &str = "settings";
const PIPEWIRE_FORCE_RATE_PROPERTY: &str = "clock.force-rate";
const DIRECT_FILTER_PARAMETER: &str = "audioconvert.filter-graph.0";
const EFFECTS_FILTER_PARAMETER: &str = "audioconvert.filter-graph.1";
static NEXT_TRANSITION_SINK: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
struct PreviousFilterState<'a> {
    graph: Option<&'a str>,
    signature: Option<&'a str>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipeWireNode {
    pub id: u32,
    pub node_name: String,
    pub description: String,
    pub is_default: bool,
    pub volume_percent: Option<u16>,
    pub muted: Option<bool>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoftwareEqOutput {
    pub node: PipeWireNode,
    pub signature: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoftwareEffectsOutput {
    pub node: PipeWireNode,
    pub signature: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SoftwareVolumeOutput {
    pub node: PipeWireNode,
    pub applied_percent: f64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipeWireRouteState {
    pub profile_set: Option<String>,
    pub soft_mixer: Option<bool>,
    pub ignore_db: Option<bool>,
    pub persistent_playback: Option<bool>,
    pub active_profile: Option<String>,
    pub input_route: Option<String>,
    pub output_route: Option<String>,
}

pub(crate) struct SuspendedAe5Output {
    card_index: i32,
    resume_on_drop: bool,
    parked_inputs: Option<ParkedSinkInputs>,
}

struct ParkedSinkInputs {
    module_id: u32,
    sink_name: String,
    original_sink_name: String,
    input_ids: Vec<u32>,
}

impl SuspendedAe5Output {
    pub(crate) fn ensure_current_suspended(&mut self) -> io::Result<()> {
        let Some(node) = ae5_output(self.card_index)? else {
            return Ok(());
        };
        if self.parked_inputs.is_none() {
            self.parked_inputs = park_sink_inputs(&node.node_name)?;
        }
        if !pactl_sink_is_suspended(&node.node_name)? {
            run_pactl(&["suspend-sink", &node.node_name, "1"])?;
            self.resume_on_drop = true;
        }
        if let Some(parked) = self.parked_inputs.as_mut() {
            parked.park_additional_inputs()?;
        }
        wait_for_alsa_playback_closed(self.card_index)
    }

    pub(crate) fn resume(mut self) -> io::Result<()> {
        self.restore()
    }

    fn restore(&mut self) -> io::Result<()> {
        let resume = if self.resume_on_drop {
            match ae5_output(self.card_index)? {
                Some(node) => run_pactl(&["suspend-sink", &node.node_name, "0"]).map(|_| ()),
                None => Ok(()),
            }
        } else {
            Ok(())
        };
        if resume.is_ok() {
            self.resume_on_drop = false;
        }
        let streams = self
            .parked_inputs
            .as_mut()
            .map(ParkedSinkInputs::restore)
            .unwrap_or(Ok(()));
        if streams.is_ok() {
            self.parked_inputs = None;
        }
        combine_transition_results(resume, streams)
    }
}

impl Drop for SuspendedAe5Output {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

impl ParkedSinkInputs {
    fn park_additional_inputs(&mut self) -> io::Result<()> {
        let sink_index = pactl_sink_index(&self.original_sink_name)?;
        for input_id in pactl_sink_inputs(sink_index)? {
            if self.input_ids.contains(&input_id) {
                continue;
            }
            move_sink_input(input_id, &self.sink_name)?;
            self.input_ids.push(input_id);
        }
        Ok(())
    }

    fn restore(&mut self) -> io::Result<()> {
        let mut errors = Vec::new();
        match pactl_sink_input_ids() {
            Ok(existing) => {
                for input_id in &self.input_ids {
                    if existing.contains(input_id)
                        && let Err(error) = move_sink_input(*input_id, &self.original_sink_name)
                    {
                        errors.push(format!("stream {input_id}: {error}"));
                    }
                }
            }
            Err(error) => errors.push(format!("enumerating parked streams: {error}")),
        }
        if let Err(error) = run_pactl(&["unload-module", &self.module_id.to_string()]) {
            errors.push(format!("silent transition sink: {error}"));
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "restoring parked audio streams failed ({})",
                errors.join("; ")
            )))
        }
    }
}

impl PipeWireRouteState {
    pub fn output_issue(&self, output_choice: &str, speaker_layout: &str) -> Option<String> {
        let required_profile = match output_profile_component(output_choice, speaker_layout) {
            Ok(profile) => profile,
            Err(error) => return Some(error.to_string()),
        };
        if let Some(issue) = self.profile_issue(required_profile, "output") {
            return Some(issue);
        }
        if self.persistent_playback == Some(false) {
            return Some(
                "the AE-5 output is missing session.suspend-timeout-seconds=0; restart \
                 WirePlumber after installing the exact-card policy"
                    .to_owned(),
            );
        }
        if output_choice == "Speakers" && speaker_layout != "2.0" {
            return None;
        }
        let expected = match output_choice {
            "Speakers" => "analog-output-lineout;output-speaker",
            "Headphone" => "sound-blaster-ae5-output-headphones",
            other => return Some(format!("unsupported ALSA output choice '{other}'")),
        };
        (self.output_route.as_deref() != Some(expected)).then(|| {
            format!(
                "ALSA selects {output_choice}, but PipeWire uses {}; reapply the output choice",
                self.output_route.as_deref().unwrap_or("no output route")
            )
        })
    }

    pub fn input_issue(&self, input_choice: &str) -> Option<String> {
        if let Some(issue) = self.profile_issue("input:analog-stereo", "input") {
            return Some(issue);
        }
        let expected = match input_choice {
            "Microphone" => "sound-blaster-ae5-input-microphone",
            "Front Microphone" => "sound-blaster-ae5-input-front-microphone",
            "Line In" => "sound-blaster-ae5-input-line-in",
            other => return Some(format!("unsupported ALSA input choice '{other}'")),
        };
        (self.input_route.as_deref() != Some(expected)).then(|| {
            format!(
                "ALSA selects {input_choice}, but PipeWire uses {}; reapply the input choice",
                self.input_route.as_deref().unwrap_or("no input route")
            )
        })
    }

    fn profile_issue(&self, required: &str, direction: &str) -> Option<String> {
        if self.profile_set.as_deref() != Some(AE5_PROFILE_SET) {
            return Some(format!(
                "PipeWire is not using {AE5_PROFILE_SET}; install the AE-5 routing profile and restart WirePlumber"
            ));
        }
        if self.soft_mixer != Some(true) {
            return Some(
                "PipeWire hardware volume control is unsafe for the AE-5; enable api.alsa.soft-mixer and restart WirePlumber"
                    .to_owned(),
            );
        }
        if self.ignore_db != Some(true) {
            return Some(
                "PipeWire must ignore the AE-5 driver's invalid dB metadata; enable api.alsa.ignore-dB and restart WirePlumber"
                    .to_owned(),
            );
        }
        if !self
            .active_profile
            .as_deref()
            .is_some_and(|profile| profile.split('+').any(|part| part == required))
        {
            return Some(format!(
                "the current PipeWire profile is {}; expected {required} for {direction}",
                self.active_profile.as_deref().unwrap_or("unavailable"),
            ));
        }
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeRatesConfig {
    pub path: PathBuf,
    pub enabled: bool,
}

/// Live PipeWire graph-rate override for the current desktop session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeSampleRate {
    /// Follow PipeWire's configured clock policy.
    Auto,
    /// Force the graph and AE-5 sink to 48 kHz.
    Hz48000,
    /// Force the graph and AE-5 sink to 96 kHz.
    Hz96000,
}

impl RuntimeSampleRate {
    pub const fn policy_name(self) -> &'static str {
        match self {
            Self::Auto => "Automatic",
            Self::Hz48000 => "48 kHz",
            Self::Hz96000 => "96 kHz",
        }
    }

    pub fn from_policy_name(value: &str) -> Option<Self> {
        match value {
            "Automatic" => Some(Self::Auto),
            "48 kHz" => Some(Self::Hz48000),
            "96 kHz" => Some(Self::Hz96000),
            _ => None,
        }
    }

    fn metadata_value(self) -> &'static str {
        match self {
            Self::Auto => "0",
            Self::Hz48000 => "48000",
            Self::Hz96000 => "96000",
        }
    }

    fn hertz(self) -> Option<u32> {
        match self {
            Self::Auto => None,
            Self::Hz48000 => Some(48_000),
            Self::Hz96000 => Some(96_000),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioFormat {
    pub sample_format: String,
    pub sample_rate: u32,
}

pub fn ae5_output(card_index: i32) -> io::Result<Option<PipeWireNode>> {
    ae5_node(card_index, "sinks")
}

/// Reads the active PipeWire transport format without opening or changing the
/// AE-5 playback device.
pub fn ae5_audio_format(card_index: i32) -> io::Result<Option<AudioFormat>> {
    let Some(node) = ae5_output(card_index)? else {
        return Ok(None);
    };
    optional_active_pcm_format(active_pcm_format(node.id))
}

fn optional_active_pcm_format(result: io::Result<AudioFormat>) -> io::Result<Option<AudioFormat>> {
    match result {
        Ok(format) => Ok(Some(format)),
        Err(error)
            if error.kind() == io::ErrorKind::InvalidData
                && error
                    .to_string()
                    .starts_with("PipeWire did not report the AE-5") =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

pub fn ae5_input(card_index: i32) -> io::Result<Option<PipeWireNode>> {
    ae5_node(card_index, "sources")
}

pub fn ae5_windows_volume_curve_active(card_index: i32) -> io::Result<bool> {
    let node = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no playback output for ALSA card {card_index}"),
        )
    })?;
    let details = run_wpctl(&["inspect", &node.id.to_string()])?;
    Ok(has_windows_audio_taper(&details))
}

pub fn set_ae5_default_output(card_index: i32) -> io::Result<PipeWireNode> {
    set_ae5_default_node(card_index, "sinks", "playback output")
}

pub fn set_ae5_default_input(card_index: i32) -> io::Result<PipeWireNode> {
    set_ae5_default_node(card_index, "sources", "recording input")
}

pub fn set_ae5_software_volume(card_index: i32, percent: f64) -> io::Result<SoftwareVolumeOutput> {
    validate_software_volume(percent)?;
    let mut node = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no playback output for ALSA card {card_index}"),
        )
    })?;
    require_ae5_profile(node.id)?;
    let (applied_percent, muted) = set_software_volume_with(&node, percent, run_wpctl)?;
    node.volume_percent = Some(applied_percent.round() as u16);
    node.muted = Some(muted);
    Ok(SoftwareVolumeOutput {
        node,
        applied_percent,
    })
}

pub fn set_ae5_software_mute(card_index: i32, muted: bool) -> io::Result<PipeWireNode> {
    let mut node = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no playback output for ALSA card {card_index}"),
        )
    })?;
    require_ae5_profile(node.id)?;
    let (applied_percent, applied_mute) = set_software_mute_with(&node, muted, run_wpctl)?;
    node.volume_percent = Some(applied_percent.round() as u16);
    node.muted = Some(applied_mute);
    Ok(node)
}

/// Reads the live PipeWire graph-rate override.
///
/// # Errors
///
/// Returns an error when PipeWire metadata is unavailable or contains an
/// unsupported forced rate.
pub fn runtime_sample_rate() -> io::Result<RuntimeSampleRate> {
    parse_runtime_sample_rate(&run_pw_metadata(&["-n", PIPEWIRE_SETTINGS_METADATA_NAME])?)
}

/// Safely applies a live graph-rate override while preserving AE-5 mute state.
///
/// The setting lasts until PipeWire restarts. The exact-card S16 transport is
/// verified before an originally unmuted output is restored.
///
/// # Errors
///
/// Returns an error when the AE-5 PipeWire route is unavailable, the metadata
/// write fails, or the sink does not negotiate the requested S16 rate.
pub fn set_ae5_runtime_sample_rate(
    card_index: i32,
    requested: RuntimeSampleRate,
) -> io::Result<RuntimeSampleRate> {
    let node = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no playback output for ALSA card {card_index}"),
        )
    })?;
    require_ae5_profile(node.id)?;
    let mut suspended_output = None;
    set_runtime_sample_rate_with(
        &node,
        requested,
        run_wpctl,
        run_pw_metadata,
        |suspended| {
            if suspended {
                if suspended_output.is_none() {
                    suspended_output = Some(suspend_ae5_output(card_index)?);
                }
            } else if let Some(output) = suspended_output.take() {
                output.resume()?;
            }
            Ok(())
        },
        |rate| verify_runtime_sample_rate(node.id, &node.node_name, rate),
    )
}

fn set_runtime_sample_rate_with<W, M, S, V>(
    node: &PipeWireNode,
    requested: RuntimeSampleRate,
    mut run_wpctl: W,
    mut run_metadata: M,
    mut set_suspended: S,
    mut verify: V,
) -> io::Result<RuntimeSampleRate>
where
    W: FnMut(&[&str]) -> io::Result<String>,
    M: FnMut(&[&str]) -> io::Result<String>,
    S: FnMut(bool) -> io::Result<()>,
    V: FnMut(RuntimeSampleRate) -> io::Result<()>,
{
    let before =
        parse_runtime_sample_rate(&run_metadata(&["-n", PIPEWIRE_SETTINGS_METADATA_NAME])?)?;
    if before == requested {
        verify(requested)?;
        return Ok(requested);
    }
    let muted = node.muted.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the AE-5 output mute state is unavailable; refusing a sample-rate change",
        )
    })?;
    let node_id = node.id.to_string();
    run_wpctl(&["set-mute", &node_id, "1"])?;

    let transition: io::Result<RuntimeSampleRate> = (|| {
        set_suspended(true)?;
        let update = (|| {
            run_metadata(&[
                "-n",
                PIPEWIRE_SETTINGS_METADATA_NAME,
                "0",
                PIPEWIRE_FORCE_RATE_PROPERTY,
                requested.metadata_value(),
            ])?;
            let actual = parse_runtime_sample_rate(&run_metadata(&[
                "-n",
                PIPEWIRE_SETTINGS_METADATA_NAME,
            ])?)?;
            if actual != requested {
                return Err(io::Error::other(format!(
                    "PipeWire retained {actual:?} after requesting {requested:?}"
                )));
            }
            Ok(())
        })();
        let resumed = set_suspended(false);
        update?;
        resumed?;
        verify(requested)?;
        run_wpctl(&["set-mute", &node_id, if muted { "1" } else { "0" }])?;
        Ok(requested)
    })();
    if let Err(error) = transition {
        let rollback: io::Result<()> = (|| {
            set_suspended(true)?;
            let update = (|| {
                run_metadata(&[
                    "-n",
                    PIPEWIRE_SETTINGS_METADATA_NAME,
                    "0",
                    PIPEWIRE_FORCE_RATE_PROPERTY,
                    before.metadata_value(),
                ])?;
                let actual = parse_runtime_sample_rate(&run_metadata(&[
                    "-n",
                    PIPEWIRE_SETTINGS_METADATA_NAME,
                ])?)?;
                if actual != before {
                    return Err(io::Error::other(format!(
                        "PipeWire retained {actual:?} while restoring {before:?}"
                    )));
                }
                Ok(())
            })();
            let resumed = set_suspended(false);
            update?;
            resumed?;
            verify(before)
        })();
        let safe_mute = run_wpctl(&["set-mute", &node_id, "1"]);
        return Err(io::Error::new(
            error.kind(),
            format!(
                "{error}; rate rollback {}; output mute {}",
                if rollback.is_ok() {
                    "verified"
                } else {
                    "failed"
                },
                if safe_mute.is_ok() {
                    "confirmed"
                } else {
                    "failed"
                }
            ),
        ));
    }
    transition
}

fn verify_runtime_sample_rate(
    node_id: u32,
    node_name: &str,
    requested: RuntimeSampleRate,
) -> io::Result<()> {
    verify_runtime_sample_rate_with(
        requested,
        || active_pcm_format(node_id),
        |rate| prime_ae5_output(node_name, rate),
    )
}

fn verify_runtime_sample_rate_with<Q, P>(
    requested: RuntimeSampleRate,
    mut query: Q,
    mut prime: P,
) -> io::Result<()>
where
    Q: FnMut() -> io::Result<AudioFormat>,
    P: FnMut(u32) -> io::Result<()>,
{
    let mut last = None;
    let mut primed = false;
    for _ in 0..120 {
        match query() {
            Ok(format) if format.sample_format != "S16LE" => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "AE-5 negotiated {}, expected the qualified S16LE transport",
                        format.sample_format
                    ),
                ));
            }
            Ok(format)
                if requested
                    .hertz()
                    .is_none_or(|expected| format.sample_rate == expected) =>
            {
                return Ok(());
            }
            Ok(format) => last = Some(format!("{} Hz", format.sample_rate)),
            Err(error) if !primed => {
                let rate = requested.hertz().unwrap_or(48_000);
                prime(rate)?;
                primed = true;
                last = Some(error.to_string());
                continue;
            }
            Err(error) => last = Some(error.to_string()),
        }
        thread::sleep(Duration::from_millis(25));
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!(
            "AE-5 did not negotiate {}; last state: {}",
            requested
                .hertz()
                .map_or_else(|| "automatic rate".to_owned(), |rate| format!("{rate} Hz")),
            last.unwrap_or_else(|| "unavailable".to_owned())
        ),
    ))
}

fn prime_ae5_output(node_name: &str, sample_rate: u32) -> io::Result<()> {
    let sample_count = (sample_rate / 10).to_string();
    let sample_rate = sample_rate.to_string();
    let arguments = [
        "--raw",
        "--target",
        node_name,
        "--rate",
        &sample_rate,
        "--channels",
        "2",
        "--format",
        "s16",
        "--volume",
        "0",
        "--sample-count",
        &sample_count,
        "/dev/zero",
    ];
    let _status = Command::new("pw-play")
        .args(arguments)
        .status()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "pw-play is unavailable; install PipeWire utilities",
                )
            } else {
                error
            }
        })?;
    // pw-play can return 1 after its bounded stream has successfully activated
    // the sink. The following ALSA format query is the authoritative result.
    Ok(())
}

fn active_pcm_format(node_id: u32) -> io::Result<AudioFormat> {
    parse_active_pcm_format(&run_pw_cli(&[
        "enum-params",
        &node_id.to_string(),
        "Format",
    ])?)
}

pub fn software_eq_output(card_index: i32) -> io::Result<Option<SoftwareEqOutput>> {
    let Some(node) = ae5_output(card_index)? else {
        return Ok(None);
    };
    Ok(
        software_eq_signature(node.id)?.map(|signature| SoftwareEqOutput {
            node,
            signature: Some(signature),
        }),
    )
}

pub fn software_effects_output(card_index: i32) -> io::Result<Option<SoftwareEffectsOutput>> {
    let Some(node) = ae5_output(card_index)? else {
        return Ok(None);
    };
    Ok(
        software_effects_signature(node.id)?.map(|signature| SoftwareEffectsOutput {
            node,
            signature: Some(signature),
        }),
    )
}

pub fn apply_software_eq(
    card_index: i32,
    graph: &str,
    signature: &str,
) -> io::Result<SoftwareEqOutput> {
    if graph.is_empty() || signature.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "software equalizer graph and signature must not be empty",
        ));
    }
    let output = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no playback output for ALSA card {card_index}"),
        )
    })?;
    require_direct_filter_support(output.id)?;
    let suspended = suspend_ae5_output(card_index)?;
    set_direct_filter(output.id, graph)?;
    if let Err(error) = set_software_eq_signature(output.id, Some(signature))
        .and_then(|_| require_software_eq_signature(output.id, Some(signature)))
    {
        let _ = set_direct_filter(output.id, "");
        let _ = set_software_eq_signature(output.id, None);
        return Err(error);
    }
    suspended.resume()?;
    let node = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "PipeWire lost the AE-5 output after loading the software equalizer",
        )
    })?;
    Ok(SoftwareEqOutput {
        node,
        signature: Some(signature.to_owned()),
    })
}

pub fn replace_software_eq(
    card_index: i32,
    graph: &str,
    signature: &str,
    previous_graph: Option<&str>,
    previous_signature: Option<&str>,
) -> io::Result<SoftwareEqOutput> {
    let output = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no playback output for ALSA card {card_index}"),
        )
    })?;
    require_direct_filter_support(output.id)?;
    let suspended = suspend_ae5_output(card_index)?;
    replace_software_filter_state_with(
        "software EQ",
        graph,
        signature,
        PreviousFilterState {
            graph: previous_graph,
            signature: previous_signature,
        },
        |graph| set_direct_filter(output.id, graph),
        |signature| set_software_eq_signature(output.id, signature),
        || software_eq_signature(output.id),
    )?;
    if let Err(error) = suspended.resume() {
        return rollback_replaced_software_eq(
            card_index,
            signature,
            previous_graph,
            previous_signature,
            error,
        );
    }
    let current = match software_eq_output(card_index) {
        Ok(Some(current)) => current,
        Ok(None) => {
            return rollback_replaced_software_eq(
                card_index,
                signature,
                previous_graph,
                previous_signature,
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "PipeWire lost the AE-5 software equalizer after applying it",
                ),
            );
        }
        Err(error) => {
            return rollback_replaced_software_eq(
                card_index,
                signature,
                previous_graph,
                previous_signature,
                error,
            );
        }
    };
    if current.node.id != output.id || current.signature.as_deref() != Some(signature) {
        return rollback_replaced_software_eq(
            card_index,
            signature,
            previous_graph,
            previous_signature,
            io::Error::other(
                "PipeWire changed the AE-5 output identity or EQ marker after applying the graph",
            ),
        );
    }
    Ok(current)
}

fn rollback_replaced_software_eq(
    card_index: i32,
    applied_signature: &str,
    previous_graph: Option<&str>,
    previous_signature: Option<&str>,
    apply_error: io::Error,
) -> io::Result<SoftwareEqOutput> {
    match restore_software_eq_runtime(
        card_index,
        Some(applied_signature),
        previous_graph,
        previous_signature,
    ) {
        Ok(()) => Err(apply_error),
        Err(rollback_error) => Err(io::Error::other(format!(
            "{apply_error}; software EQ rollback also failed: {rollback_error}"
        ))),
    }
}

fn restore_software_eq_runtime(
    card_index: i32,
    expected_signature: Option<&str>,
    graph: Option<&str>,
    signature: Option<&str>,
) -> io::Result<()> {
    if graph.is_some() != signature.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "restored software equalizer graph and signature must be provided together",
        ));
    }
    let output = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "PipeWire lost the AE-5 output before software EQ rollback",
        )
    })?;
    require_direct_filter_support(output.id)?;
    let actual = software_eq_signature(output.id)?;
    if actual.as_deref() != expected_signature {
        return Err(io::Error::other(format!(
            "refusing software EQ rollback because the active marker changed (expected {}, found {})",
            expected_signature.unwrap_or("none"),
            actual.as_deref().unwrap_or("none")
        )));
    }

    let suspended = suspend_ae5_output(card_index)?;
    let rollback = set_direct_filter(output.id, graph.unwrap_or(""))
        .and_then(|_| set_software_eq_signature(output.id, signature))
        .and_then(|_| require_software_eq_signature(output.id, signature));
    let resume = suspended.resume();
    match (rollback, resume) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(resume_error)) => Err(io::Error::other(format!(
            "{error}; resuming the AE-5 output also failed: {resume_error}"
        ))),
    }
}

pub fn remove_software_eq(
    card_index: i32,
    previous_graph: &str,
    previous_signature: &str,
) -> io::Result<PipeWireNode> {
    if previous_graph.is_empty() || previous_signature.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "previous software equalizer graph and signature must not be empty",
        ));
    }
    let output = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no playback output for ALSA card {card_index}"),
        )
    })?;
    require_direct_filter_support(output.id)?;
    if software_eq_signature(output.id)?.as_deref() != Some(previous_signature) {
        return Err(io::Error::other(
            "the active software EQ marker changed outside AE-5 Control",
        ));
    }

    let suspended = suspend_ae5_output(card_index)?;
    let removed = set_direct_filter(output.id, "")
        .and_then(|_| set_software_eq_signature(output.id, None))
        .and_then(|_| require_software_eq_signature(output.id, None));
    if let Err(remove_error) = removed {
        let rollback = set_direct_filter(output.id, previous_graph)
            .and_then(|_| set_software_eq_signature(output.id, Some(previous_signature)))
            .and_then(|_| require_software_eq_signature(output.id, Some(previous_signature)));
        return match rollback {
            Ok(()) => Err(remove_error),
            Err(rollback_error) => Err(io::Error::other(format!(
                "{remove_error}; software EQ rollback also failed: {rollback_error}"
            ))),
        };
    }
    if let Err(error) = suspended.resume() {
        restore_software_eq_runtime(
            card_index,
            None,
            Some(previous_graph),
            Some(previous_signature),
        )
        .map_err(|rollback_error| {
            io::Error::other(format!(
                "{error}; software EQ rollback also failed: {rollback_error}"
            ))
        })?;
        return Err(error);
    }
    match software_eq_output(card_index) {
        Ok(None) => ae5_output(card_index)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "PipeWire lost the AE-5 output after disabling software EQ",
            )
        }),
        Ok(Some(_)) => {
            let error = io::Error::other(
                "PipeWire still reports an active AE-5 software EQ after disabling it",
            );
            restore_software_eq_runtime(
                card_index,
                None,
                Some(previous_graph),
                Some(previous_signature),
            )
            .map_err(|rollback_error| {
                io::Error::other(format!(
                    "{error}; software EQ rollback also failed: {rollback_error}"
                ))
            })?;
            Err(error)
        }
        Err(error) => {
            restore_software_eq_runtime(
                card_index,
                None,
                Some(previous_graph),
                Some(previous_signature),
            )
            .map_err(|rollback_error| {
                io::Error::other(format!(
                    "{error}; software EQ rollback also failed: {rollback_error}"
                ))
            })?;
            Err(error)
        }
    }
}

pub fn replace_software_effects(
    card_index: i32,
    graph: &str,
    signature: &str,
    previous_graph: Option<&str>,
    previous_signature: Option<&str>,
) -> io::Result<SoftwareEffectsOutput> {
    let output = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no playback output for ALSA card {card_index}"),
        )
    })?;
    require_direct_filter_support(output.id)?;
    let suspended = suspend_ae5_output(card_index)?;
    replace_software_filter_state_with(
        "software Effects",
        graph,
        signature,
        PreviousFilterState {
            graph: previous_graph,
            signature: previous_signature,
        },
        |graph| set_effects_filter(output.id, graph),
        |signature| set_software_effects_signature(output.id, signature),
        || software_effects_signature(output.id),
    )?;
    if let Err(error) = suspended.resume() {
        return rollback_replaced_software_effects(
            card_index,
            signature,
            previous_graph,
            previous_signature,
            error,
        );
    }
    let current = match software_effects_output(card_index) {
        Ok(Some(current)) => current,
        Ok(None) => {
            return rollback_replaced_software_effects(
                card_index,
                signature,
                previous_graph,
                previous_signature,
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "PipeWire lost the AE-5 software Effects graph after applying it",
                ),
            );
        }
        Err(error) => {
            return rollback_replaced_software_effects(
                card_index,
                signature,
                previous_graph,
                previous_signature,
                error,
            );
        }
    };
    if current.node.id != output.id || current.signature.as_deref() != Some(signature) {
        return rollback_replaced_software_effects(
            card_index,
            signature,
            previous_graph,
            previous_signature,
            io::Error::other(
                "PipeWire changed the AE-5 output identity or Effects marker after applying the graph",
            ),
        );
    }
    Ok(current)
}

fn rollback_replaced_software_effects(
    card_index: i32,
    applied_signature: &str,
    previous_graph: Option<&str>,
    previous_signature: Option<&str>,
    apply_error: io::Error,
) -> io::Result<SoftwareEffectsOutput> {
    match restore_software_effects_runtime(
        card_index,
        Some(applied_signature),
        previous_graph,
        previous_signature,
    ) {
        Ok(()) => Err(apply_error),
        Err(rollback_error) => Err(io::Error::other(format!(
            "{apply_error}; software Effects rollback also failed: {rollback_error}"
        ))),
    }
}

fn restore_software_effects_runtime(
    card_index: i32,
    expected_signature: Option<&str>,
    graph: Option<&str>,
    signature: Option<&str>,
) -> io::Result<()> {
    if graph.is_some() != signature.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "restored software Effects graph and signature must be provided together",
        ));
    }
    let output = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "PipeWire lost the AE-5 output before software Effects rollback",
        )
    })?;
    require_direct_filter_support(output.id)?;
    let actual = software_effects_signature(output.id)?;
    if actual.as_deref() != expected_signature {
        return Err(io::Error::other(format!(
            "refusing software Effects rollback because the active marker changed (expected {}, found {})",
            expected_signature.unwrap_or("none"),
            actual.as_deref().unwrap_or("none")
        )));
    }

    let suspended = suspend_ae5_output(card_index)?;
    let rollback = set_effects_filter(output.id, graph.unwrap_or(""))
        .and_then(|_| set_software_effects_signature(output.id, signature))
        .and_then(|_| require_software_effects_signature(output.id, signature));
    let resume = suspended.resume();
    match (rollback, resume) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(error), Err(resume_error)) => Err(io::Error::other(format!(
            "{error}; resuming the AE-5 output also failed: {resume_error}"
        ))),
    }
}

pub fn remove_software_effects(
    card_index: i32,
    previous_graph: &str,
    previous_signature: &str,
) -> io::Result<PipeWireNode> {
    if previous_graph.is_empty() || previous_signature.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "previous software Effects graph and signature must not be empty",
        ));
    }
    let output = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no playback output for ALSA card {card_index}"),
        )
    })?;
    require_direct_filter_support(output.id)?;
    if software_effects_signature(output.id)?.as_deref() != Some(previous_signature) {
        return Err(io::Error::other(
            "the active software Effects marker changed outside AE-5 Control",
        ));
    }

    let suspended = suspend_ae5_output(card_index)?;
    let removed = set_effects_filter(output.id, "")
        .and_then(|_| set_software_effects_signature(output.id, None))
        .and_then(|_| require_software_effects_signature(output.id, None));
    if let Err(remove_error) = removed {
        let rollback = set_effects_filter(output.id, previous_graph)
            .and_then(|_| set_software_effects_signature(output.id, Some(previous_signature)))
            .and_then(|_| require_software_effects_signature(output.id, Some(previous_signature)));
        return match rollback {
            Ok(()) => Err(remove_error),
            Err(rollback_error) => Err(io::Error::other(format!(
                "{remove_error}; software Effects rollback also failed: {rollback_error}"
            ))),
        };
    }
    if let Err(error) = suspended.resume() {
        restore_software_effects_runtime(
            card_index,
            None,
            Some(previous_graph),
            Some(previous_signature),
        )
        .map_err(|rollback_error| {
            io::Error::other(format!(
                "{error}; software Effects rollback also failed: {rollback_error}"
            ))
        })?;
        return Err(error);
    }
    match software_effects_output(card_index) {
        Ok(None) => ae5_output(card_index)?.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "PipeWire lost the AE-5 output after disabling software Effects",
            )
        }),
        Ok(Some(_)) => {
            let error = io::Error::other(
                "PipeWire still reports active AE-5 software Effects after disabling them",
            );
            restore_software_effects_runtime(
                card_index,
                None,
                Some(previous_graph),
                Some(previous_signature),
            )
            .map_err(|rollback_error| {
                io::Error::other(format!(
                    "{error}; software Effects rollback also failed: {rollback_error}"
                ))
            })?;
            Err(error)
        }
        Err(error) => {
            restore_software_effects_runtime(
                card_index,
                None,
                Some(previous_graph),
                Some(previous_signature),
            )
            .map_err(|rollback_error| {
                io::Error::other(format!(
                    "{error}; software Effects rollback also failed: {rollback_error}"
                ))
            })?;
            Err(error)
        }
    }
}

fn replace_software_filter_state_with<SetFilter, SetSignature, ReadSignature>(
    filter_name: &str,
    graph: &str,
    signature: &str,
    previous: PreviousFilterState<'_>,
    mut set_filter: SetFilter,
    mut set_signature: SetSignature,
    mut read_signature: ReadSignature,
) -> io::Result<()>
where
    SetFilter: FnMut(&str) -> io::Result<()>,
    SetSignature: FnMut(Option<&str>) -> io::Result<()>,
    ReadSignature: FnMut() -> io::Result<Option<String>>,
{
    if graph.is_empty() || signature.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{filter_name} graph and signature must not be empty"),
        ));
    }
    if previous.graph.is_some() != previous.signature.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("previous {filter_name} graph and signature must be provided together"),
        ));
    }
    let current = read_signature()?;
    if current.as_deref() != previous.signature {
        return Err(io::Error::other(format!(
            "the active {filter_name} marker changed outside AE-5 Control (expected {}, found {})",
            previous.signature.unwrap_or("none"),
            current.as_deref().unwrap_or("none")
        )));
    }

    let applied = set_filter(graph)
        .and_then(|_| set_signature(Some(signature)))
        .and_then(|_| {
            let actual = read_signature()?;
            if actual.as_deref() == Some(signature) {
                Ok(())
            } else {
                Err(io::Error::other(format!(
                    "PipeWire did not retain the {filter_name} marker (expected {signature}, found {})",
                    actual.as_deref().unwrap_or("none")
                )))
            }
        });
    if let Err(apply_error) = applied {
        let rollback = set_filter(previous.graph.unwrap_or(""))
            .and_then(|_| set_signature(previous.signature))
            .and_then(|_| {
                let actual = read_signature()?;
                if actual.as_deref() == previous.signature {
                    Ok(())
                } else {
                    Err(io::Error::other(format!(
                        "rollback marker read back as {}, expected {}",
                        actual.as_deref().unwrap_or("none"),
                        previous.signature.unwrap_or("none")
                    )))
                }
            });
        return match rollback {
            Ok(()) => Err(apply_error),
            Err(rollback_error) => Err(io::Error::other(format!(
                "{apply_error}; {filter_name} rollback also failed: {rollback_error}"
            ))),
        };
    }
    Ok(())
}

pub fn unload_software_eq(card_index: i32) -> io::Result<PipeWireNode> {
    let output = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no playback output for ALSA card {card_index}"),
        )
    })?;
    require_direct_filter_support(output.id)?;
    let suspended = suspend_ae5_output(card_index)?;
    set_direct_filter(output.id, "")?;
    set_software_eq_signature(output.id, None)?;
    require_software_eq_signature(output.id, None)?;
    suspended.resume()?;
    ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "PipeWire lost the AE-5 output after unloading the software equalizer",
        )
    })
}

pub(crate) fn suspend_ae5_output(card_index: i32) -> io::Result<SuspendedAe5Output> {
    let Some(node) = ae5_output(card_index)? else {
        return Ok(SuspendedAe5Output {
            card_index,
            resume_on_drop: false,
            parked_inputs: None,
        });
    };
    if pactl_sink_is_suspended(&node.node_name)? {
        wait_for_alsa_playback_closed(card_index)?;
        return Ok(SuspendedAe5Output {
            card_index,
            resume_on_drop: false,
            parked_inputs: None,
        });
    }

    let parked_inputs = park_sink_inputs(&node.node_name)?;
    let mut suspended = SuspendedAe5Output {
        card_index,
        resume_on_drop: false,
        parked_inputs,
    };
    if let Err(error) = run_pactl(&["suspend-sink", &node.node_name, "1"]) {
        return match suspended.restore() {
            Ok(()) => Err(error),
            Err(restore_error) => Err(io::Error::other(format!(
                "{error}; transition stream rollback also failed: {restore_error}"
            ))),
        };
    }
    suspended.resume_on_drop = true;
    if let Some(parked) = suspended.parked_inputs.as_mut() {
        parked.park_additional_inputs()?;
    }
    wait_for_alsa_playback_closed(card_index)?;
    Ok(suspended)
}

pub fn ae5_route_state(card_index: i32) -> io::Result<PipeWireRouteState> {
    let mut state = parse_route_state(&run_pw_dump()?, card_index)?;
    state.persistent_playback = match ae5_output(card_index)? {
        Some(node) => {
            let details = run_wpctl(&["inspect", &node.id.to_string()])?;
            Some(property(&details, "session.suspend-timeout-seconds").as_deref() == Some("0"))
        }
        None => None,
    };
    Ok(state)
}

pub(crate) fn set_ae5_output_profile(
    card_index: i32,
    output_choice: &str,
    speaker_layout: &str,
) -> io::Result<Option<String>> {
    let card = ae5_card_profile(card_index)?;
    let target = output_profile(
        output_choice,
        speaker_layout,
        card.active_profile.contains("+input:analog-stereo"),
    )?;
    if card.active_profile == target {
        return Ok(None);
    }
    set_card_profile(&card, &target)?;
    Ok(Some(card.active_profile))
}

pub(crate) fn restore_ae5_output_profile(card_index: i32, previous: &str) -> io::Result<()> {
    let card = ae5_card_profile(card_index)?;
    if card.active_profile != previous {
        set_card_profile(&card, previous)?;
    }
    Ok(())
}

pub(crate) fn set_ae5_control_route(
    card_index: i32,
    control: &str,
    choice: &str,
) -> io::Result<bool> {
    let Some((nodes, direction, route_name)) = ae5_control_route(control, choice) else {
        return Ok(false);
    };
    let node = ae5_node(card_index, nodes)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no AE-5 {nodes} node for ALSA card {card_index}"),
        )
    })?;
    require_ae5_profile(node.id)?;
    let route = parse_route_index(&run_pw_dump()?, card_index, direction, route_name)?;
    if nodes == "sinks" {
        set_output_route_with(&node, route, run_wpctl)?;
    } else {
        run_wpctl(&["set-route", &node.id.to_string(), &route.to_string()])?;
    }
    Ok(true)
}

fn set_output_route_with<F>(node: &PipeWireNode, route: u32, mut run: F) -> io::Result<()>
where
    F: FnMut(&[&str]) -> io::Result<String>,
{
    let volume = node.volume_percent.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the AE-5 output volume is unavailable; refusing an unsafe route change",
        )
    })?;
    let muted = node.muted.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the AE-5 output mute state is unavailable; refusing an unsafe route change",
        )
    })?;
    let node_id = node.id.to_string();
    let route = route.to_string();
    let volume = format!("{volume}%");
    let muted = if muted { "1" } else { "0" };

    run(&["set-mute", &node_id, "1"])?;
    let transition = (|| -> io::Result<()> {
        run(&["set-route", &node_id, &route])?;
        run(&["set-volume", &node_id, &volume])?;
        run(&["set-mute", &node_id, muted])?;
        Ok(())
    })();
    if let Err(error) = transition {
        let detail = match run(&["set-mute", &node_id, "1"]) {
            Ok(_) => format!("{error}; the AE-5 output was left muted"),
            Err(mute_error) => {
                format!("{error}; failed to force the AE-5 output muted: {mute_error}")
            }
        };
        return Err(io::Error::new(error.kind(), detail));
    }
    Ok(())
}

fn set_software_volume_with<F>(
    node: &PipeWireNode,
    percent: f64,
    mut run: F,
) -> io::Result<(f64, bool)>
where
    F: FnMut(&[&str]) -> io::Result<String>,
{
    validate_software_volume(percent)?;
    let node_id = node.id.to_string();
    let before = parse_wpctl_volume(&run(&["get-volume", &node_id])?)?;
    let requested = format_volume_percent(percent);
    run(&["set-volume", &node_id, &requested])?;

    let result = (|| {
        let after = parse_wpctl_volume(&run(&["get-volume", &node_id])?)?;
        if (after.0 * 100.0 - percent).abs() > 0.1 {
            return Err(io::Error::other(format!(
                "PipeWire read back {:.3}% after requesting {percent:.3}%",
                after.0 * 100.0
            )));
        }
        if after.1 != before.1 {
            return Err(io::Error::other(
                "PipeWire changed the AE-5 mute state while setting volume",
            ));
        }
        Ok((after.0 * 100.0, after.1))
    })();
    match result {
        Ok(applied) => Ok(applied),
        Err(error) => {
            let restore_volume = format_volume_percent(before.0 * 100.0);
            let volume_restore = run(&["set-volume", &node_id, &restore_volume]);
            let mute_restore = run(&["set-mute", &node_id, if before.1 { "1" } else { "0" }]);
            Err(match (volume_restore, mute_restore) {
                (Ok(_), Ok(_)) => match run(&["get-volume", &node_id])
                    .and_then(|output| parse_wpctl_volume(&output))
                {
                    Ok(restored)
                        if (restored.0 - before.0).abs() <= 0.001 && restored.1 == before.1 =>
                    {
                        error
                    }
                    Ok(restored) => io::Error::other(format!(
                        "{error}; rollback read back {:.3}% and mute={}, expected {:.3}% and mute={}",
                        restored.0 * 100.0,
                        restored.1,
                        before.0 * 100.0,
                        before.1
                    )),
                    Err(rollback) => io::Error::other(format!(
                        "{error}; rollback verification failed: {rollback}"
                    )),
                },
                (volume, mute) => io::Error::other(format!(
                    "{error}; rollback failed (volume: {}; mute: {})",
                    volume
                        .err()
                        .map_or_else(|| "restored".to_owned(), |error| error.to_string()),
                    mute.err()
                        .map_or_else(|| "restored".to_owned(), |error| error.to_string())
                )),
            })
        }
    }
}

fn set_software_mute_with<F>(
    node: &PipeWireNode,
    muted: bool,
    mut run: F,
) -> io::Result<(f64, bool)>
where
    F: FnMut(&[&str]) -> io::Result<String>,
{
    let node_id = node.id.to_string();
    let before = parse_wpctl_volume(&run(&["get-volume", &node_id])?)?;
    if before.1 == muted {
        return Ok((before.0 * 100.0, before.1));
    }

    run(&["set-mute", &node_id, if muted { "1" } else { "0" }])?;
    let result = (|| {
        let after = parse_wpctl_volume(&run(&["get-volume", &node_id])?)?;
        if (after.0 - before.0).abs() > 0.001 {
            return Err(io::Error::other(format!(
                "PipeWire changed AE-5 volume from {:.3}% to {:.3}% while setting mute",
                before.0 * 100.0,
                after.0 * 100.0
            )));
        }
        if after.1 != muted {
            return Err(io::Error::other(format!(
                "PipeWire read back mute={} after requesting mute={muted}",
                after.1
            )));
        }
        Ok((after.0 * 100.0, after.1))
    })();
    match result {
        Ok(applied) => Ok(applied),
        Err(error) => {
            let restore_volume = format_volume_percent(before.0 * 100.0);
            let volume_restore = run(&["set-volume", &node_id, &restore_volume]);
            let mute_restore = run(&["set-mute", &node_id, if before.1 { "1" } else { "0" }]);
            Err(match (volume_restore, mute_restore) {
                (Ok(_), Ok(_)) => match run(&["get-volume", &node_id])
                    .and_then(|output| parse_wpctl_volume(&output))
                {
                    Ok(restored)
                        if (restored.0 - before.0).abs() <= 0.001 && restored.1 == before.1 =>
                    {
                        error
                    }
                    Ok(restored) => io::Error::other(format!(
                        "{error}; rollback read back {:.3}% and mute={}, expected {:.3}% and mute={}",
                        restored.0 * 100.0,
                        restored.1,
                        before.0 * 100.0,
                        before.1
                    )),
                    Err(rollback) => io::Error::other(format!(
                        "{error}; rollback verification failed: {rollback}"
                    )),
                },
                (volume, mute) => io::Error::other(format!(
                    "{error}; rollback failed (volume: {}; mute: {})",
                    volume
                        .err()
                        .map_or_else(|| "restored".to_owned(), |error| error.to_string()),
                    mute.err()
                        .map_or_else(|| "restored".to_owned(), |error| error.to_string())
                )),
            })
        }
    }
}

fn validate_software_volume(percent: f64) -> io::Result<()> {
    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "PipeWire software volume must be between 0 and 100 percent",
        ));
    }
    Ok(())
}

fn format_volume_percent(percent: f64) -> String {
    let formatted = format!("{percent:.3}");
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    format!("{trimmed}%")
}

fn parse_wpctl_volume(output: &str) -> io::Result<(f64, bool)> {
    let line = output.trim();
    let scalar = line
        .strip_prefix("Volume:")
        .and_then(|value| value.split_ascii_whitespace().next())
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "wpctl returned an invalid software volume",
            )
        })?;
    Ok((
        scalar,
        line.split_ascii_whitespace().any(|part| {
            part.trim_matches(|character| character == '[' || character == ']') == "MUTED"
        }),
    ))
}

pub fn native_rates_config() -> io::Result<NativeRatesConfig> {
    native_rates_config_at(&native_rates_path()?)
}

pub fn set_native_rates_enabled(enabled: bool) -> io::Result<NativeRatesConfig> {
    let path = native_rates_path()?;
    let current = native_rates_config_at(&path)?;
    if current.enabled == enabled {
        return Ok(current);
    }

    if enabled {
        fs::create_dir_all(path.parent().expect("rate config has a parent"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(NATIVE_RATES_CONFIG.as_bytes())?;
        file.sync_all()?;
    } else {
        fs::remove_file(&path)?;
    }
    native_rates_config_at(&path)
}

fn native_rates_path() -> io::Result<PathBuf> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return Ok(path.join("pipewire/pipewire.conf.d/91-ae5-control-rates.conf"));
    }
    env::var_os("HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(".config/pipewire/pipewire.conf.d/91-ae5-control-rates.conf"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "neither XDG_CONFIG_HOME nor HOME is available",
            )
        })
}

fn native_rates_config_at(path: &Path) -> io::Result<NativeRatesConfig> {
    match fs::read_to_string(path) {
        Ok(contents) if contents == NATIVE_RATES_CONFIG => Ok(NativeRatesConfig {
            path: path.to_owned(),
            enabled: true,
        }),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "{} exists but is not managed by AE-5 Control",
                path.display()
            ),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(NativeRatesConfig {
            path: path.to_owned(),
            enabled: false,
        }),
        Err(error) => Err(error),
    }
}

fn ae5_node(card_index: i32, nodes: &str) -> io::Result<Option<PipeWireNode>> {
    let mut fallback = None;
    let status = run_wpctl(&["status", "-n"])?;
    for listing in parse_status_node_list(&status, nodes) {
        let details = run_wpctl(&["inspect", &listing.id.to_string()])?;
        if property(&details, "alsa.card").and_then(|value| value.parse().ok()) != Some(card_index)
        {
            continue;
        }
        let Some(node_name) = property(&details, "node.name") else {
            continue;
        };
        let node = PipeWireNode {
            id: listing.id,
            description: property(&details, "node.description")
                .unwrap_or_else(|| node_name.clone()),
            node_name,
            is_default: listing.is_default,
            volume_percent: listing.volume_percent,
            muted: listing.muted,
        };
        if property(&details, "alsa.device").as_deref() == Some("0") {
            return Ok(Some(node));
        }
        fallback.get_or_insert(node);
    }
    Ok(fallback)
}

#[cfg(test)]
fn node_from_details(listing: NodeListing, details: &str) -> Option<PipeWireNode> {
    let node_name = property(details, "node.name")?;
    Some(PipeWireNode {
        id: listing.id,
        description: property(details, "node.description").unwrap_or_else(|| node_name.clone()),
        node_name,
        is_default: listing.is_default,
        volume_percent: listing.volume_percent,
        muted: listing.muted,
    })
}

#[derive(Debug)]
struct PipeWireCardProfile {
    card_name: String,
    active_profile: String,
    profiles: BTreeSet<String>,
}

fn ae5_card_profile(card_index: i32) -> io::Result<PipeWireCardProfile> {
    parse_pactl_card_profile(&run_pactl(&["--format=json", "list", "cards"])?, card_index)
}

fn parse_pactl_card_profile(output: &str, card_index: i32) -> io::Result<PipeWireCardProfile> {
    let cards = serde_json::from_str::<serde_json::Value>(output)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let cards = cards.as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "pactl did not return a card array",
        )
    })?;
    let mut matches = cards.iter().filter(|card| {
        let value = &card["properties"]["alsa.card"];
        value.as_i64() == Some(i64::from(card_index))
            || value.as_str().and_then(|value| value.parse().ok()) == Some(card_index)
    });
    let card = matches.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("pactl has no ALSA card {card_index}"),
        )
    })?;
    if matches.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("pactl returned multiple ALSA card {card_index} entries"),
        ));
    }
    parse_card_profile(card)
}

fn parse_card_profile(card: &serde_json::Value) -> io::Result<PipeWireCardProfile> {
    if card["properties"]["device.profile-set"].as_str() != Some(AE5_PROFILE_SET) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("PipeWire is not using {AE5_PROFILE_SET}"),
        ));
    }
    if json_bool(&card["properties"]["api.alsa.soft-mixer"]) != Some(true) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PipeWire must enable api.alsa.soft-mixer for safe AE-5 routing",
        ));
    }
    if json_bool(&card["properties"]["api.alsa.ignore-dB"]) != Some(true) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PipeWire must enable api.alsa.ignore-dB for working AE-5 volume",
        ));
    }
    let profiles = card["profiles"].as_object().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the AE-5 PipeWire card has no profile map",
        )
    })?;
    Ok(PipeWireCardProfile {
        card_name: required_json_string(card, "name")?.to_owned(),
        active_profile: required_json_string(card, "active_profile")?.to_owned(),
        profiles: profiles
            .iter()
            .filter(|(_, profile)| profile["available"].as_bool() == Some(true))
            .map(|(name, _)| name.clone())
            .collect(),
    })
}

fn required_json_string<'a>(value: &'a serde_json::Value, field: &str) -> io::Result<&'a str> {
    value[field].as_str().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("the AE-5 PipeWire card has no {field}"),
        )
    })
}

fn json_bool(value: &serde_json::Value) -> Option<bool> {
    value
        .as_bool()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
}

fn output_profile_component(output_choice: &str, speaker_layout: &str) -> io::Result<&'static str> {
    match (output_choice, speaker_layout) {
        ("Headphone", _) | ("Speakers", "2.0") => Ok("output:analog-stereo"),
        ("Speakers", "2.1") => Ok("output:analog-surround-21"),
        ("Speakers", "4.0") => Ok("output:analog-surround-40"),
        ("Speakers", "4.1") => Ok("output:analog-surround-41"),
        ("Speakers", "5.1") => Ok("output:analog-surround-51"),
        ("Speakers", other) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported AE-5 speaker layout '{other}'"),
        )),
        (other, _) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unsupported AE-5 output choice '{other}'"),
        )),
    }
}

fn output_profile(
    output_choice: &str,
    speaker_layout: &str,
    preserve_input: bool,
) -> io::Result<String> {
    let output = output_profile_component(output_choice, speaker_layout)?;
    Ok(if preserve_input {
        format!("{output}+input:analog-stereo")
    } else {
        output.to_owned()
    })
}

fn set_card_profile(card: &PipeWireCardProfile, target: &str) -> io::Result<()> {
    if !card.profiles.contains(target) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("the AE-5 PipeWire profile '{target}' is unavailable"),
        ));
    }
    run_pactl(&["set-card-profile", &card.card_name, target])?;
    // WirePlumber can take several seconds to expose the active profile after
    // a session-policy restart.
    for _ in 0..200 {
        if ae5_card_profile_by_name(&card.card_name)?.active_profile == target {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(25));
    }

    let rollback = run_pactl(&["set-card-profile", &card.card_name, &card.active_profile])
        .and_then(|_| {
            for _ in 0..40 {
                if ae5_card_profile_by_name(&card.card_name)?.active_profile == card.active_profile
                {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "PipeWire did not restore the previous AE-5 profile",
            ))
        });
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!(
            "PipeWire did not activate '{target}'; {}",
            if rollback.is_ok() {
                "restored the previous profile"
            } else {
                "failed to restore the previous profile"
            }
        ),
    ))
}

fn ae5_card_profile_by_name(card_name: &str) -> io::Result<PipeWireCardProfile> {
    let cards = run_pactl(&["--format=json", "list", "cards"])?;
    let cards = serde_json::from_str::<serde_json::Value>(&cards)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let cards = cards.as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "pactl did not return a card array",
        )
    })?;
    let mut matches = cards
        .iter()
        .filter(|card| card["name"].as_str() == Some(card_name));
    let card = matches.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("pactl has no card named '{card_name}'"),
        )
    })?;
    if matches.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("pactl returned multiple cards named '{card_name}'"),
        ));
    }
    parse_card_profile(card)
}

fn set_ae5_default_node(
    card_index: i32,
    nodes: &str,
    description: &str,
) -> io::Result<PipeWireNode> {
    let node = ae5_node(card_index, nodes)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no {description} for ALSA card {card_index}"),
        )
    })?;
    if node.is_default {
        return Ok(node);
    }
    run_wpctl(&["set-default", &node.id.to_string()])?;
    ae5_node(card_index, nodes)?
        .filter(|node| node.is_default)
        .ok_or_else(|| io::Error::other(format!("PipeWire did not retain the AE-5 {description}")))
}

fn require_ae5_profile(node_id: u32) -> io::Result<()> {
    let node = run_wpctl(&["inspect", &node_id.to_string()])?;
    let device_id = property(&node, "device.id").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the AE-5 PipeWire node has no device.id",
        )
    })?;
    let device = run_wpctl(&["inspect", &device_id])?;
    if property(&device, "device.profile-set").as_deref() != Some(AE5_PROFILE_SET) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the AE-5 desktop route is not using sound-blaster-ae5.conf; restart WirePlumber after installing the package",
        ));
    }
    if property(&device, "api.alsa.soft-mixer").as_deref() != Some("true") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the AE-5 desktop route is missing api.alsa.soft-mixer; restart WirePlumber after installing the package",
        ));
    }
    if property(&device, "api.alsa.ignore-dB").as_deref() != Some("true") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "the AE-5 desktop route is missing api.alsa.ignore-dB; restart WirePlumber after installing the package",
        ));
    }
    Ok(())
}

fn ae5_control_route(
    control: &str,
    choice: &str,
) -> Option<(&'static str, &'static str, &'static str)> {
    match (control, choice) {
        ("Input Source", "Microphone") => {
            Some(("sources", "Input", "sound-blaster-ae5-input-microphone"))
        }
        ("Input Source", "Front Microphone") => Some((
            "sources",
            "Input",
            "sound-blaster-ae5-input-front-microphone",
        )),
        ("Input Source", "Line In") => {
            Some(("sources", "Input", "sound-blaster-ae5-input-line-in"))
        }
        ("Output Select", "Speakers") => {
            Some(("sinks", "Output", "analog-output-lineout;output-speaker"))
        }
        ("Output Select", "Headphone") => {
            Some(("sinks", "Output", "sound-blaster-ae5-output-headphones"))
        }
        _ => None,
    }
}

fn parse_route_state(output: &str, card_index: i32) -> io::Result<PipeWireRouteState> {
    let objects = serde_json::from_str::<serde_json::Value>(output)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let device = ae5_device(&objects, card_index)?;

    let profile_set = device["info"]["props"]["device.profile-set"]
        .as_str()
        .map(str::to_owned);
    let soft_mixer = json_bool(&device["info"]["props"]["api.alsa.soft-mixer"]);
    let ignore_db = json_bool(&device["info"]["props"]["api.alsa.ignore-dB"]);
    let active_profile = single_param_name(device, "Profile", None)?;
    let input_route = single_param_name(device, "Route", Some("Input"))?;
    let output_route = single_param_name(device, "Route", Some("Output"))?;
    Ok(PipeWireRouteState {
        profile_set,
        soft_mixer,
        ignore_db,
        persistent_playback: None,
        active_profile,
        input_route,
        output_route,
    })
}

fn parse_route_index(
    output: &str,
    card_index: i32,
    direction: &str,
    name: &str,
) -> io::Result<u32> {
    let objects = serde_json::from_str::<serde_json::Value>(output)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let device = ae5_device(&objects, card_index)?;
    let routes = device["info"]["params"]["EnumRoute"]
        .as_array()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("pw-dump has no available routes for ALSA card {card_index}"),
            )
        })?;
    let mut matches = routes.iter().filter(|route| {
        route["direction"].as_str() == Some(direction) && route["name"].as_str() == Some(name)
    });
    let route = matches.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no {direction} route named '{name}'"),
        )
    })?;
    if matches.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("PipeWire returned multiple {direction} routes named '{name}'"),
        ));
    }
    route["index"]
        .as_u64()
        .and_then(|index| u32::try_from(index).ok())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("PipeWire route '{name}' has no valid index"),
            )
        })
}

fn ae5_device(objects: &serde_json::Value, card_index: i32) -> io::Result<&serde_json::Value> {
    let devices = objects.as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "pw-dump did not return an object array",
        )
    })?;
    let mut matches = devices.iter().filter(|device| {
        let card = &device["info"]["props"]["api.alsa.card"];
        card.as_i64() == Some(i64::from(card_index))
            || card.as_str().and_then(|value| value.parse().ok()) == Some(card_index)
    });
    let device = matches.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("pw-dump has no ALSA card {card_index} device"),
        )
    })?;
    if matches.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("pw-dump returned multiple ALSA card {card_index} devices"),
        ));
    }
    Ok(device)
}

fn single_param_name(
    device: &serde_json::Value,
    parameter: &str,
    direction: Option<&str>,
) -> io::Result<Option<String>> {
    let Some(values) = device["info"]["params"][parameter].as_array() else {
        return Ok(None);
    };
    let mut names = values.iter().filter(|value| {
        direction.is_none_or(|expected| value["direction"].as_str() == Some(expected))
    });
    let Some(value) = names.next() else {
        return Ok(None);
    };
    if names.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("pw-dump returned multiple active {parameter} entries"),
        ));
    }
    value["name"]
        .as_str()
        .map(str::to_owned)
        .map(Some)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("pw-dump {parameter} entry has no name"),
            )
        })
}

fn require_direct_filter_support(node_id: u32) -> io::Result<()> {
    let output = run_pw_cli(&["enum-params", &node_id.to_string(), "PropInfo"])?;
    if output.contains(r#"String "audioconvert.filter-graph.N""#) {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "this PipeWire output does not support in-place audioconvert filter graphs; PipeWire 1.4 or newer is required",
        ))
    }
}

fn set_direct_filter(node_id: u32, graph: &str) -> io::Result<()> {
    let parameter = direct_filter_parameter(DIRECT_FILTER_PARAMETER, graph)?;
    run_pw_cli(&["set-param", &node_id.to_string(), "Props", &parameter]).map(|_| ())
}

fn set_effects_filter(node_id: u32, graph: &str) -> io::Result<()> {
    let parameter = direct_filter_parameter(EFFECTS_FILTER_PARAMETER, graph)?;
    run_pw_cli(&["set-param", &node_id.to_string(), "Props", &parameter]).map(|_| ())
}

fn direct_filter_parameter(property: &str, graph: &str) -> io::Result<String> {
    let graph = escape_spa_string(graph)?;
    Ok(format!("{{ params = [ \"{property}\" \"{graph}\" ] }}"))
}

fn escape_spa_string(value: &str) -> io::Result<String> {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "software equalizer graph contains an unsupported control character",
                ));
            }
            character => escaped.push(character),
        }
    }
    Ok(escaped)
}

fn set_software_eq_signature(node_id: u32, signature: Option<&str>) -> io::Result<()> {
    set_software_filter_signature(node_id, SOFTWARE_EQ_SIGNATURE_PROPERTY, signature)
}

fn set_software_effects_signature(node_id: u32, signature: Option<&str>) -> io::Result<()> {
    set_software_filter_signature(node_id, SOFTWARE_EFFECTS_SIGNATURE_PROPERTY, signature)
}

fn set_software_filter_signature(
    node_id: u32,
    property: &str,
    signature: Option<&str>,
) -> io::Result<()> {
    let id = node_id.to_string();
    let mut arguments = vec!["-n", PIPEWIRE_SETTINGS_METADATA_NAME];
    if signature.is_none() {
        arguments.push("-d");
    }
    arguments.extend([id.as_str(), property]);
    if let Some(signature) = signature {
        arguments.extend([signature, "Spa:String"]);
    }
    run_pw_metadata(&arguments).map(|_| ())
}

fn require_software_eq_signature(node_id: u32, expected: Option<&str>) -> io::Result<()> {
    let actual = software_eq_signature(node_id)?;
    if actual.as_deref() == expected {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "PipeWire did not retain the software equalizer marker (expected {}, read back {})",
        expected.unwrap_or("not loaded"),
        actual.as_deref().unwrap_or("not loaded")
    )))
}

fn require_software_effects_signature(node_id: u32, expected: Option<&str>) -> io::Result<()> {
    let actual = software_effects_signature(node_id)?;
    if actual.as_deref() == expected {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "PipeWire did not retain the software Effects marker (expected {}, read back {})",
        expected.unwrap_or("not loaded"),
        actual.as_deref().unwrap_or("not loaded")
    )))
}

fn software_eq_signature(node_id: u32) -> io::Result<Option<String>> {
    parse_metadata_value(
        &run_pw_metadata(&["-n", PIPEWIRE_SETTINGS_METADATA_NAME])?,
        node_id,
        SOFTWARE_EQ_SIGNATURE_PROPERTY,
    )
}

fn software_effects_signature(node_id: u32) -> io::Result<Option<String>> {
    parse_metadata_value(
        &run_pw_metadata(&["-n", PIPEWIRE_SETTINGS_METADATA_NAME])?,
        node_id,
        SOFTWARE_EFFECTS_SIGNATURE_PROPERTY,
    )
}

fn parse_runtime_sample_rate(output: &str) -> io::Result<RuntimeSampleRate> {
    match parse_metadata_value(output, 0, PIPEWIRE_FORCE_RATE_PROPERTY)?.as_deref() {
        None | Some("0") => Ok(RuntimeSampleRate::Auto),
        Some("48000") => Ok(RuntimeSampleRate::Hz48000),
        Some("96000") => Ok(RuntimeSampleRate::Hz96000),
        Some(rate) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("PipeWire uses unsupported forced sample rate '{rate}'"),
        )),
    }
}

fn parse_active_pcm_format(output: &str) -> io::Result<AudioFormat> {
    let mut sample_format = None;
    let mut sample_rate = None;
    let mut lines = output.lines();
    while let Some(line) = lines.next() {
        if line.contains("Format:Audio:format") {
            sample_format = lines.next().and_then(|value| {
                value
                    .trim()
                    .split_once("(Spa:Enum:AudioFormat:")
                    .and_then(|(_, value)| value.strip_suffix(')'))
                    .map(str::to_owned)
            });
        } else if line.contains("Format:Audio:rate") {
            sample_rate = lines
                .next()
                .and_then(|value| value.split_ascii_whitespace().nth(1))
                .and_then(|value| value.parse().ok());
        }
    }
    Ok(AudioFormat {
        sample_format: sample_format.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "PipeWire did not report the AE-5 PCM format",
            )
        })?,
        sample_rate: sample_rate.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "PipeWire did not report the AE-5 sample rate",
            )
        })?,
    })
}

fn parse_metadata_value(output: &str, id: u32, key: &str) -> io::Result<Option<String>> {
    let prefix = format!("update: id:{id} key:'{key}' value:'");
    let mut values = output.lines().filter_map(|line| {
        line.strip_prefix(&prefix)
            .and_then(|value| value.split_once("' type:'"))
            .map(|(value, _)| value.to_owned())
    });
    let value = values.next();
    if values.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("PipeWire metadata has duplicate '{key}' values for node {id}"),
        ));
    }
    Ok(value)
}

fn run_wpctl(arguments: &[&str]) -> io::Result<String> {
    let output = Command::new("wpctl")
        .args(arguments)
        .output()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "wpctl is unavailable; install WirePlumber",
                )
            } else {
                error
            }
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(io::Error::other(if detail.is_empty() {
            format!("wpctl {} failed", arguments.join(" "))
        } else {
            detail
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_pw_cli(arguments: &[&str]) -> io::Result<String> {
    let output = Command::new("pw-cli")
        .args(arguments)
        .output()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "pw-cli is unavailable; install PipeWire utilities",
                )
            } else {
                error
            }
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(io::Error::other(if detail.is_empty() {
            format!("pw-cli {} failed", arguments.join(" "))
        } else {
            detail
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_pw_metadata(arguments: &[&str]) -> io::Result<String> {
    let output = Command::new("pw-metadata")
        .args(arguments)
        .output()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "pw-metadata is unavailable; install PipeWire utilities",
                )
            } else {
                error
            }
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(io::Error::other(if detail.is_empty() {
            format!("pw-metadata {} failed", arguments.join(" "))
        } else {
            detail
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_pw_dump() -> io::Result<String> {
    let output = Command::new("pw-dump").output().map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            io::Error::new(
                io::ErrorKind::NotFound,
                "pw-dump is unavailable; install PipeWire utilities",
            )
        } else {
            error
        }
    })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(io::Error::other(if detail.is_empty() {
            "pw-dump failed".to_owned()
        } else {
            detail
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn run_pactl(arguments: &[&str]) -> io::Result<String> {
    let output = Command::new("pactl")
        .args(arguments)
        .output()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "pactl is unavailable; install PipeWire PulseAudio utilities",
                )
            } else {
                error
            }
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(io::Error::other(if detail.is_empty() {
            format!("pactl {} failed", arguments.join(" "))
        } else {
            detail
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn park_sink_inputs(node_name: &str) -> io::Result<Option<ParkedSinkInputs>> {
    let sink_index = pactl_sink_index(node_name)?;
    let input_ids = pactl_sink_inputs(sink_index)?;
    if input_ids.is_empty() {
        return Ok(None);
    }

    let sequence = NEXT_TRANSITION_SINK.fetch_add(1, Ordering::Relaxed);
    let sink_name = format!("ae5_control_transition_{}_{}", std::process::id(), sequence);
    let sink_argument = format!("sink_name={sink_name}");
    let module_output = run_pactl(&[
        "load-module",
        "module-null-sink",
        &sink_argument,
        "rate=96000",
        "channels=2",
        "sink_properties=device.description=AE5_Control_Transition",
    ])?;
    let module_id = module_output.trim().parse::<u32>().map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "pactl returned invalid transition module id {:?}: {error}",
                module_output.trim()
            ),
        )
    })?;
    let mut parked = ParkedSinkInputs {
        module_id,
        sink_name,
        original_sink_name: node_name.to_owned(),
        input_ids: Vec::with_capacity(input_ids.len()),
    };
    for input_id in input_ids {
        if let Err(error) = move_sink_input(input_id, &parked.sink_name) {
            return match parked.restore() {
                Ok(()) => Err(error),
                Err(restore_error) => Err(io::Error::other(format!(
                    "{error}; transition stream rollback also failed: {restore_error}"
                ))),
            };
        }
        parked.input_ids.push(input_id);
    }
    Ok(Some(parked))
}

fn move_sink_input(input_id: u32, sink_name: &str) -> io::Result<()> {
    run_pactl(&["move-sink-input", &input_id.to_string(), sink_name]).map(|_| ())
}

fn pactl_sink_index(node_name: &str) -> io::Result<u32> {
    let output = run_pactl(&["--format=json", "list", "sinks"])?;
    parse_pactl_sink_index(&output, node_name)
}

fn parse_pactl_sink_index(output: &str, node_name: &str) -> io::Result<u32> {
    let sinks = serde_json::from_str::<serde_json::Value>(output)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let sinks = sinks.as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "pactl did not return a sink array",
        )
    })?;
    let mut matches = sinks
        .iter()
        .filter(|sink| sink["name"].as_str() == Some(node_name));
    let sink = matches.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("pactl has no AE-5 sink named '{node_name}'"),
        )
    })?;
    if matches.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("pactl returned duplicate sinks named '{node_name}'"),
        ));
    }
    json_u32(sink, "index", "sink")
}

fn pactl_sink_inputs(sink_index: u32) -> io::Result<Vec<u32>> {
    let output = run_pactl(&["--format=json", "list", "sink-inputs"])?;
    parse_pactl_sink_inputs(&output, sink_index)
}

fn pactl_sink_input_ids() -> io::Result<BTreeSet<u32>> {
    let output = run_pactl(&["--format=json", "list", "sink-inputs"])?;
    parse_pactl_sink_input_ids(&output)
}

fn parse_pactl_sink_inputs(output: &str, sink_index: u32) -> io::Result<Vec<u32>> {
    let inputs = pactl_sink_input_values(output)?;
    inputs
        .iter()
        .filter(|input| input["sink"].as_u64() == Some(u64::from(sink_index)))
        .map(|input| json_u32(input, "index", "sink input"))
        .collect()
}

fn parse_pactl_sink_input_ids(output: &str) -> io::Result<BTreeSet<u32>> {
    pactl_sink_input_values(output)?
        .iter()
        .map(|input| json_u32(input, "index", "sink input"))
        .collect()
}

fn pactl_sink_input_values(output: &str) -> io::Result<Vec<serde_json::Value>> {
    serde_json::from_str::<serde_json::Value>(output)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
        .as_array()
        .cloned()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "pactl did not return a sink-input array",
            )
        })
}

fn json_u32(value: &serde_json::Value, field: &str, object: &str) -> io::Result<u32> {
    value[field]
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("pactl {object} has no valid {field}"),
            )
        })
}

fn combine_transition_results(first: io::Result<()>, second: io::Result<()>) -> io::Result<()> {
    match (first, second) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(first), Err(second)) => Err(io::Error::other(format!("{first}; {second}"))),
    }
}

fn pactl_sink_is_suspended(node_name: &str) -> io::Result<bool> {
    let output = run_pactl(&["--format=json", "list", "sinks"])?;
    parse_pactl_sink_suspended(&output, node_name)
}

fn parse_pactl_sink_suspended(output: &str, node_name: &str) -> io::Result<bool> {
    let sinks = serde_json::from_str::<serde_json::Value>(output)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    let sinks = sinks.as_array().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "pactl did not return a sink array",
        )
    })?;
    let mut matches = sinks
        .iter()
        .filter(|sink| sink["name"].as_str() == Some(node_name));
    let sink = matches.next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("pactl has no AE-5 sink named '{node_name}'"),
        )
    })?;
    if matches.next().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("pactl returned duplicate sinks named '{node_name}'"),
        ));
    }
    sink["state"]
        .as_str()
        .map(|state| state.eq_ignore_ascii_case("SUSPENDED"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("pactl sink '{node_name}' has no state"),
            )
        })
}

fn wait_for_alsa_playback_closed(card_index: i32) -> io::Result<()> {
    let path = PathBuf::from(format!("/proc/asound/card{card_index}/pcm0p/sub0/status"));
    // A newly created sink can briefly report suspended while ALSA is still
    // settling after a session-policy restart.
    for _ in 0..200 {
        match fs::read_to_string(&path) {
            Ok(status) if status.trim() == "closed" => return Ok(()),
            Ok(_) => thread::sleep(Duration::from_millis(25)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        }
    }
    // Something is still holding the card open. That is almost always a
    // program playing audio: switching the output route while a stream owns
    // the PCM is exactly the transition this project has seen produce faults,
    // so the guard is correct to refuse. Say which program, and what to do
    // about it, rather than reporting the internal condition.
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        match playback_holders(card_index) {
            holders if !holders.is_empty() => {
                format!("{holders} is still playing audio. Pause or stop it, then switch output.")
            }
            _ => format!(
                "Something is still playing audio on ALSA card {card_index}. \
                 Pause or stop playback, then switch output."
            ),
        },
    ))
}

/// Names of the processes currently holding the card's playback device.
///
/// Best-effort: used only to make a refusal legible, never to decide anything.
fn playback_holders(card_index: i32) -> String {
    let Ok(entries) = fs::read_dir("/proc") else {
        return String::new();
    };
    let device = format!("/dev/snd/pcmC{card_index}D0p");
    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let pid = entry.file_name();
        let Some(pid) = pid.to_str() else { continue };
        if !pid.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let Ok(fds) = fs::read_dir(format!("/proc/{pid}/fd")) else {
            continue;
        };
        let holds = fds.flatten().any(|fd| {
            fs::read_link(fd.path()).is_ok_and(|target| target.to_string_lossy() == device)
        });
        if holds && let Ok(comm) = fs::read_to_string(format!("/proc/{pid}/comm")) {
            let name = comm.trim().to_owned();
            if !name.is_empty() && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names.join(", ")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NodeListing {
    id: u32,
    is_default: bool,
    volume_percent: Option<u16>,
    muted: Option<bool>,
}

fn parse_status_node_list(output: &str, nodes: &str) -> Vec<NodeListing> {
    let heading = match nodes {
        "sinks" => "Sinks:",
        "sources" => "Sources:",
        _ => return Vec::new(),
    };
    let mut in_section = false;
    let mut listings = Vec::new();

    for line in output.lines() {
        let line = line
            .trim()
            .trim_start_matches(|character: char| "│├└─ ".contains(character))
            .trim_start();
        if line == heading {
            in_section = true;
            continue;
        }
        if in_section && line.ends_with(':') {
            break;
        }
        if !in_section {
            continue;
        }

        let (is_default, line) = line
            .strip_prefix('*')
            .map_or((false, line), |line| (true, line.trim_start()));
        let Some(id) = line
            .split_once('.')
            .and_then(|(id, _)| id.trim().parse().ok())
        else {
            continue;
        };
        listings.push(NodeListing {
            id,
            is_default,
            volume_percent: parse_node_volume_percent(line),
            muted: parse_node_muted(line),
        });
    }
    listings
}

fn parse_node_volume_percent(line: &str) -> Option<u16> {
    let value = line
        .split_once("[vol:")?
        .1
        .trim_start()
        .split(|character: char| character.is_whitespace() || character == ']')
        .next()?
        .parse::<f64>()
        .ok()?;
    let percent = (value * 100.0).round();
    (percent.is_finite() && (0.0..=f64::from(u16::MAX)).contains(&percent))
        .then_some(percent as u16)
}

fn parse_node_muted(line: &str) -> Option<bool> {
    let value = line.split_once("[vol:")?.1.split_once(']')?.0;
    Some(value.split_ascii_whitespace().any(|part| part == "MUTED"))
}

fn property(output: &str, name: &str) -> Option<String> {
    let prefix = format!("{name} = ");
    output.lines().find_map(|line| {
        line.trim()
            .strip_prefix("* ")
            .unwrap_or(line.trim())
            .strip_prefix(&prefix)
            .map(|value| value.trim_matches('"').to_owned())
    })
}

fn has_windows_audio_taper(details: &str) -> bool {
    property(details, "channelmix.volume-curve").as_deref() == Some("windows-audio-taper")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn replacing_eq_restores_the_previous_graph_after_failed_readback() {
        let signature = RefCell::new(Some("old".to_owned()));
        let writes = RefCell::new(Vec::new());

        let error = replace_software_filter_state_with(
            "software EQ",
            "new-graph",
            "new",
            PreviousFilterState {
                graph: Some("old-graph"),
                signature: Some("old"),
            },
            |graph| {
                writes.borrow_mut().push(format!("filter:{graph}"));
                Ok(())
            },
            |requested| {
                writes
                    .borrow_mut()
                    .push(format!("signature:{}", requested.unwrap_or("none")));
                *signature.borrow_mut() = requested.map(|value| {
                    if value == "new" {
                        "wrong".to_owned()
                    } else {
                        value.to_owned()
                    }
                });
                Ok(())
            },
            || Ok(signature.borrow().clone()),
        )
        .unwrap_err();

        assert_eq!(
            (error.kind(), signature.into_inner(), writes.into_inner(),),
            (
                io::ErrorKind::Other,
                Some("old".to_owned()),
                vec![
                    "filter:new-graph".to_owned(),
                    "signature:new".to_owned(),
                    "filter:old-graph".to_owned(),
                    "signature:old".to_owned(),
                ],
            )
        );
    }

    #[test]
    fn replacing_eq_restores_the_previous_graph_after_filter_write_failure() {
        let signature = RefCell::new(Some("old".to_owned()));
        let writes = RefCell::new(Vec::new());

        let error = replace_software_filter_state_with(
            "software EQ",
            "new-graph",
            "new",
            PreviousFilterState {
                graph: Some("old-graph"),
                signature: Some("old"),
            },
            |graph| {
                writes.borrow_mut().push(format!("filter:{graph}"));
                if graph == "new-graph" {
                    Err(io::Error::new(
                        io::ErrorKind::BrokenPipe,
                        "simulated filter write failure",
                    ))
                } else {
                    Ok(())
                }
            },
            |requested| {
                writes
                    .borrow_mut()
                    .push(format!("signature:{}", requested.unwrap_or("none")));
                *signature.borrow_mut() = requested.map(str::to_owned);
                Ok(())
            },
            || Ok(signature.borrow().clone()),
        )
        .unwrap_err();

        assert_eq!(
            (error.kind(), signature.into_inner(), writes.into_inner()),
            (
                io::ErrorKind::BrokenPipe,
                Some("old".to_owned()),
                vec![
                    "filter:new-graph".to_owned(),
                    "filter:old-graph".to_owned(),
                    "signature:old".to_owned(),
                ],
            )
        );
    }

    #[test]
    fn replacing_eq_refuses_an_unowned_runtime_graph_before_writing() {
        let writes = RefCell::new(Vec::new());

        let error = replace_software_filter_state_with(
            "software EQ",
            "new-graph",
            "new",
            PreviousFilterState {
                graph: None,
                signature: None,
            },
            |graph| {
                writes.borrow_mut().push(format!("filter:{graph}"));
                Ok(())
            },
            |signature| {
                writes.borrow_mut().push(format!("signature:{signature:?}"));
                Ok(())
            },
            || Ok(Some("foreign".to_owned())),
        )
        .unwrap_err();

        assert_eq!(
            (error.kind(), writes.into_inner()),
            (io::ErrorKind::Other, Vec::<String>::new())
        );
    }

    #[test]
    fn parses_wpctl_node_identity_and_default_marker() {
        let listing = "\
Audio
 ├─ Devices:
 │      47. alsa_card.pci-ae5                 [alsa]
 │
 ├─ Sinks:
 │      48. alsa_output.pci-hdmi              [vol: 1.00]
 │  *   49. alsa_output.pci-ae5.analog-stereo [vol: 0.40 MUTED]
 │
 ├─ Sources:
 │  *   50. alsa_input.pci-ae5.analog-stereo  [vol: 1.00]
 │
 └─ Streams:
";
        assert_eq!(
            parse_status_node_list(listing, "sinks"),
            vec![
                NodeListing {
                    id: 48,
                    is_default: false,
                    volume_percent: Some(100),
                    muted: Some(false),
                },
                NodeListing {
                    id: 49,
                    is_default: true,
                    volume_percent: Some(40),
                    muted: Some(true),
                },
            ]
        );
        assert_eq!(
            parse_status_node_list(listing, "sources"),
            vec![NodeListing {
                id: 50,
                is_default: true,
                volume_percent: Some(100),
                muted: Some(false),
            }]
        );
        assert_eq!(parse_node_volume_percent("87. ae5 [vol: 0.43]"), Some(43));
        assert_eq!(
            parse_node_volume_percent("87. ae5 [vol: 1.50 MUTED]"),
            Some(150)
        );
        assert_eq!(parse_node_volume_percent("87. ae5"), None);
        assert_eq!(parse_node_muted("87. ae5 [vol: 0.43]"), Some(false));
        assert_eq!(parse_node_muted("87. ae5 [vol: 1.50 MUTED]"), Some(true));
        assert_eq!(parse_node_muted("87. ae5"), None);

        let details = r#"
id 58, type PipeWire:Interface:Node
    alsa.card = "1"
    alsa.device = "0"
  * node.description = "Creative Sound BlasterX AE-5"
  * node.name = "alsa_output.pci-ae5.analog-stereo"
"#;
        assert_eq!(property(details, "alsa.card").as_deref(), Some("1"));
        assert_eq!(
            property(details, "node.name").as_deref(),
            Some("alsa_output.pci-ae5.analog-stereo")
        );
        assert!(!has_windows_audio_taper(details));
        assert!(has_windows_audio_taper(
            "channelmix.volume-curve = \"windows-audio-taper\""
        ));
        assert!(!has_windows_audio_taper(
            "channelmix.volume-curve = \"cubic\""
        ));
        assert_eq!(
            node_from_details(
                NodeListing {
                    id: 49,
                    is_default: true,
                    volume_percent: Some(40),
                    muted: Some(true),
                },
                details,
            ),
            Some(PipeWireNode {
                id: 49,
                node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
                description: "Creative Sound BlasterX AE-5".to_owned(),
                is_default: true,
                volume_percent: Some(40),
                muted: Some(true),
            })
        );
    }

    #[test]
    fn builds_a_single_in_place_filter_parameter_without_shell_parsing() {
        assert_eq!(
            direct_filter_parameter(DIRECT_FILTER_PARAMETER, "{ inputs = [ \"preL:In\" ] }\n")
                .unwrap(),
            "{ params = [ \"audioconvert.filter-graph.0\" \"{ inputs = [ \\\"preL:In\\\" ] }\\n\" ] }"
        );
        assert_eq!(
            direct_filter_parameter(DIRECT_FILTER_PARAMETER, "").unwrap(),
            "{ params = [ \"audioconvert.filter-graph.0\" \"\" ] }"
        );
        assert_eq!(
            direct_filter_parameter(DIRECT_FILTER_PARAMETER, "bad\u{7f}")
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            direct_filter_parameter(EFFECTS_FILTER_PARAMETER, "").unwrap(),
            "{ params = [ \"audioconvert.filter-graph.1\" \"\" ] }"
        );
    }

    #[test]
    fn reads_only_the_exact_direct_filter_metadata_marker() {
        let metadata = "\
Found \"settings\" metadata 32
update: id:0 key:'clock.rate' value:'48000' type:''
update: id:51 key:'ae5.control.eq.signature' value:'direct-v1|sink|-10.25|0,10' type:'Spa:String'
update: id:52 key:'ae5.control.eq.signature' value:'other' type:'Spa:String'
";
        assert_eq!(
            parse_metadata_value(metadata, 51, SOFTWARE_EQ_SIGNATURE_PROPERTY).unwrap(),
            Some("direct-v1|sink|-10.25|0,10".to_owned())
        );
        assert_eq!(
            parse_metadata_value(metadata, 50, SOFTWARE_EQ_SIGNATURE_PROPERTY).unwrap(),
            None
        );

        let duplicate = format!(
            "{metadata}update: id:51 key:'{SOFTWARE_EQ_SIGNATURE_PROPERTY}' value:'duplicate' type:'Spa:String'\n"
        );
        assert_eq!(
            parse_metadata_value(&duplicate, 51, SOFTWARE_EQ_SIGNATURE_PROPERTY)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn parses_the_exact_pactl_sink_state_without_ambiguity() {
        let sinks = r#"[
          {"index":68,"name":"alsa_output.pci-ae5.analog-stereo","state":"SUSPENDED"},
          {"index":73,"name":"alsa_output.usb-other.analog-stereo","state":"RUNNING"}
        ]"#;
        assert!(parse_pactl_sink_suspended(sinks, "alsa_output.pci-ae5.analog-stereo").unwrap());
        assert_eq!(
            parse_pactl_sink_index(sinks, "alsa_output.pci-ae5.analog-stereo").unwrap(),
            68
        );
        assert_eq!(
            parse_pactl_sink_suspended(sinks, "missing")
                .unwrap_err()
                .kind(),
            io::ErrorKind::NotFound
        );

        let duplicate = r#"[
          {"name":"duplicate","state":"SUSPENDED"},
          {"name":"duplicate","state":"RUNNING"}
        ]"#;
        assert_eq!(
            parse_pactl_sink_suspended(duplicate, "duplicate")
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn selects_only_streams_linked_to_the_exact_sink() {
        let inputs = r#"[
          {"index":118191,"sink":68,"corked":false},
          {"index":118192,"sink":73,"corked":false},
          {"index":118193,"sink":68,"corked":true}
        ]"#;

        assert_eq!(
            parse_pactl_sink_inputs(inputs, 68).unwrap(),
            vec![118191, 118193]
        );
        assert_eq!(
            parse_pactl_sink_input_ids(inputs).unwrap(),
            BTreeSet::from([118191, 118192, 118193])
        );
        assert_eq!(
            parse_pactl_sink_inputs(r#"[{"index":"bad","sink":68}]"#, 68)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn maps_only_the_packaged_ae5_desktop_routes() {
        assert_eq!(
            [
                ("Input Source", "Microphone"),
                ("Input Source", "Front Microphone"),
                ("Input Source", "Line In"),
                ("Output Select", "Speakers"),
                ("Output Select", "Headphone"),
            ]
            .map(|(control, choice)| ae5_control_route(control, choice)),
            [
                Some(("sources", "Input", "sound-blaster-ae5-input-microphone")),
                Some((
                    "sources",
                    "Input",
                    "sound-blaster-ae5-input-front-microphone"
                )),
                Some(("sources", "Input", "sound-blaster-ae5-input-line-in")),
                Some(("sinks", "Output", "analog-output-lineout;output-speaker")),
                Some(("sinks", "Output", "sound-blaster-ae5-output-headphones")),
            ]
        );
        assert_eq!(ae5_control_route("Output Select", "Unknown"), None);
        assert_eq!(
            ae5_control_route("AE-5: Sound Filter", "Fast Roll Off"),
            None
        );
    }

    #[test]
    fn resolves_route_index_by_exact_name_instead_of_path_order() {
        let output = r#"[
          {
            "info": {
              "props": {"api.alsa.card": 0},
              "params": {
                "EnumRoute": [
                  {"index": 6, "direction": "Output", "name": "iec958-stereo-output"},
                  {"index": 5, "direction": "Output", "name": "sound-blaster-ae5-output-headphones"},
                  {"index": 0, "direction": "Input", "name": "sound-blaster-ae5-input-microphone"}
                ]
              }
            }
          }
        ]"#;

        assert_eq!(
            parse_route_index(output, 0, "Output", "sound-blaster-ae5-output-headphones").unwrap(),
            5
        );
    }

    #[test]
    fn rejects_duplicate_exact_route_names() {
        let output = r#"[
          {
            "info": {
              "props": {"api.alsa.card": 0},
              "params": {
                "EnumRoute": [
                  {"index": 5, "direction": "Output", "name": "sound-blaster-ae5-output-headphones"},
                  {"index": 9, "direction": "Output", "name": "sound-blaster-ae5-output-headphones"}
                ]
              }
            }
          }
        ]"#;

        assert_eq!(
            parse_route_index(output, 0, "Output", "sound-blaster-ae5-output-headphones")
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn output_route_transition_mutes_before_route_and_restores_desktop_state() {
        let node = PipeWireNode {
            id: 62,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5".to_owned(),
            is_default: false,
            volume_percent: Some(5),
            muted: Some(false),
        };
        let mut commands = Vec::new();

        set_output_route_with(&node, 5, |arguments| {
            commands.push(
                arguments
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            );
            Ok(String::new())
        })
        .unwrap();

        assert_eq!(
            commands,
            [
                ["set-mute", "62", "1"],
                ["set-route", "62", "5"],
                ["set-volume", "62", "5%"],
                ["set-mute", "62", "0"],
            ]
            .map(|command| command.map(str::to_owned).to_vec())
        );
    }

    #[test]
    fn software_volume_updates_the_existing_sink_and_preserves_mute() {
        let node = PipeWireNode {
            id: 62,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5".to_owned(),
            is_default: true,
            volume_percent: Some(5),
            muted: Some(true),
        };
        let mut commands = Vec::new();
        let mut reads = 0;

        let applied = set_software_volume_with(&node, 34.567, |arguments| {
            commands.push(
                arguments
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            );
            Ok(if arguments.first() == Some(&"get-volume") {
                reads += 1;
                if reads == 1 {
                    "Volume: 0.050 [MUTED]\n".to_owned()
                } else {
                    "Volume: 0.346 [MUTED]\n".to_owned()
                }
            } else {
                String::new()
            })
        })
        .unwrap();

        assert!((applied.0 - 34.6).abs() < 1e-9);
        assert!(applied.1);
        assert_eq!(
            commands,
            vec![
                vec!["get-volume".to_owned(), "62".to_owned()],
                vec![
                    "set-volume".to_owned(),
                    "62".to_owned(),
                    "34.567%".to_owned()
                ],
                vec!["get-volume".to_owned(), "62".to_owned()],
            ]
        );
    }

    #[test]
    fn software_volume_rejects_a_mute_state_change() {
        let node = PipeWireNode {
            id: 62,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5".to_owned(),
            is_default: true,
            volume_percent: Some(5),
            muted: Some(true),
        };
        let mut reads = 0;
        let mut commands = Vec::new();

        let error = set_software_volume_with(&node, 20.0, |arguments| {
            commands.push(
                arguments
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            );
            Ok(if arguments.first() == Some(&"get-volume") {
                reads += 1;
                if reads == 1 || reads == 3 {
                    "Volume: 0.050 [MUTED]\n".to_owned()
                } else {
                    "Volume: 0.200\n".to_owned()
                }
            } else {
                String::new()
            })
        })
        .unwrap_err();

        assert!(error.to_string().contains("mute"));
        assert_eq!(
            commands,
            vec![
                vec!["get-volume".to_owned(), "62".to_owned()],
                vec!["set-volume".to_owned(), "62".to_owned(), "20%".to_owned()],
                vec!["get-volume".to_owned(), "62".to_owned()],
                vec!["set-volume".to_owned(), "62".to_owned(), "5%".to_owned()],
                vec!["set-mute".to_owned(), "62".to_owned(), "1".to_owned()],
                vec!["get-volume".to_owned(), "62".to_owned()],
            ]
        );
    }

    #[test]
    fn output_route_transition_refuses_unknown_desktop_state() {
        let node = PipeWireNode {
            id: 62,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5".to_owned(),
            is_default: false,
            volume_percent: None,
            muted: Some(true),
        };
        let mut command_ran = false;

        let error = set_output_route_with(&node, 5, |_| {
            command_ran = true;
            Ok(String::new())
        })
        .unwrap_err();

        assert_eq!(
            (error.kind(), command_ran),
            (io::ErrorKind::InvalidData, false)
        );
    }

    #[test]
    fn output_route_transition_leaves_sink_muted_when_restore_fails() {
        let node = PipeWireNode {
            id: 62,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5".to_owned(),
            is_default: false,
            volume_percent: Some(5),
            muted: Some(false),
        };
        let mut commands = Vec::new();

        let error = set_output_route_with(&node, 5, |arguments| {
            commands.push(arguments.join(" "));
            if arguments.first() == Some(&"set-volume") {
                Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    "volume restore failed",
                ))
            } else {
                Ok(String::new())
            }
        })
        .unwrap_err();

        assert_eq!(
            (error.kind(), commands),
            (
                io::ErrorKind::BrokenPipe,
                vec![
                    "set-mute 62 1",
                    "set-route 62 5",
                    "set-volume 62 5%",
                    "set-mute 62 1",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect()
            )
        );
    }

    #[test]
    fn maps_alsa_layouts_to_available_pipewire_profiles() {
        assert_eq!(
            [
                ("Headphone", "5.1"),
                ("Speakers", "2.0"),
                ("Speakers", "2.1"),
                ("Speakers", "4.0"),
                ("Speakers", "4.1"),
                ("Speakers", "5.1"),
            ]
            .map(|(output, layout)| output_profile(output, layout, true).unwrap()),
            [
                "output:analog-stereo+input:analog-stereo",
                "output:analog-stereo+input:analog-stereo",
                "output:analog-surround-21+input:analog-stereo",
                "output:analog-surround-40+input:analog-stereo",
                "output:analog-surround-41+input:analog-stereo",
                "output:analog-surround-51+input:analog-stereo",
            ]
        );
        assert_eq!(
            output_profile("Speakers", "5.1", false).unwrap(),
            "output:analog-surround-51"
        );
        assert_eq!(
            output_profile("Speakers", "7.1", true).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn parses_the_exact_ae5_pactl_card_profile() {
        let cards = r#"[
          {
            "name": "alsa_card.pci-ae5",
            "properties": {
              "alsa.card": "0",
              "device.profile-set": "sound-blaster-ae5.conf",
              "api.alsa.soft-mixer": "true",
              "api.alsa.ignore-dB": "true"
            },
            "active_profile": "output:analog-stereo+input:analog-stereo",
            "profiles": {
              "output:analog-stereo+input:analog-stereo": {"available": true},
              "output:analog-surround-51+input:analog-stereo": {"available": true},
              "off": {"available": false}
            }
          }
        ]"#;
        let card = parse_pactl_card_profile(cards, 0).unwrap();
        assert_eq!(card.card_name, "alsa_card.pci-ae5");
        assert_eq!(
            card.active_profile,
            "output:analog-stereo+input:analog-stereo"
        );
        assert_eq!(
            card.profiles,
            BTreeSet::from([
                "output:analog-stereo+input:analog-stereo".to_owned(),
                "output:analog-surround-51+input:analog-stereo".to_owned(),
            ])
        );
        assert_eq!(
            parse_pactl_card_profile(cards, 1).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );

        let wrong_profile_set = cards.replace("sound-blaster-ae5.conf", "default.conf");
        assert_eq!(
            parse_pactl_card_profile(&wrong_profile_set, 0)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        let unsafe_hardware_mixer = cards.replace(
            r#""api.alsa.soft-mixer": "true""#,
            r#""api.alsa.soft-mixer": false"#,
        );
        assert_eq!(
            parse_pactl_card_profile(&unsafe_hardware_mixer, 0)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        let invalid_db_metadata = cards.replace(
            r#""api.alsa.ignore-dB": "true""#,
            r#""api.alsa.ignore-dB": false"#,
        );
        assert_eq!(
            parse_pactl_card_profile(&invalid_db_metadata, 0)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn parses_and_validates_the_active_ae5_route() {
        let output = r#"[
          {
            "id": 12,
            "info": {
              "props": {"api.alsa.card": 2},
              "params": {}
            }
          },
          {
            "id": 55,
            "info": {
              "props": {
                "api.alsa.card": 0,
                "device.profile-set": "sound-blaster-ae5.conf",
                "api.alsa.soft-mixer": true,
                "api.alsa.ignore-dB": true
              },
              "params": {
                "Profile": [
                  {"name": "output:analog-stereo+input:analog-stereo"}
                ],
                "Route": [
                  {
                    "direction": "Input",
                    "name": "sound-blaster-ae5-input-microphone"
                  },
                  {
                    "direction": "Output",
                    "name": "sound-blaster-ae5-output-headphones"
                  }
                ]
              }
            }
          }
        ]"#;
        let state = parse_route_state(output, 0).unwrap();
        assert_eq!(
            state,
            PipeWireRouteState {
                profile_set: Some(AE5_PROFILE_SET.to_owned()),
                soft_mixer: Some(true),
                ignore_db: Some(true),
                persistent_playback: None,
                active_profile: Some("output:analog-stereo+input:analog-stereo".to_owned()),
                input_route: Some("sound-blaster-ae5-input-microphone".to_owned()),
                output_route: Some("sound-blaster-ae5-output-headphones".to_owned()),
            }
        );
        assert_eq!(state.output_issue("Headphone", "2.0"), None);
        assert_eq!(state.input_issue("Microphone"), None);
        assert_eq!(
            state.output_issue("Speakers", "2.0").as_deref(),
            Some(
                "ALSA selects Speakers, but PipeWire uses sound-blaster-ae5-output-headphones; reapply the output choice"
            )
        );
        assert_eq!(
            state.input_issue("Line In").as_deref(),
            Some(
                "ALSA selects Line In, but PipeWire uses sound-blaster-ae5-input-microphone; reapply the input choice"
            )
        );
        assert_eq!(
            parse_route_state(output, 1).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }

    #[test]
    fn rejects_ambiguous_route_state_and_wrong_profile() {
        let duplicate = r#"[
          {
            "info": {
              "props": {"api.alsa.card": "0"},
              "params": {
                "Route": [
                  {"direction": "Output", "name": "first"},
                  {"direction": "Output", "name": "second"}
                ]
              }
            }
          }
        ]"#;
        assert_eq!(
            parse_route_state(duplicate, 0).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
        let mut state = PipeWireRouteState {
            profile_set: Some("default.conf".to_owned()),
            soft_mixer: Some(true),
            ignore_db: Some(true),
            persistent_playback: None,
            active_profile: None,
            input_route: None,
            output_route: None,
        };
        assert!(
            state
                .output_issue("Headphone", "2.0")
                .unwrap()
                .contains(AE5_PROFILE_SET)
        );
        state.profile_set = Some(AE5_PROFILE_SET.to_owned());
        state.soft_mixer = Some(false);
        assert!(
            state
                .output_issue("Headphone", "2.0")
                .unwrap()
                .contains("hardware volume control is unsafe")
        );
        state.soft_mixer = Some(true);
        state.ignore_db = Some(false);
        assert!(
            state
                .output_issue("Headphone", "2.0")
                .unwrap()
                .contains("invalid dB metadata")
        );
        state.ignore_db = Some(true);
        state.active_profile = Some("output:analog-stereo+input:analog-stereo".to_owned());
        state.persistent_playback = Some(false);
        assert!(
            state
                .output_issue("Headphone", "2.0")
                .unwrap()
                .contains("session.suspend-timeout-seconds=0")
        );
        state.persistent_playback = Some(true);
        state.active_profile = Some("output:iec958-stereo".to_owned());
        assert!(
            state
                .output_issue("Headphone", "2.0")
                .unwrap()
                .contains("expected output:analog-stereo for output")
        );
        assert!(
            state
                .input_issue("Microphone")
                .unwrap()
                .contains("expected input:analog-stereo for input")
        );
    }

    #[test]
    fn validates_surround_output_by_the_exact_profile() {
        let mut state = PipeWireRouteState {
            profile_set: Some(AE5_PROFILE_SET.to_owned()),
            soft_mixer: Some(true),
            ignore_db: Some(true),
            persistent_playback: Some(true),
            active_profile: Some("output:analog-surround-51+input:analog-stereo".to_owned()),
            input_route: Some("sound-blaster-ae5-input-microphone".to_owned()),
            output_route: None,
        };
        assert_eq!(state.output_issue("Speakers", "5.1"), None);

        state.active_profile = Some("output:analog-stereo+input:analog-stereo".to_owned());
        assert!(
            state
                .output_issue("Speakers", "5.1")
                .unwrap()
                .contains("expected output:analog-surround-51 for output")
        );
        assert!(
            state
                .output_issue("Speakers", "7.1")
                .unwrap()
                .contains("unsupported AE-5 speaker layout")
        );
    }

    #[test]
    fn native_rate_config_is_idempotent_and_refuses_foreign_content() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = env::temp_dir().join(format!(
            "ae5-control-rate-test-{}-{unique}",
            std::process::id()
        ));
        let path = directory.join("91-ae5-control-rates.conf");

        assert!(!native_rates_config_at(&path).unwrap().enabled);
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, NATIVE_RATES_CONFIG).unwrap();
        assert!(native_rates_config_at(&path).unwrap().enabled);
        fs::write(&path, "user configuration\n").unwrap();
        assert_eq!(
            native_rates_config_at(&path).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );

        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn parses_the_runtime_sample_rate_from_pipewire_metadata() {
        let metadata = "\
Found \"settings\" metadata 32
update: id:0 key:'clock.rate' value:'48000' type:''
update: id:0 key:'clock.force-rate' value:'96000' type:''
";

        assert_eq!(
            parse_runtime_sample_rate(metadata).unwrap(),
            RuntimeSampleRate::Hz96000
        );
    }

    #[test]
    fn runtime_sample_rate_policy_names_round_trip() {
        for rate in [
            RuntimeSampleRate::Auto,
            RuntimeSampleRate::Hz48000,
            RuntimeSampleRate::Hz96000,
        ] {
            assert_eq!(
                RuntimeSampleRate::from_policy_name(rate.policy_name()),
                Some(rate)
            );
        }
        assert_eq!(RuntimeSampleRate::from_policy_name("192 kHz"), None);
    }

    #[test]
    fn parses_the_active_s16_pipewire_format() {
        let format = "\
    Prop: key Spa:Pod:Object:Param:Format:Audio:format (65537), flags 00000000
      Id 259      (Spa:Enum:AudioFormat:S16LE)
    Prop: key Spa:Pod:Object:Param:Format:Audio:rate (65539), flags 00000000
      Int 96000
";

        assert_eq!(
            parse_active_pcm_format(format).unwrap(),
            AudioFormat {
                sample_format: "S16LE".to_owned(),
                sample_rate: 96_000,
            }
        );
    }

    #[test]
    fn idle_pipewire_format_is_reported_as_absent() {
        let missing = parse_active_pcm_format("").unwrap_err();
        assert_eq!(optional_active_pcm_format(Err(missing)).unwrap(), None);
    }

    #[test]
    fn unexpected_pipewire_format_errors_are_preserved() {
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        let actual = optional_active_pcm_format(Err(error)).unwrap_err();
        assert_eq!(actual.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn software_mute_updates_the_existing_sink_and_preserves_volume() {
        let node = PipeWireNode {
            id: 62,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5".to_owned(),
            is_default: true,
            volume_percent: Some(20),
            muted: Some(false),
        };
        let mut commands = Vec::new();
        let mut reads = 0;

        let applied = set_software_mute_with(&node, true, |arguments| {
            commands.push(
                arguments
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            );
            Ok(if arguments.first() == Some(&"get-volume") {
                reads += 1;
                if reads == 1 {
                    "Volume: 0.200\n".to_owned()
                } else {
                    "Volume: 0.200 [MUTED]\n".to_owned()
                }
            } else {
                String::new()
            })
        })
        .unwrap();

        assert_eq!(applied, (20.0, true));
        assert_eq!(
            commands,
            vec![
                vec!["get-volume".to_owned(), "62".to_owned()],
                vec!["set-mute".to_owned(), "62".to_owned(), "1".to_owned()],
                vec!["get-volume".to_owned(), "62".to_owned()],
            ]
        );
    }

    #[test]
    fn software_mute_rolls_back_an_unexpected_volume_change() {
        let node = PipeWireNode {
            id: 62,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5".to_owned(),
            is_default: true,
            volume_percent: Some(20),
            muted: Some(false),
        };
        let mut commands = Vec::new();
        let mut reads = 0;

        let error = set_software_mute_with(&node, true, |arguments| {
            commands.push(
                arguments
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            );
            Ok(if arguments.first() == Some(&"get-volume") {
                reads += 1;
                match reads {
                    1 => "Volume: 0.200\n".to_owned(),
                    2 => "Volume: 0.150 [MUTED]\n".to_owned(),
                    _ => "Volume: 0.200\n".to_owned(),
                }
            } else {
                String::new()
            })
        })
        .unwrap_err();

        assert!(error.to_string().contains("volume"));
        assert_eq!(
            commands,
            vec![
                vec!["get-volume".to_owned(), "62".to_owned()],
                vec!["set-mute".to_owned(), "62".to_owned(), "1".to_owned()],
                vec!["get-volume".to_owned(), "62".to_owned()],
                vec!["set-volume".to_owned(), "62".to_owned(), "20%".to_owned()],
                vec!["set-mute".to_owned(), "62".to_owned(), "0".to_owned()],
                vec!["get-volume".to_owned(), "62".to_owned()],
            ]
        );
    }

    #[test]
    fn runtime_rate_transition_mutes_before_the_change_and_restores_mute_state() {
        let node = PipeWireNode {
            id: 62,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5".to_owned(),
            is_default: true,
            volume_percent: Some(20),
            muted: Some(false),
        };
        let mut wpctl_commands = Vec::new();
        let mut metadata_commands = Vec::new();
        let mut metadata_reads = 0;
        let mut suspension_changes = Vec::new();
        let mut verified = None;

        let applied = set_runtime_sample_rate_with(
            &node,
            RuntimeSampleRate::Hz96000,
            |arguments| {
                wpctl_commands.push(arguments.join(" "));
                Ok(String::new())
            },
            |arguments| {
                metadata_commands.push(arguments.join(" "));
                if arguments.len() == 2 {
                    metadata_reads += 1;
                    Ok(format!(
                        "update: id:0 key:'clock.force-rate' value:'{}' type:''\n",
                        if metadata_reads == 1 { "0" } else { "96000" }
                    ))
                } else {
                    Ok(String::new())
                }
            },
            |suspended| {
                suspension_changes.push(suspended);
                Ok(())
            },
            |rate| {
                verified = Some(rate);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!(
            (
                applied,
                wpctl_commands,
                metadata_commands,
                suspension_changes,
                verified,
            ),
            (
                RuntimeSampleRate::Hz96000,
                vec!["set-mute 62 1".to_owned(), "set-mute 62 0".to_owned()],
                vec![
                    "-n settings".to_owned(),
                    "-n settings 0 clock.force-rate 96000".to_owned(),
                    "-n settings".to_owned(),
                ],
                vec![true, false],
                Some(RuntimeSampleRate::Hz96000),
            )
        );
    }

    #[test]
    fn runtime_rate_failure_reopens_at_the_previous_rate_and_stays_muted() {
        let node = PipeWireNode {
            id: 62,
            node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
            description: "AE-5".to_owned(),
            is_default: true,
            volume_percent: Some(20),
            muted: Some(false),
        };
        let mut wpctl_commands = Vec::new();
        let mut metadata_commands = Vec::new();
        let mut metadata_reads = 0;
        let mut suspension_changes = Vec::new();
        let mut verified = Vec::new();

        let error = set_runtime_sample_rate_with(
            &node,
            RuntimeSampleRate::Hz48000,
            |arguments| {
                wpctl_commands.push(arguments.join(" "));
                Ok(String::new())
            },
            |arguments| {
                metadata_commands.push(arguments.join(" "));
                if arguments.len() == 2 {
                    metadata_reads += 1;
                    Ok(format!(
                        "update: id:0 key:'clock.force-rate' value:'{}' type:''\n",
                        match metadata_reads {
                            1 | 3 => "96000",
                            _ => "48000",
                        }
                    ))
                } else {
                    Ok(String::new())
                }
            },
            |suspended| {
                suspension_changes.push(suspended);
                Ok(())
            },
            |rate| {
                verified.push(rate);
                if rate == RuntimeSampleRate::Hz48000 {
                    Err(io::Error::other("AE-5 stayed at 96000 Hz"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("AE-5 stayed at 96000 Hz"));
        assert!(error.to_string().contains("rate rollback verified"));
        assert_eq!(
            wpctl_commands,
            vec!["set-mute 62 1".to_owned(), "set-mute 62 1".to_owned()]
        );
        assert_eq!(
            metadata_commands,
            vec![
                "-n settings".to_owned(),
                "-n settings 0 clock.force-rate 48000".to_owned(),
                "-n settings".to_owned(),
                "-n settings 0 clock.force-rate 96000".to_owned(),
                "-n settings".to_owned(),
            ]
        );
        assert_eq!(suspension_changes, vec![true, false, true, false]);
        assert_eq!(
            verified,
            vec![RuntimeSampleRate::Hz48000, RuntimeSampleRate::Hz96000]
        );
    }

    #[test]
    fn runtime_rate_verification_primes_a_closed_sink_with_silent_s16() {
        let mut attempts = 0;
        let mut primed_at = None;

        verify_runtime_sample_rate_with(
            RuntimeSampleRate::Hz48000,
            || {
                attempts += 1;
                if attempts == 1 {
                    Err(io::Error::new(
                        io::ErrorKind::NotFound,
                        "the sink is closed",
                    ))
                } else {
                    Ok(AudioFormat {
                        sample_format: "S16LE".to_owned(),
                        sample_rate: 48_000,
                    })
                }
            },
            |rate| {
                primed_at = Some(rate);
                Ok(())
            },
        )
        .unwrap();

        assert_eq!((attempts, primed_at), (2, Some(48_000)));
    }
}
