use crate::pipewire::{
    PipeWireRouteState, ae5_route_state, restore_ae5_output_profile, set_ae5_control_route,
    set_ae5_output_profile, suspend_ae5_output,
};
use alsa::mixer::{Mixer, Selem, SelemChannelId};
use std::error::Error;
use std::fmt;
use std::io;
use std::time::Duration;

const CHANNELS: &[SelemChannelId] = &[
    SelemChannelId::FrontLeft,
    SelemChannelId::FrontRight,
    SelemChannelId::RearLeft,
    SelemChannelId::RearRight,
    SelemChannelId::FrontCenter,
    SelemChannelId::Woofer,
    SelemChannelId::SideLeft,
    SelemChannelId::SideRight,
    SelemChannelId::RearCenter,
];
const ROUTE_PLAYBACK_CONTROLS: &[&str] = &["Master", "Front", "Surround", "Center", "LFE", "PCM"];

pub(crate) const EQUALIZER_PRESET_CONTROL: &str = "FX: Equalizer Preset";
pub const DIRECT_MODE_CONTROL: &str = "AE-5: Direct Mode";
pub const HARDWARE_OUTFX_CONTROL: &str = "Enable OutFX";
const UNSAFE_HARDWARE_OUTFX: &str = "Hardware OutFX is disabled because AE-5 tests reproduced \
    severe stream distortion. Use software effects; recovering an already-corrupted route \
    requires a driver rebind or cold boot.";
const UNSAFE_DIRECT_MODE: &str = "Direct Mode is disabled because repeated AE-5 transitions \
    corrupted normal playback. Reboot into the maintained kernel without Direct Mode.";
const UNSAFE_OUTPUT_ROUTE_TRANSITION: &str = "Output route changes are disabled because they \
    suspend and reopen AE-5 playback, which reproduced severe stream distortion. Keep the current \
    route until the kernel PCM-reopen defect is fixed.";
const EQUALIZER_BAND_EDIT_BLOCK: &str = "Factory EQ presets use DSP values that the individual \
    1 dB controls cannot represent reliably. Select Flat before editing custom bands.";
const DIRECT_MODE_DSP_BLOCK: &str = "Direct Mode bypasses the CA0132 DSP, so this control has no \
    effect. Disable Direct Mode first.";
const FIXED_SMART_VOLUME_LEVEL: &str = "Loud and Night use fixed CA0132 DSP levels. Select Normal \
    to adjust the Smart Volume level.";
const INEFFECTIVE_WHAT_U_HEAR_CONTROL: &str = "The AE-5 DSP loopback bypasses this advertised \
    HDA gain and mute control. Use the recording application's stream-level volume or mute.";
const MUTED_HEADPHONE_PLAYBACK: &str = "ALSA selects Headphone, but Front playback is muted; \
    use the explicit route repair action";
const UNVERIFIED_HEADPHONE_PLAYBACK: &str =
    "ALSA selects Headphone, but the Front playback switch is unavailable";
const MUTED_HEADPHONE_MASTER: &str = "ALSA selects Headphone, but hardware Master playback is \
    muted; PipeWire's software mute cannot unmute it, so use the explicit route repair action";
const UNVERIFIED_HEADPHONE_MASTER: &str =
    "ALSA selects Headphone, but the hardware Master playback switch is unavailable";

#[derive(Debug)]
pub enum ControlError {
    Alsa(alsa::Error),
    DesktopRoute(io::Error),
    Missing(String),
    Invalid(String),
    Verification(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Level {
    pub value: i64,
    pub min: i64,
    pub max: i64,
    pub db: Option<DecibelRange>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DecibelRange {
    /// ALSA represents decibels in hundredths of a dB.
    pub min: i64,
    pub max: i64,
    pub step: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChannelLevel {
    pub name: String,
    pub value: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ControlSnapshot {
    pub name: String,
    pub selected: Option<String>,
    pub choices: Vec<String>,
    pub playback_switch: Option<bool>,
    pub capture_switch: Option<bool>,
    pub playback_level: Option<Level>,
    pub capture_level: Option<Level>,
    pub playback_channels: Vec<ChannelLevel>,
    pub capture_channels: Vec<ChannelLevel>,
}

#[derive(Debug)]
pub struct Ae5Mixer {
    mixer: Mixer,
    card_index: i32,
}

#[derive(Debug, Default, Eq, PartialEq)]
struct RouteRepairPlan {
    output: Option<String>,
    input: Option<String>,
    unmute_master: bool,
    unmute_front: bool,
}

pub fn snapshot_controls(card_index: i32) -> alsa::Result<Vec<ControlSnapshot>> {
    Ae5Mixer::open(card_index)?.snapshots()
}

pub fn playback_switch_block_reason(
    name: &str,
    enabled: bool,
    controls: &[ControlSnapshot],
) -> Option<&'static str> {
    if let Some(reason) = unsafe_playback_control_block_reason(name) {
        return Some(reason);
    }
    if !enabled {
        return None;
    }
    if let Some(reason) = direct_mode_block_reason(name, controls) {
        return Some(reason);
    }
    bass_switch_block_reason(name, controls)
}

pub fn unsafe_playback_control_block_reason(name: &str) -> Option<&'static str> {
    if name == DIRECT_MODE_CONTROL {
        Some(UNSAFE_DIRECT_MODE)
    } else if is_unsafe_output_route_control(name) {
        Some(UNSAFE_OUTPUT_ROUTE_TRANSITION)
    } else if is_unsafe_hardware_playback_control(name) {
        Some(UNSAFE_HARDWARE_OUTFX)
    } else {
        None
    }
}

fn bass_switch_block_reason(name: &str, controls: &[ControlSnapshot]) -> Option<&'static str> {
    let speakers_selected = controls.iter().any(|control| {
        control.name == "Output Select" && control.selected.as_deref() == Some("Speakers")
    });
    let has_lfe = controls.iter().any(|control| {
        control.name == "Surround Channel Config"
            && control
                .selected
                .as_deref()
                .is_some_and(|layout| layout.ends_with(".1"))
    });

    match name {
        "Bass Redirection" if !speakers_selected => {
            Some("Select Speakers output before enabling bass redirection.")
        }
        "Bass Redirection" if !has_lfe => Some("Select a 2.1, 4.1, or 5.1 speaker layout first."),
        "Bass Redirection"
            if controls.iter().any(|control| {
                control.name == "FX: X-Bass" && control.playback_switch == Some(true)
            }) =>
        {
            Some("Turn off X-Bass before enabling speaker bass redirection.")
        }
        "FX: X-Bass" if speakers_selected && has_lfe => {
            Some("X-Bass is unavailable for speaker layouts with an LFE channel.")
        }
        _ => None,
    }
}

pub fn equalizer_band_block_reason(
    name: &str,
    controls: &[ControlSnapshot],
) -> Option<&'static str> {
    if !is_equalizer_band(name) {
        return None;
    }
    if let Some(reason) = direct_mode_block_reason(name, controls) {
        return Some(reason);
    }
    controls
        .iter()
        .find(|control| control.name == EQUALIZER_PRESET_CONTROL)
        .and_then(|control| control.selected.as_deref())
        .filter(|preset| !preset.eq_ignore_ascii_case("Flat"))
        .map(|_| EQUALIZER_BAND_EDIT_BLOCK)
}

pub fn direct_mode_block_reason(name: &str, controls: &[ControlSnapshot]) -> Option<&'static str> {
    if name == DIRECT_MODE_CONTROL || !is_direct_mode_bypassed_control(name, controls) {
        return None;
    }
    controls
        .iter()
        .any(|control| control.name == DIRECT_MODE_CONTROL && control.playback_switch == Some(true))
        .then_some(DIRECT_MODE_DSP_BLOCK)
}

pub fn smart_volume_level_block_reason(
    name: &str,
    controls: &[ControlSnapshot],
) -> Option<&'static str> {
    if name != "FX: Smart Volume" {
        return None;
    }
    controls
        .iter()
        .find(|control| control.name == "FX: Smart Volume Setting")
        .and_then(|control| control.selected.as_deref())
        .is_some_and(|mode| mode.eq_ignore_ascii_case("Loud") || mode.eq_ignore_ascii_case("Night"))
        .then_some(FIXED_SMART_VOLUME_LEVEL)
}

pub fn capture_control_block_reason(name: &str) -> Option<&'static str> {
    (name == "What U Hear").then_some(INEFFECTIVE_WHAT_U_HEAR_CONTROL)
}

pub fn headphone_playback_issue(controls: &[ControlSnapshot]) -> Option<&'static str> {
    let headphone_selected = controls.iter().any(|control| {
        control.name == "Output Select" && control.selected.as_deref() == Some("Headphone")
    });
    let direct_mode = controls.iter().any(|control| {
        control.name == DIRECT_MODE_CONTROL && control.playback_switch == Some(true)
    });
    if !headphone_selected || direct_mode {
        return None;
    }
    match controls
        .iter()
        .find(|control| control.name == "Master")
        .and_then(|control| control.playback_switch)
    {
        Some(true) => {}
        Some(false) => return Some(MUTED_HEADPHONE_MASTER),
        None => return Some(UNVERIFIED_HEADPHONE_MASTER),
    }
    match controls
        .iter()
        .find(|control| control.name == "Front")
        .and_then(|control| control.playback_switch)
    {
        Some(true) => None,
        Some(false) => Some(MUTED_HEADPHONE_PLAYBACK),
        None => Some(UNVERIFIED_HEADPHONE_PLAYBACK),
    }
}

pub fn front_vmaster_clamp_warning(controls: &[ControlSnapshot]) -> Option<String> {
    let master = controls
        .iter()
        .find(|control| control.name == "Master")?
        .playback_level
        .as_ref()?;
    let front = controls
        .iter()
        .find(|control| control.name == "Front")?
        .playback_level
        .as_ref()?;
    let effective = (front.value + master.value - master.max).clamp(front.min, front.max);
    if effective != front.min {
        return None;
    }

    let last_clamped_master = (master.max + front.min - front.value).clamp(master.min, master.max);
    let consequence = if last_clamped_master < master.max {
        format!(
            "Master changes remain at the floor through {last_clamped_master}/{} while Front stays at {}/{}.",
            master.max, front.value, front.max
        )
    } else {
        "Master cannot raise effective Front until the Front level changes.".to_owned()
    };
    Some(format!(
        "ALSA's virtual Master and Front attenuations stack: effective Front is {effective}/{} \
         ({} + {} − {}). {consequence}",
        front.max, master.value, front.value, master.max
    ))
}

pub(crate) fn is_equalizer_band(name: &str) -> bool {
    name.strip_prefix("EQ Band")
        .is_some_and(|band| band.parse::<u8>().is_ok_and(|band| band < 10))
}

pub(crate) fn is_unsafe_hardware_playback_control(name: &str) -> bool {
    name == HARDWARE_OUTFX_CONTROL
        || name == DIRECT_MODE_CONTROL
        || is_equalizer_band(name)
        || matches!(
            name,
            "FX: Surround"
                | "FX: Crystalizer"
                | "FX: Dialog Plus"
                | "FX: Smart Volume"
                | "FX: Smart Volume Setting"
                | "FX: X-Bass"
                | "FX: X-Bass Crossover"
                | "FX: Equalizer"
                | EQUALIZER_PRESET_CONTROL
        )
}

pub(crate) fn is_unsafe_output_route_control(name: &str) -> bool {
    matches!(name, "Output Select" | "Surround Channel Config")
}

fn is_direct_mode_bypassed_control(name: &str, controls: &[ControlSnapshot]) -> bool {
    let is_playback_effect = controls.iter().any(|control| {
        control.name == name
            && name.starts_with("FX:")
            && (control.playback_switch.is_some() || control.playback_level.is_some())
    });

    name == "Enable OutFX"
        || is_playback_effect
        || name.starts_with("EQ Band")
        || matches!(
            name,
            "FX: Equalizer Preset"
                | "FX: Smart Volume Setting"
                | "FX: X-Bass Crossover"
                | "Bass Redirection"
                | "Bass Redirection Crossover"
                | "Surround Channel Config"
                | "Full-Range Front Speakers"
                | "Full-Range Rear Speakers"
                | "Front"
                | "Surround"
                | "Center"
                | "LFE"
                | "Master"
                | "PCM"
        )
}

pub(crate) fn invalid_bass_state_reason(controls: &[ControlSnapshot]) -> Option<&'static str> {
    ["Bass Redirection", "FX: X-Bass"]
        .into_iter()
        .find_map(|name| {
            let enabled = controls
                .iter()
                .find(|control| control.name == name)
                .and_then(|control| control.playback_switch)
                .unwrap_or(false);
            enabled
                .then(|| bass_switch_block_reason(name, controls))
                .flatten()
        })
}

impl Ae5Mixer {
    pub fn open(card_index: i32) -> alsa::Result<Self> {
        Ok(Self {
            mixer: Mixer::new(&format!("hw:{card_index}"), false)?,
            card_index,
        })
    }

    pub fn snapshots(&self) -> alsa::Result<Vec<ControlSnapshot>> {
        let mut controls = self
            .mixer
            .iter()
            .filter_map(Selem::new)
            .map(read_control)
            .collect::<alsa::Result<Vec<_>>>()?;
        controls.sort_unstable_by(|left, right| left.name.cmp(&right.name));
        Ok(controls)
    }

    pub fn repair_routes(&self) -> Result<Vec<String>, ControlError> {
        let controls = self.snapshots()?;
        let state = ae5_route_state(self.card_index)?;
        let plan = route_repair_plan(&controls, &state)?;
        if plan == RouteRepairPlan::default() {
            return Ok(Vec::new());
        }
        let mut changes = Vec::new();

        if let Some(output) = &plan.output {
            self.set_choice("Output Select", output)?;
            changes.push(format!("reapplied {output} output"));
        }
        if let Some(input) = &plan.input {
            self.set_choice("Input Source", input)?;
            changes.push(format!("reapplied {input} input"));
        }
        if plan.unmute_master || plan.unmute_front {
            if plan.unmute_master {
                self.set_playback_switch("Master", true)?;
                changes.push("unmuted hardware Master playback".to_owned());
            }
            if plan.unmute_front {
                self.set_playback_switch("Front", true)?;
                changes.push("unmuted Front playback".to_owned());
            }
        }

        let controls = self.snapshots()?;
        let state = ae5_route_state(self.card_index)?;
        let remaining = route_repair_plan(&controls, &state)?;
        if remaining != RouteRepairPlan::default() {
            return Err(ControlError::Verification(format!(
                "route repair did not converge: {remaining:?}"
            )));
        }
        Ok(changes)
    }

    pub fn wait_for_event(&self, timeout: Duration) -> alsa::Result<bool> {
        let timeout_ms = timeout.as_millis().min(u32::MAX.into()) as u32;
        self.mixer.wait(Some(timeout_ms))?;
        Ok(self.mixer.handle_events()? > 0)
    }

    pub fn snapshot(&self, name: &str) -> Result<ControlSnapshot, ControlError> {
        read_control(self.find(name)?).map_err(Into::into)
    }

    pub fn set_choice(&self, name: &str, requested: &str) -> Result<ControlSnapshot, ControlError> {
        self.set_choice_checked(name, requested, false)
    }

    pub fn set_choice_checked(
        &self,
        name: &str,
        requested: &str,
        allow_high_gain: bool,
    ) -> Result<ControlSnapshot, ControlError> {
        self.ensure_safe_playback_control(name)?;
        if is_high_headphone_gain(name, requested) && !allow_high_gain {
            return Err(ControlError::Invalid(
                "high headphone gain requires explicit approval".to_owned(),
            ));
        }
        let element = self.find(name)?;
        if !element.is_enumerated() {
            return Err(ControlError::Invalid(format!(
                "'{name}' is not an enumerated control"
            )));
        }

        let choices = element.iter_enum()?.collect::<alsa::Result<Vec<_>>>()?;
        let Some(index) = choice_index(&choices, requested) else {
            return Err(ControlError::Invalid(format!(
                "'{requested}' is not valid for '{name}'; expected one of: {}",
                choices.join(", ")
            )));
        };
        let expected = &choices[index];
        let previous = choices
            .get(element.get_enum_item(SelemChannelId::FrontLeft)? as usize)
            .cloned();
        let changes_output_route = matches!(name, "Output Select" | "Surround Channel Config");
        let previous_controls = changes_output_route.then(|| self.snapshots()).transpose()?;
        let projected_controls = if let Some(controls) = &previous_controls {
            let mut controls = controls.clone();
            if let Some(control) = controls.iter_mut().find(|control| control.name == name)
                && control.selected.as_deref() != Some(expected)
            {
                control.selected = Some(expected.clone());
                if let Some(reason) = invalid_bass_state_reason(&controls) {
                    return Err(ControlError::Invalid(reason.to_owned()));
                }
            }
            Some(controls)
        } else {
            None
        };
        let mut suspended = changes_output_route
            .then(|| suspend_ae5_output(self.card_index))
            .transpose()?;
        let mut previous_profile = None;

        let applied = (|| {
            if let Some(controls) = &projected_controls {
                let output = selected_choice(controls, "Output Select")?;
                let layout = selected_choice(controls, "Surround Channel Config")?;
                previous_profile = set_ae5_output_profile(self.card_index, output, layout)?;
                suspended
                    .as_mut()
                    .expect("output-route changes suspend the sink")
                    .ensure_current_suspended()?;
            }
            let routed = set_ae5_control_route(self.card_index, name, expected)?;
            if !routed {
                element.set_enum_item(SelemChannelId::FrontLeft, index as u32)?;
            }
            if let Some(controls) = &previous_controls {
                suspended
                    .as_mut()
                    .expect("output-route changes suspend the sink")
                    .ensure_current_suspended()?;
                self.restore_route_playback_state(controls)?;
            }
            let actual = read_control(element)?;
            if actual.selected.as_deref() != Some(expected) {
                return Err(ControlError::Verification(format!(
                    "'{name}' read back as {:?}, expected '{expected}'",
                    actual.selected,
                )));
            }
            Ok(actual)
        })();
        let result = match applied {
            Ok(actual) => Ok(actual),
            Err(error) => {
                if let Some(suspended) = suspended.as_mut() {
                    let _ = suspended.ensure_current_suspended();
                }
                let profile_rollback = previous_profile.as_deref().map_or_else(
                    || "PipeWire profile unchanged".to_owned(),
                    |profile| match restore_ae5_output_profile(self.card_index, profile) {
                        Ok(()) => format!("restored PipeWire profile '{profile}'"),
                        Err(error) => format!("PipeWire profile rollback failed: {error}"),
                    },
                );
                if let Some(suspended) = suspended.as_mut() {
                    let _ = suspended.ensure_current_suspended();
                }
                let choice_rollback = previous.as_deref().map_or_else(
                    || "previous ALSA choice unavailable".to_owned(),
                    |choice| self.restore_choice(name, choice),
                );
                if let Some(suspended) = suspended.as_mut() {
                    let _ = suspended.ensure_current_suspended();
                }
                let playback_rollback = previous_controls.as_deref().map_or_else(
                    || "route-sensitive playback state unchanged".to_owned(),
                    |controls| match self.restore_route_playback_state(controls) {
                        Ok(()) => "restored route-sensitive playback state".to_owned(),
                        Err(error) => format!("playback-state rollback failed: {error}"),
                    },
                );
                Err(ControlError::Verification(format!(
                    "{error}; {profile_rollback}; {choice_rollback}; {playback_rollback}"
                )))
            }
        };
        let resumed = suspended
            .map(|output| output.resume())
            .transpose()
            .map_err(ControlError::from);
        match (result, resumed) {
            (Ok(actual), Ok(_)) => Ok(actual),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    pub fn set_playback_switch(
        &self,
        name: &str,
        enabled: bool,
    ) -> Result<ControlSnapshot, ControlError> {
        let element = self.find(name)?;
        if !element.has_playback_switch() {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no playback switch"
            )));
        }
        if let Some(reason) = playback_switch_block_reason(name, enabled, &self.snapshots()?) {
            return Err(ControlError::Invalid(reason.to_owned()));
        }
        if name == DIRECT_MODE_CONTROL {
            let current = self.snapshot(name)?;
            if current.playback_switch == Some(enabled) {
                return Ok(current);
            }
        }
        let set_and_verify = || {
            element.set_playback_switch_all(i32::from(enabled))?;
            let actual = read_control(element)?;
            verify(name, "playback switch", enabled, actual.playback_switch)?;
            Ok(actual)
        };
        if name != DIRECT_MODE_CONTROL {
            return set_and_verify();
        }

        let suspended = suspend_ae5_output(self.card_index)?;
        let result = set_and_verify();
        let resumed = suspended.resume().map_err(ControlError::from);
        match (result, resumed) {
            (Ok(actual), Ok(())) => Ok(actual),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(error),
        }
    }

    pub fn set_capture_switch(
        &self,
        name: &str,
        enabled: bool,
    ) -> Result<ControlSnapshot, ControlError> {
        if let Some(reason) = capture_control_block_reason(name) {
            return Err(ControlError::Invalid(reason.to_owned()));
        }
        let element = self.find(name)?;
        if !element.has_capture_switch() {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no capture switch"
            )));
        }
        element.set_capture_switch_all(i32::from(enabled))?;
        let actual = read_control(element)?;
        verify(name, "capture switch", enabled, actual.capture_switch)?;
        Ok(actual)
    }

    pub fn set_playback_level(
        &self,
        name: &str,
        value: i64,
    ) -> Result<ControlSnapshot, ControlError> {
        self.ensure_safe_playback_control(name)?;
        self.ensure_equalizer_band_editable(name)?;
        let element = self.find(name)?;
        if !element.has_playback_volume() {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no playback level"
            )));
        }
        let (min, max) = element.get_playback_volume_range();
        validate_range(name, value, min, max)?;
        element.set_playback_volume_all(value)?;
        let actual = read_control(element)?;
        verify_channels(name, "playback level", value, &actual.playback_channels)?;
        Ok(actual)
    }

    pub fn set_playback_channel_level(
        &self,
        name: &str,
        channel: &str,
        value: i64,
    ) -> Result<ControlSnapshot, ControlError> {
        self.ensure_safe_playback_control(name)?;
        self.ensure_equalizer_band_editable(name)?;
        let element = self.find(name)?;
        if !element.has_playback_volume() {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no playback level"
            )));
        }
        let (min, max) = element.get_playback_volume_range();
        validate_range(name, value, min, max)?;
        let channel_id = find_channel(&element, channel, false)?;
        element.set_playback_volume(channel_id, value)?;
        let actual = read_control(element)?;
        verify_channel(name, "playback", channel, value, &actual.playback_channels)?;
        Ok(actual)
    }

    pub fn set_capture_level(
        &self,
        name: &str,
        value: i64,
    ) -> Result<ControlSnapshot, ControlError> {
        if let Some(reason) = capture_control_block_reason(name) {
            return Err(ControlError::Invalid(reason.to_owned()));
        }
        let element = self.find(name)?;
        if !element.has_capture_volume() {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no capture level"
            )));
        }
        let (min, max) = element.get_capture_volume_range();
        validate_range(name, value, min, max)?;
        element.set_capture_volume_all(value)?;
        let actual = read_control(element)?;
        verify_channels(name, "capture level", value, &actual.capture_channels)?;
        Ok(actual)
    }

    pub fn set_capture_channel_level(
        &self,
        name: &str,
        channel: &str,
        value: i64,
    ) -> Result<ControlSnapshot, ControlError> {
        if let Some(reason) = capture_control_block_reason(name) {
            return Err(ControlError::Invalid(reason.to_owned()));
        }
        let element = self.find(name)?;
        if !element.has_capture_volume() || name == "Bass Redirection Crossover" {
            return Err(ControlError::Invalid(format!(
                "'{name}' has no capture level"
            )));
        }
        let (min, max) = element.get_capture_volume_range();
        validate_range(name, value, min, max)?;
        let channel_id = find_channel(&element, channel, true)?;
        element.set_capture_volume(channel_id, value)?;
        let actual = read_control(element)?;
        verify_channel(name, "capture", channel, value, &actual.capture_channels)?;
        Ok(actual)
    }

    fn find(&self, name: &str) -> Result<Selem<'_>, ControlError> {
        self.mixer
            .find_selem(&alsa::mixer::SelemId::new(name, 0))
            .ok_or_else(|| ControlError::Missing(name.to_owned()))
    }

    fn ensure_equalizer_band_editable(&self, name: &str) -> Result<(), ControlError> {
        if is_equalizer_band(name)
            && let Some(reason) = equalizer_band_block_reason(name, &self.snapshots()?)
        {
            return Err(ControlError::Invalid(reason.to_owned()));
        }
        Ok(())
    }

    fn ensure_safe_playback_control(&self, name: &str) -> Result<(), ControlError> {
        if let Some(reason) = unsafe_playback_control_block_reason(name) {
            return Err(ControlError::Invalid(reason.to_owned()));
        }
        Ok(())
    }

    fn restore_choice(&self, name: &str, previous: &str) -> String {
        let restored = (|| {
            if !set_ae5_control_route(self.card_index, name, previous)? {
                let element = self.find(name)?;
                let choices = element.iter_enum()?.collect::<alsa::Result<Vec<_>>>()?;
                let index = choice_index(&choices, previous).ok_or_else(|| {
                    ControlError::Invalid(format!("'{previous}' is no longer valid for '{name}'"))
                })?;
                element.set_enum_item(SelemChannelId::FrontLeft, index as u32)?;
            }
            let control = self.snapshot(name)?;
            if control.selected.as_deref() != Some(previous) {
                return Err(ControlError::Verification(format!(
                    "rollback read back as {:?}, expected '{previous}'",
                    control.selected
                )));
            }
            Ok(())
        })();
        match restored {
            Ok(()) => format!("restored '{previous}'"),
            Err(error) => format!("choice rollback failed: {error}"),
        }
    }

    fn restore_route_playback_state(
        &self,
        controls: &[ControlSnapshot],
    ) -> Result<(), ControlError> {
        for name in ROUTE_PLAYBACK_CONTROLS {
            let Some(previous) = controls.iter().find(|control| control.name == *name) else {
                continue;
            };
            let element = self.find(name)?;
            for channel in &previous.playback_channels {
                let channel_id = find_channel(&element, &channel.name, false)?;
                element.set_playback_volume(channel_id, channel.value)?;
            }
            if *name == "Master"
                && let Some(enabled) = previous.playback_switch
            {
                element.set_playback_switch_all(i32::from(enabled))?;
            }
        }
        for _ in 0..40 {
            let mut matched = true;
            for name in ROUTE_PLAYBACK_CONTROLS {
                let Some(previous) = controls.iter().find(|control| control.name == *name) else {
                    continue;
                };
                let actual = self.snapshot(name)?;
                matched &= (*name != "Master"
                    || actual.playback_switch == previous.playback_switch)
                    && actual.playback_channels == previous.playback_channels;
            }
            if matched {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        Err(ControlError::Verification(
            "route-sensitive playback state did not settle to its previous values".to_owned(),
        ))
    }
}

fn selected_choice<'a>(
    controls: &'a [ControlSnapshot],
    name: &str,
) -> Result<&'a str, ControlError> {
    controls
        .iter()
        .find(|control| control.name == name)
        .and_then(|control| control.selected.as_deref())
        .ok_or_else(|| ControlError::Missing(name.to_owned()))
}

fn route_repair_plan(
    controls: &[ControlSnapshot],
    state: &PipeWireRouteState,
) -> Result<RouteRepairPlan, ControlError> {
    let output = selected_choice(controls, "Output Select")?;
    let layout = selected_choice(controls, "Surround Channel Config")?;
    let input = selected_choice(controls, "Input Source")?;
    let direct_mode = controls.iter().any(|control| {
        control.name == DIRECT_MODE_CONTROL && control.playback_switch == Some(true)
    });
    let unmute_master = if output == "Headphone" && !direct_mode {
        match controls
            .iter()
            .find(|control| control.name == "Master")
            .and_then(|control| control.playback_switch)
        {
            Some(enabled) => !enabled,
            None => {
                return Err(ControlError::Missing("Master playback switch".to_owned()));
            }
        }
    } else {
        false
    };
    let unmute_front = if output == "Headphone" && !direct_mode {
        match controls
            .iter()
            .find(|control| control.name == "Front")
            .and_then(|control| control.playback_switch)
        {
            Some(enabled) => !enabled,
            None => return Err(ControlError::Missing("Front playback switch".to_owned())),
        }
    } else {
        false
    };
    Ok(RouteRepairPlan {
        output: state
            .output_issue(output, layout)
            .map(|_| output.to_owned()),
        input: state.input_issue(input).map(|_| input.to_owned()),
        unmute_master,
        unmute_front,
    })
}

fn read_control(element: Selem<'_>) -> alsa::Result<ControlSnapshot> {
    let id = element.get_id();
    let name = id.get_name()?.to_owned();
    let (selected, choices) = if element.is_enumerated() {
        let choices = element.iter_enum()?.collect::<alsa::Result<Vec<_>>>()?;
        let selected_index = element.get_enum_item(SelemChannelId::FrontLeft)? as usize;
        (choices.get(selected_index).cloned(), choices)
    } else {
        (None, Vec::new())
    };
    let has_capture_level = name != "Bass Redirection Crossover" && element.has_capture_volume();
    let playback_channels = if element.has_playback_volume() {
        read_channels(&element, false)?
    } else {
        Vec::new()
    };
    let capture_channels = if has_capture_level {
        read_channels(&element, true)?
    } else {
        Vec::new()
    };

    Ok(ControlSnapshot {
        name,
        selected,
        choices,
        playback_switch: element
            .has_playback_switch()
            .then(|| {
                element
                    .get_playback_switch(SelemChannelId::FrontLeft)
                    .map(|value| value != 0)
            })
            .transpose()?,
        capture_switch: element
            .has_capture_switch()
            .then(|| {
                element
                    .get_capture_switch(SelemChannelId::FrontLeft)
                    .map(|value| value != 0)
            })
            .transpose()?,
        playback_level: playback_channels.first().map(|channel| {
            let (min, max) = element.get_playback_volume_range();
            Level {
                value: channel.value,
                min,
                max,
                db: playback_db_range(&element, min, max),
            }
        }),
        capture_level: capture_channels.first().map(|channel| {
            let (min, max) = element.get_capture_volume_range();
            Level {
                value: channel.value,
                min,
                max,
                db: capture_db_range(&element, min, max),
            }
        }),
        playback_channels,
        capture_channels,
    })
}

fn playback_db_range(element: &Selem<'_>, min: i64, max: i64) -> Option<DecibelRange> {
    let db_min = element.ask_playback_vol_db(min).ok()?.0;
    Some(DecibelRange {
        min: db_min,
        max: element.ask_playback_vol_db(max).ok()?.0,
        step: element.ask_playback_vol_db((min + 1).min(max)).ok()?.0 - db_min,
    })
}

fn capture_db_range(element: &Selem<'_>, min: i64, max: i64) -> Option<DecibelRange> {
    let db_min = element.ask_capture_vol_db(min).ok()?.0;
    Some(DecibelRange {
        min: db_min,
        max: element.ask_capture_vol_db(max).ok()?.0,
        step: element.ask_capture_vol_db((min + 1).min(max)).ok()?.0 - db_min,
    })
}

fn read_channels(element: &Selem<'_>, capture: bool) -> alsa::Result<Vec<ChannelLevel>> {
    CHANNELS
        .iter()
        .copied()
        .filter(|channel| {
            if capture {
                element.has_capture_channel(*channel)
            } else {
                element.has_playback_channel(*channel)
            }
        })
        .map(|channel| {
            let value = if capture {
                element.get_capture_volume(channel)?
            } else {
                element.get_playback_volume(channel)?
            };
            Ok(ChannelLevel {
                name: Selem::channel_name(channel)?.to_owned(),
                value,
            })
        })
        .collect()
}

fn find_channel(
    element: &Selem<'_>,
    requested: &str,
    capture: bool,
) -> Result<SelemChannelId, ControlError> {
    let channels = CHANNELS
        .iter()
        .copied()
        .filter(|channel| {
            if capture {
                element.has_capture_channel(*channel)
            } else {
                element.has_playback_channel(*channel)
            }
        })
        .collect::<Vec<_>>();
    channels
        .iter()
        .copied()
        .find(|channel| {
            Selem::channel_name(*channel).is_ok_and(|name| name.eq_ignore_ascii_case(requested))
        })
        .ok_or_else(|| {
            let choices = channels
                .iter()
                .filter_map(|channel| Selem::channel_name(*channel).ok())
                .collect::<Vec<_>>()
                .join(", ");
            ControlError::Invalid(format!(
                "'{requested}' is not a valid channel; expected one of: {choices}"
            ))
        })
}

fn choice_index(choices: &[String], requested: &str) -> Option<usize> {
    choices
        .iter()
        .position(|choice| choice.eq_ignore_ascii_case(requested))
}

fn is_high_headphone_gain(name: &str, requested: &str) -> bool {
    name == "AE-5: Headphone Gain" && requested.to_ascii_lowercase().starts_with("high")
}

fn validate_range(name: &str, value: i64, min: i64, max: i64) -> Result<(), ControlError> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(ControlError::Invalid(format!(
            "{value} is outside the valid range for '{name}' ({min}..{max})"
        )))
    }
}

fn verify<T>(name: &str, field: &str, expected: T, actual: Option<T>) -> Result<(), ControlError>
where
    T: Copy + fmt::Debug + PartialEq,
{
    if actual == Some(expected) {
        Ok(())
    } else {
        Err(ControlError::Verification(format!(
            "'{name}' {field} read back as {actual:?}, expected {expected:?}"
        )))
    }
}

fn verify_channels(
    name: &str,
    field: &str,
    expected: i64,
    actual: &[ChannelLevel],
) -> Result<(), ControlError> {
    if !actual.is_empty() && actual.iter().all(|channel| channel.value == expected) {
        Ok(())
    } else {
        Err(ControlError::Verification(format!(
            "'{name}' {field} read back as {actual:?}, expected every channel to be {expected}"
        )))
    }
}

fn verify_channel(
    name: &str,
    field: &str,
    channel: &str,
    expected: i64,
    actual: &[ChannelLevel],
) -> Result<(), ControlError> {
    let value = actual
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case(channel))
        .map(|candidate| candidate.value);
    verify(
        name,
        &format!("{field} channel '{channel}' level"),
        expected,
        value,
    )
}

impl fmt::Display for ControlError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Alsa(error) => write!(output, "{error}"),
            Self::DesktopRoute(error) => write!(output, "desktop audio operation failed: {error}"),
            Self::Missing(name) => write!(output, "ALSA control '{name}' is unavailable"),
            Self::Invalid(message) | Self::Verification(message) => output.write_str(message),
        }
    }
}

impl Error for ControlError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Alsa(error) => Some(error),
            Self::DesktopRoute(error) => Some(error),
            _ => None,
        }
    }
}

impl From<alsa::Error> for ControlError {
    fn from(error: alsa::Error) -> Self {
        Self::Alsa(error)
    }
}

impl From<io::Error> for ControlError {
    fn from(error: io::Error) -> Self {
        Self::DesktopRoute(error)
    }
}

impl fmt::Display for ControlSnapshot {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "{}", self.name)?;
        if let Some(selected) = &self.selected {
            write!(output, ": {selected}")?;
        }
        if let Some(enabled) = self.playback_switch {
            write!(output, " | playback {}", on_off(enabled))?;
        }
        if let Some(level) = &self.playback_level {
            write!(
                output,
                " | playback level {} [{}..{}]",
                level.value, level.min, level.max
            )?;
            if let Some(db) = &level.db {
                write!(
                    output,
                    " | playback dB {}..{} ({} step)",
                    format_db(db.min),
                    format_db(db.max),
                    format_db(db.step)
                )?;
            }
        }
        if let Some(enabled) = self.capture_switch {
            write!(output, " | capture {}", on_off(enabled))?;
        }
        if let Some(level) = &self.capture_level {
            write!(
                output,
                " | capture level {} [{}..{}]",
                level.value, level.min, level.max
            )?;
            if let Some(db) = &level.db {
                write!(
                    output,
                    " | capture dB {}..{} ({} step)",
                    format_db(db.min),
                    format_db(db.max),
                    format_db(db.step)
                )?;
            }
        }
        if self.playback_channels.len() > 1 {
            write!(
                output,
                " | playback {}",
                format_channels(&self.playback_channels)
            )?;
        }
        if self.capture_channels.len() > 1 {
            write!(
                output,
                " | capture {}",
                format_channels(&self.capture_channels)
            )?;
        }
        Ok(())
    }
}

fn format_db(hundredths: i64) -> String {
    format!("{:+.2} dB", hundredths as f64 / 100.0)
}

fn format_channels(channels: &[ChannelLevel]) -> String {
    channels
        .iter()
        .map(|channel| format!("{}={}", channel.name, channel.value))
        .collect::<Vec<_>>()
        .join(", ")
}

fn on_off(enabled: bool) -> &'static str {
    if enabled { "on" } else { "off" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn playback_switch(name: &str, enabled: bool) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: Some(enabled),
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        }
    }

    fn capture_switch(name: &str, enabled: bool) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: None,
            capture_switch: Some(enabled),
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        }
    }

    fn selected_choice(name: &str, selected: &str) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: Some(selected.to_owned()),
            choices: vec![selected.to_owned()],
            playback_switch: None,
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        }
    }

    fn playback_level(name: &str, value: i64, min: i64, max: i64) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: None,
            capture_switch: None,
            playback_level: Some(Level {
                value,
                min,
                max,
                db: None,
            }),
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        }
    }

    #[test]
    fn formats_a_compound_control_readably() {
        let control = ControlSnapshot {
            name: "FX: Crystalizer".to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: Some(true),
            capture_switch: None,
            playback_level: Some(Level {
                value: 65,
                min: 0,
                max: 100,
                db: None,
            }),
            capture_level: None,
            playback_channels: vec![ChannelLevel {
                name: "Front Left".to_owned(),
                value: 65,
            }],
            capture_channels: Vec::new(),
        };

        assert_eq!(
            control.to_string(),
            "FX: Crystalizer | playback on | playback level 65 [0..100]"
        );
    }

    #[test]
    fn formats_the_live_alsa_db_mapping() {
        let control = ControlSnapshot {
            name: "EQ Band0".to_owned(),
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
        };

        assert_eq!(
            control.to_string(),
            "EQ Band0 | playback level 24 [0..48] | playback dB -24.00 dB..+24.00 dB (+1.00 dB step)"
        );
    }

    #[test]
    fn validates_choices_and_ranges_before_hardware_writes() {
        let choices = vec!["Speakers".to_owned(), "Headphone".to_owned()];
        assert_eq!(choice_index(&choices, "headphone"), Some(1));
        assert_eq!(choice_index(&choices, "HDMI"), None);
        assert!(is_high_headphone_gain(
            "AE-5: Headphone Gain",
            "High (150-600 Ohms)"
        ));
        assert!(!is_high_headphone_gain(
            "AE-5: Headphone Gain",
            "Medium (32-149 Ohms)"
        ));
        assert!(validate_range("Level", 50, 0, 100).is_ok());
        assert!(validate_range("Level", 101, 0, 100).is_err());
    }

    #[test]
    fn rejects_incompatible_bass_management_and_blocks_hardware_xbass() {
        let controls = vec![
            playback_switch("FX: X-Bass", true),
            playback_switch("Bass Redirection", false),
            selected_choice("Surround Channel Config", "5.1"),
            selected_choice("Output Select", "Speakers"),
        ];

        assert_eq!(
            playback_switch_block_reason("Bass Redirection", true, &controls),
            Some("Turn off X-Bass before enabling speaker bass redirection.")
        );
        assert_eq!(
            playback_switch_block_reason("FX: X-Bass", false, &controls),
            Some(UNSAFE_HARDWARE_OUTFX)
        );
        assert_eq!(
            invalid_bass_state_reason(&controls),
            Some("X-Bass is unavailable for speaker layouts with an LFE channel.")
        );

        let controls = vec![
            playback_switch("FX: X-Bass", false),
            playback_switch("Bass Redirection", false),
            selected_choice("Surround Channel Config", "2.0"),
            selected_choice("Output Select", "Speakers"),
        ];
        assert_eq!(
            playback_switch_block_reason("Bass Redirection", true, &controls),
            Some("Select a 2.1, 4.1, or 5.1 speaker layout first.")
        );

        let controls = vec![
            playback_switch("FX: X-Bass", false),
            playback_switch("Bass Redirection", false),
            selected_choice("Surround Channel Config", "5.1"),
            selected_choice("Output Select", "Headphone"),
        ];
        assert_eq!(
            playback_switch_block_reason("Bass Redirection", true, &controls),
            Some("Select Speakers output before enabling bass redirection.")
        );
        assert_eq!(
            playback_switch_block_reason("FX: X-Bass", true, &controls),
            Some(UNSAFE_HARDWARE_OUTFX)
        );
    }

    #[test]
    fn blocks_custom_eq_bands_until_the_factory_preset_is_flat() {
        let mut controls = vec![selected_choice(EQUALIZER_PRESET_CONTROL, "Acoustic")];
        assert_eq!(
            equalizer_band_block_reason("EQ Band0", &controls),
            Some(EQUALIZER_BAND_EDIT_BLOCK)
        );
        assert_eq!(equalizer_band_block_reason("Front", &controls), None);

        controls[0].selected = Some("Flat".to_owned());
        assert_eq!(equalizer_band_block_reason("EQ Band9", &controls), None);
    }

    #[test]
    fn exposes_the_smart_volume_level_only_in_normal_mode() {
        let mut controls = vec![selected_choice("FX: Smart Volume Setting", "Normal")];
        assert_eq!(
            smart_volume_level_block_reason("FX: Smart Volume", &controls),
            None
        );

        controls[0].selected = Some("Loud".to_owned());
        assert_eq!(
            smart_volume_level_block_reason("FX: Smart Volume", &controls),
            Some(FIXED_SMART_VOLUME_LEVEL)
        );

        controls[0].selected = Some("Night".to_owned());
        assert_eq!(
            smart_volume_level_block_reason("FX: Smart Volume", &controls),
            Some(FIXED_SMART_VOLUME_LEVEL)
        );
        assert_eq!(
            smart_volume_level_block_reason("FX: Crystalizer", &controls),
            None
        );
        assert_eq!(
            smart_volume_level_block_reason("FX: Smart Volume", &[]),
            None
        );
    }

    #[test]
    fn marks_only_dsp_playback_controls_unavailable_in_direct_mode() {
        let controls = vec![
            playback_switch(DIRECT_MODE_CONTROL, true),
            playback_switch("FX: Surround", false),
            playback_switch("FX: X-Bass", true),
            playback_switch("Bass Redirection", false),
            capture_switch("FX: Noise Reduction", true),
            selected_choice("Surround Channel Config", "2.0"),
            selected_choice("Output Select", "Headphone"),
        ];

        assert_eq!(
            direct_mode_block_reason("FX: Surround", &controls),
            Some(DIRECT_MODE_DSP_BLOCK)
        );
        assert_eq!(
            playback_switch_block_reason("FX: Surround", true, &controls),
            Some(UNSAFE_HARDWARE_OUTFX)
        );
        assert_eq!(
            playback_switch_block_reason("FX: Surround", false, &controls),
            Some(UNSAFE_HARDWARE_OUTFX)
        );
        assert_eq!(
            playback_switch_block_reason("FX: X-Bass", true, &controls),
            Some(UNSAFE_HARDWARE_OUTFX)
        );
        assert_eq!(invalid_bass_state_reason(&controls), None);
        assert_eq!(
            equalizer_band_block_reason("EQ Band0", &controls),
            Some(DIRECT_MODE_DSP_BLOCK)
        );
        assert_eq!(
            direct_mode_block_reason("Surround Channel Config", &controls),
            Some(DIRECT_MODE_DSP_BLOCK)
        );
        assert_eq!(
            direct_mode_block_reason("FX: Noise Reduction", &controls),
            None
        );
        assert_eq!(direct_mode_block_reason("VoiceFX", &controls), None);
        assert_eq!(
            direct_mode_block_reason("AE-5: Sound Filter", &controls),
            None
        );
        assert_eq!(
            direct_mode_block_reason("AE-5: Headphone Gain", &controls),
            None
        );
        assert_eq!(direct_mode_block_reason("Output Select", &controls), None);
        assert_eq!(
            direct_mode_block_reason("PCM", &controls),
            Some(DIRECT_MODE_DSP_BLOCK)
        );
        assert_eq!(
            direct_mode_block_reason(DIRECT_MODE_CONTROL, &controls),
            None
        );
    }

    #[test]
    fn blocks_unsafe_ae5_route_transitions_in_both_directions() {
        for enabled in [false, true] {
            assert_eq!(
                playback_switch_block_reason("Enable OutFX", enabled, &[]),
                Some(UNSAFE_HARDWARE_OUTFX)
            );
            assert_eq!(
                playback_switch_block_reason(DIRECT_MODE_CONTROL, enabled, &[]),
                Some(UNSAFE_DIRECT_MODE)
            );
        }
        assert_eq!(
            unsafe_playback_control_block_reason("Output Select"),
            Some(UNSAFE_OUTPUT_ROUTE_TRANSITION)
        );
        assert_eq!(
            unsafe_playback_control_block_reason("Surround Channel Config"),
            Some(UNSAFE_OUTPUT_ROUTE_TRANSITION)
        );
        assert_eq!(
            unsafe_playback_control_block_reason("AE-5: Headphone Gain"),
            None
        );
    }

    #[test]
    fn blocks_only_the_ineffective_what_u_hear_capture_control() {
        assert_eq!(
            capture_control_block_reason("What U Hear"),
            Some(INEFFECTIVE_WHAT_U_HEAR_CONTROL)
        );
        assert_eq!(capture_control_block_reason("Capture"), None);
    }

    #[test]
    fn detects_a_muted_or_unverifiable_headphone_dac() {
        let mut controls = vec![
            selected_choice("Output Select", "Headphone"),
            playback_switch("Master", true),
            playback_switch("Front", true),
        ];
        assert_eq!(headphone_playback_issue(&controls), None);

        controls[2].playback_switch = Some(false);
        assert_eq!(
            headphone_playback_issue(&controls),
            Some(MUTED_HEADPHONE_PLAYBACK)
        );

        controls[2].playback_switch = Some(true);
        controls[1].playback_switch = Some(false);
        assert_eq!(
            headphone_playback_issue(&controls),
            Some(MUTED_HEADPHONE_MASTER)
        );

        controls.remove(1);
        assert_eq!(
            headphone_playback_issue(&controls),
            Some(UNVERIFIED_HEADPHONE_MASTER)
        );

        controls.pop();
        assert_eq!(
            headphone_playback_issue(&controls),
            Some(UNVERIFIED_HEADPHONE_MASTER)
        );

        controls[0].selected = Some("Speakers".to_owned());
        assert_eq!(headphone_playback_issue(&controls), None);

        controls[0].selected = Some("Headphone".to_owned());
        controls.push(playback_switch("Master", true));
        controls.push(playback_switch("Front", false));
        controls.push(playback_switch(DIRECT_MODE_CONTROL, true));
        assert_eq!(headphone_playback_issue(&controls), None);

        controls.pop();
        controls.pop();
        controls.push(playback_switch("Front", true));
        controls.pop();
        assert_eq!(
            headphone_playback_issue(&controls),
            Some(UNVERIFIED_HEADPHONE_PLAYBACK)
        );
    }

    #[test]
    fn explains_when_virtual_master_changes_are_clamped_by_front() {
        let mut controls = vec![
            playback_level("Master", 19, 0, 99),
            playback_level("Front", 19, 0, 99),
        ];
        assert_eq!(
            front_vmaster_clamp_warning(&controls),
            Some(
                "ALSA's virtual Master and Front attenuations stack: effective Front is 0/99 \
                 (19 + 19 − 99). Master changes remain at the floor through 80/99 while Front \
                 stays at 19/99."
                    .to_owned()
            )
        );

        controls[0].playback_level.as_mut().unwrap().value = 81;
        assert_eq!(front_vmaster_clamp_warning(&controls), None);
        controls[0].playback_level.as_mut().unwrap().value = 99;
        controls[1].playback_level.as_mut().unwrap().value = 0;
        assert_eq!(
            front_vmaster_clamp_warning(&controls),
            Some(
                "ALSA's virtual Master and Front attenuations stack: effective Front is 0/99 \
                 (99 + 0 − 99). Master cannot raise effective Front until the Front level changes."
                    .to_owned()
            )
        );
    }

    #[test]
    fn plans_only_the_required_route_repairs() {
        let mut controls = vec![
            selected_choice("Output Select", "Headphone"),
            selected_choice("Surround Channel Config", "2.0"),
            selected_choice("Input Source", "Microphone"),
            playback_switch("Master", true),
            playback_switch("Front", true),
        ];
        let mut state = PipeWireRouteState {
            profile_set: Some("sound-blaster-ae5.conf".to_owned()),
            soft_mixer: Some(true),
            ignore_db: Some(true),
            persistent_playback: Some(true),
            active_profile: Some("output:analog-stereo+input:analog-stereo".to_owned()),
            input_route: Some("sound-blaster-ae5-input-microphone".to_owned()),
            output_route: Some("sound-blaster-ae5-output-headphones;output-headphones".to_owned()),
        };
        assert_eq!(
            route_repair_plan(&controls, &state).unwrap(),
            RouteRepairPlan::default()
        );

        state.output_route = Some("analog-output-lineout;output-speaker".to_owned());
        state.input_route = Some("sound-blaster-ae5-input-line-in".to_owned());
        assert_eq!(
            route_repair_plan(&controls, &state).unwrap(),
            RouteRepairPlan {
                output: Some("Headphone".to_owned()),
                input: Some("Microphone".to_owned()),
                unmute_master: false,
                unmute_front: false,
            }
        );

        state.output_route =
            Some("sound-blaster-ae5-output-headphones;output-headphones".to_owned());
        state.input_route = Some("sound-blaster-ae5-input-microphone".to_owned());
        controls[4].playback_switch = Some(false);
        assert_eq!(
            route_repair_plan(&controls, &state).unwrap(),
            RouteRepairPlan {
                unmute_front: true,
                ..RouteRepairPlan::default()
            }
        );

        controls[4].playback_switch = Some(true);
        controls[3].playback_switch = Some(false);
        assert_eq!(
            route_repair_plan(&controls, &state).unwrap(),
            RouteRepairPlan {
                unmute_master: true,
                ..RouteRepairPlan::default()
            }
        );

        controls[3].playback_switch = Some(true);
        controls.push(playback_switch(DIRECT_MODE_CONTROL, true));
        assert_eq!(
            route_repair_plan(&controls, &state).unwrap(),
            RouteRepairPlan::default()
        );

        controls.pop();
        controls.pop();
        assert!(matches!(
            route_repair_plan(&controls, &state),
            Err(ControlError::Missing(name)) if name == "Front playback switch"
        ));

        controls.push(playback_switch("Front", true));
        controls.remove(3);
        assert!(matches!(
            route_repair_plan(&controls, &state),
            Err(ControlError::Missing(name)) if name == "Master playback switch"
        ));
    }
}
