use std::collections::BTreeSet;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
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
const SOFTWARE_EQ_METADATA_NAME: &str = "settings";
const DIRECT_FILTER_PARAMETER: &str = "audioconvert.filter-graph.0";

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
}

impl SuspendedAe5Output {
    pub(crate) fn ensure_current_suspended(&mut self) -> io::Result<()> {
        let Some(node) = ae5_output(self.card_index)? else {
            return Ok(());
        };
        if !pactl_sink_is_suspended(&node.node_name)? {
            run_pactl(&["suspend-sink", &node.node_name, "1"])?;
            self.resume_on_drop = true;
        }
        wait_for_alsa_playback_closed(self.card_index)
    }

    pub(crate) fn resume(mut self) -> io::Result<()> {
        if !self.resume_on_drop {
            return Ok(());
        }
        if let Some(node) = ae5_output(self.card_index)? {
            run_pactl(&["suspend-sink", &node.node_name, "0"])?;
        }
        self.resume_on_drop = false;
        Ok(())
    }
}

impl Drop for SuspendedAe5Output {
    fn drop(&mut self) {
        if self.resume_on_drop {
            if let Ok(Some(node)) = ae5_output(self.card_index) {
                let _ = run_pactl(&["suspend-sink", &node.node_name, "0"]);
            }
            self.resume_on_drop = false;
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

pub fn ae5_output(card_index: i32) -> io::Result<Option<PipeWireNode>> {
    ae5_node(card_index, "sinks")
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
        });
    };
    if pactl_sink_is_suspended(&node.node_name)? {
        wait_for_alsa_playback_closed(card_index)?;
        return Ok(SuspendedAe5Output {
            card_index,
            resume_on_drop: false,
        });
    }

    run_pactl(&["suspend-sink", &node.node_name, "1"])?;
    let suspended = SuspendedAe5Output {
        card_index,
        resume_on_drop: true,
    };
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
    let parameter = direct_filter_parameter(graph)?;
    run_pw_cli(&["set-param", &node_id.to_string(), "Props", &parameter]).map(|_| ())
}

fn direct_filter_parameter(graph: &str) -> io::Result<String> {
    let graph = escape_spa_string(graph)?;
    Ok(format!(
        "{{ params = [ \"{DIRECT_FILTER_PARAMETER}\" \"{graph}\" ] }}"
    ))
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
    let id = node_id.to_string();
    let mut arguments = vec!["-n", SOFTWARE_EQ_METADATA_NAME];
    if signature.is_none() {
        arguments.push("-d");
    }
    arguments.extend([id.as_str(), SOFTWARE_EQ_SIGNATURE_PROPERTY]);
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

fn software_eq_signature(node_id: u32) -> io::Result<Option<String>> {
    parse_metadata_value(
        &run_pw_metadata(&["-n", SOFTWARE_EQ_METADATA_NAME])?,
        node_id,
        SOFTWARE_EQ_SIGNATURE_PROPERTY,
    )
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
    use std::time::{SystemTime, UNIX_EPOCH};

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
            direct_filter_parameter("{ inputs = [ \"preL:In\" ] }\n").unwrap(),
            "{ params = [ \"audioconvert.filter-graph.0\" \"{ inputs = [ \\\"preL:In\\\" ] }\\n\" ] }"
        );
        assert_eq!(
            direct_filter_parameter("").unwrap(),
            "{ params = [ \"audioconvert.filter-graph.0\" \"\" ] }"
        );
        assert_eq!(
            direct_filter_parameter("bad\u{7f}").unwrap_err().kind(),
            io::ErrorKind::InvalidInput
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
          {"name":"alsa_output.pci-ae5.analog-stereo","state":"SUSPENDED"},
          {"name":"alsa_output.usb-other.analog-stereo","state":"RUNNING"}
        ]"#;
        assert!(parse_pactl_sink_suspended(sinks, "alsa_output.pci-ae5.analog-stereo").unwrap());
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
}
