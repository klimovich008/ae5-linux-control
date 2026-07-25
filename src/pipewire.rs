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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipeWireNode {
    pub id: u32,
    pub node_name: String,
    pub description: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipeWireRouteState {
    pub profile_set: Option<String>,
    pub soft_mixer: Option<bool>,
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
        if output_choice == "Speakers" && speaker_layout != "2.0" {
            return None;
        }
        let expected = match output_choice {
            "Speakers" => "analog-output-lineout;output-speaker",
            "Headphone" => "sound-blaster-ae5-output-headphones;output-headphones",
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

pub fn set_ae5_default_output(card_index: i32) -> io::Result<PipeWireNode> {
    set_ae5_default_node(card_index, "sinks", "playback output")
}

pub fn set_ae5_default_input(card_index: i32) -> io::Result<PipeWireNode> {
    set_ae5_default_node(card_index, "sources", "recording input")
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
    let node = ae5_output(card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no AE-5 playback output for ALSA card {card_index}"),
        )
    })?;
    let details = run_wpctl(&["inspect", &node.id.to_string()])?;
    let device_id = property(&details, "device.id").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "the AE-5 PipeWire node has no device.id",
        )
    })?;
    parse_route_state(&run_pw_dump(&device_id)?, card_index)
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
    let Some((nodes, route)) = ae5_control_route(control, choice) else {
        return Ok(false);
    };
    let node = ae5_node(card_index, nodes)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no AE-5 {nodes} node for ALSA card {card_index}"),
        )
    })?;
    require_ae5_profile(node.id)?;
    run_wpctl(&["set-route", &node.id.to_string(), &route.to_string()])?;
    Ok(true)
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
        };
        if property(&details, "alsa.device").as_deref() == Some("0") {
            return Ok(Some(node));
        }
        fallback.get_or_insert(node);
    }
    Ok(fallback)
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
    for _ in 0..40 {
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
    Ok(())
}

fn ae5_control_route(control: &str, choice: &str) -> Option<(&'static str, u32)> {
    // These indices follow the exact path order enforced by check-ae5-acp-profile.sh.
    match (control, choice) {
        ("Input Source", "Microphone") => Some(("sources", 0)),
        ("Input Source", "Front Microphone") => Some(("sources", 1)),
        ("Input Source", "Line In") => Some(("sources", 2)),
        ("Output Select", "Speakers") => Some(("sinks", 3)),
        ("Output Select", "Headphone") => Some(("sinks", 6)),
        _ => None,
    }
}

fn parse_route_state(output: &str, card_index: i32) -> io::Result<PipeWireRouteState> {
    let objects = serde_json::from_str::<serde_json::Value>(output)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
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

    let profile_set = device["info"]["props"]["device.profile-set"]
        .as_str()
        .map(str::to_owned);
    let soft_mixer = json_bool(&device["info"]["props"]["api.alsa.soft-mixer"]);
    let active_profile = single_param_name(device, "Profile", None)?;
    let input_route = single_param_name(device, "Route", Some("Input"))?;
    let output_route = single_param_name(device, "Route", Some("Output"))?;
    Ok(PipeWireRouteState {
        profile_set,
        soft_mixer,
        active_profile,
        input_route,
        output_route,
    })
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

fn run_pw_dump(device_id: &str) -> io::Result<String> {
    let output = Command::new("pw-dump")
        .arg(device_id)
        .output()
        .map_err(|error| {
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
            format!("pw-dump {device_id} failed")
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
    for _ in 0..40 {
        match fs::read_to_string(&path) {
            Ok(status) if status.trim() == "closed" => return Ok(()),
            Ok(_) => thread::sleep(Duration::from_millis(25)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        format!("ALSA card {card_index} analog playback remained open after suspending PipeWire"),
    ))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NodeListing {
    id: u32,
    is_default: bool,
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
        listings.push(NodeListing { id, is_default });
    }
    listings
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
 │  *   49. alsa_output.pci-ae5.analog-stereo [vol: 0.40]
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
                },
                NodeListing {
                    id: 49,
                    is_default: true,
                },
            ]
        );
        assert_eq!(
            parse_status_node_list(listing, "sources"),
            vec![NodeListing {
                id: 50,
                is_default: true,
            }]
        );

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
                Some(("sources", 0)),
                Some(("sources", 1)),
                Some(("sources", 2)),
                Some(("sinks", 3)),
                Some(("sinks", 6)),
            ]
        );
        assert_eq!(ae5_control_route("Output Select", "Unknown"), None);
        assert_eq!(
            ae5_control_route("AE-5: Sound Filter", "Fast Roll Off"),
            None
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
              "api.alsa.soft-mixer": "true"
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
    }

    #[test]
    fn parses_and_validates_the_active_ae5_route() {
        let output = r#"[
          {
            "id": 55,
            "info": {
              "props": {
                "api.alsa.card": 0,
                "device.profile-set": "sound-blaster-ae5.conf",
                "api.alsa.soft-mixer": true
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
                    "name": "sound-blaster-ae5-output-headphones;output-headphones"
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
                active_profile: Some("output:analog-stereo+input:analog-stereo".to_owned()),
                input_route: Some("sound-blaster-ae5-input-microphone".to_owned()),
                output_route: Some(
                    "sound-blaster-ae5-output-headphones;output-headphones".to_owned()
                ),
            }
        );
        assert_eq!(state.output_issue("Headphone", "2.0"), None);
        assert_eq!(state.input_issue("Microphone"), None);
        assert_eq!(
            state.output_issue("Speakers", "2.0").as_deref(),
            Some(
                "ALSA selects Speakers, but PipeWire uses sound-blaster-ae5-output-headphones;output-headphones; reapply the output choice"
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
