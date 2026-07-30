use serde::{Deserialize, Serialize};

use crate::{
    Ae5Device, AudioFormat, ControlSnapshot, DIRECT_MODE_CONTROL, EffectsChainConfig,
    EqChainConfig, HARDWARE_OUTFX_CONTROL, HardwareEffectsConfig, PipeWireNode, PipeWireRouteState,
    RuntimeSampleRate, SoftwareEffectsOutput, SoftwareEqOutput, ae5_audio_format, ae5_output,
    ae5_route_state, effects_chain_config, eq_chain_config, hardware_effects_config,
    hardware_effects_profile_matches, hardware_outfx_lab_active, runtime_sample_rate,
    snapshot_controls, software_effects_output, software_eq_output,
    unsafe_playback_control_block_reason, validate_effects_runtime_support,
};

const READ_ONLY_REASON: &str =
    "Output and Direct Mode writes remain read-only until their checked ae5d paths are connected.";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceStatusCode {
    Ready,
    Partial,
    NoDevice,
    FirmwareMissing,
    PermissionDenied,
    DeviceBusy,
    WriteFailed,
    DaemonUnavailable,
    Connecting,
    DeviceError,
}

impl DeviceStatusCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Partial => "partial",
            Self::NoDevice => "no-device",
            Self::FirmwareMissing => "firmware-missing",
            Self::PermissionDenied => "permission-denied",
            Self::DeviceBusy => "device-busy",
            Self::WriteFailed => "write-failed",
            Self::DaemonUnavailable => "daemon-unavailable",
            Self::Connecting => "connecting",
            Self::DeviceError => "device-error",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Ready => "Connected",
            Self::Partial => "Partial capabilities",
            Self::NoDevice => "Not detected",
            Self::FirmwareMissing => "Firmware missing",
            Self::PermissionDenied => "Permission denied",
            Self::DeviceBusy => "Device busy",
            Self::WriteFailed => "Write failed",
            Self::DaemonUnavailable => "Daemon unavailable",
            Self::Connecting => "Connecting",
            Self::DeviceError => "Device error",
        }
    }

    pub fn from_code(code: &str) -> Self {
        match code {
            "ready" => Self::Ready,
            "partial" => Self::Partial,
            "no-device" => Self::NoDevice,
            "firmware-missing" => Self::FirmwareMissing,
            "permission-denied" => Self::PermissionDenied,
            "device-busy" => Self::DeviceBusy,
            "write-failed" => Self::WriteFailed,
            "daemon-unavailable" => Self::DaemonUnavailable,
            "connecting" => Self::Connecting,
            _ => Self::DeviceError,
        }
    }

    pub fn from_known_error(message: &str) -> Option<Self> {
        let normalized = message.to_ascii_lowercase();
        if normalized.contains("permission denied")
            || normalized.contains("operation not permitted")
            || normalized.contains("access denied")
            || normalized.contains("not authorized")
        {
            Some(Self::PermissionDenied)
        } else if normalized.contains("device or resource busy")
            || normalized.contains("device busy")
            || normalized.contains("resource busy")
        {
            Some(Self::DeviceBusy)
        } else if normalized.contains("firmware")
            && (normalized.contains("missing")
                || normalized.contains("not found")
                || normalized.contains("no such file")
                || normalized.contains("unavailable"))
        {
            Some(Self::FirmwareMissing)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "daemon", derive(zbus::zvariant::Type))]
pub struct DeviceOutputState {
    pub schema_version: u32,
    pub device_name: String,
    pub connected: bool,
    pub status_code: String,
    pub status_message: String,
    pub audio_format: String,
    pub audio_format_available: bool,
    pub sample_rate_policy: String,
    pub sample_rate_policy_available: bool,
    pub sample_rate_write_enabled: bool,
    pub sample_rate_write_block_reason: String,
    pub master_volume: u16,
    pub volume_available: bool,
    pub muted: bool,
    pub mute_available: bool,
    pub output: String,
    pub output_available: bool,
    pub headphone_gain: String,
    pub headphone_gain_available: bool,
    pub direct_mode: bool,
    pub direct_mode_available: bool,
    pub software_eq_state: String,
    pub software_eq_detail: String,
    pub software_eq_active: bool,
    pub eq_apply_available: bool,
    pub eq_apply_block_reason: String,
    pub software_effects_state: String,
    pub software_effects_detail: String,
    pub software_effects_active: bool,
    pub software_effects_apply_available: bool,
    pub software_effects_apply_block_reason: String,
    pub hardware_effects_state: String,
    pub hardware_effects_detail: String,
    pub hardware_effects_active: bool,
    pub effects_apply_available: bool,
    pub effects_apply_block_reason: String,
    pub hardware_write_enabled: bool,
    pub volume_write_enabled: bool,
    pub mute_write_enabled: bool,
    pub output_write_enabled: bool,
    pub headphone_gain_write_enabled: bool,
    pub headphone_gain_write_block_reason: String,
    pub direct_mode_write_enabled: bool,
    pub hardware_write_block_reason: String,
    pub output_write_block_reason: String,
    pub card_index: i32,
    pub controls_count: u32,
}

struct DeviceStateParts {
    controls: Result<Vec<ControlSnapshot>, String>,
    output_node: Result<Option<PipeWireNode>, String>,
    route_state: Result<PipeWireRouteState, String>,
    audio_format: Result<Option<AudioFormat>, String>,
    sample_rate_policy: Result<RuntimeSampleRate, String>,
    eq_config: Result<EqChainConfig, String>,
    eq_output: Result<Option<SoftwareEqOutput>, String>,
    effects_config: Result<EffectsChainConfig, String>,
    effects_output: Result<Option<SoftwareEffectsOutput>, String>,
    effects_runtime_support: Result<(), String>,
    hardware_effects_config: Result<HardwareEffectsConfig, String>,
    hardware_effects_gate: bool,
    output_write_block_reason: Option<String>,
}

impl DeviceOutputState {
    pub fn capture() -> std::io::Result<Self> {
        let device = match Ae5Device::discover() {
            Ok(Some(device)) => device,
            Ok(None) => return Ok(Self::no_device()),
            Err(error) => {
                let status = DeviceStatusCode::from_known_error(&error.to_string())
                    .unwrap_or(DeviceStatusCode::DeviceError);
                return Ok(Self::unavailable(
                    status,
                    format!("AE-5 discovery failed: {error}"),
                ));
            }
        };
        let card_index = device.card_index;
        let parts = DeviceStateParts {
            controls: snapshot_controls(card_index).map_err(|error| error.to_string()),
            output_node: ae5_output(card_index).map_err(|error| error.to_string()),
            route_state: ae5_route_state(card_index).map_err(|error| error.to_string()),
            audio_format: ae5_audio_format(card_index).map_err(|error| error.to_string()),
            sample_rate_policy: runtime_sample_rate().map_err(|error| error.to_string()),
            eq_config: eq_chain_config().map_err(|error| error.to_string()),
            eq_output: software_eq_output(card_index).map_err(|error| error.to_string()),
            effects_config: effects_chain_config().map_err(|error| error.to_string()),
            effects_output: software_effects_output(card_index).map_err(|error| error.to_string()),
            effects_runtime_support: validate_effects_runtime_support()
                .map_err(|error| error.to_string()),
            hardware_effects_config: hardware_effects_config().map_err(|error| error.to_string()),
            hardware_effects_gate: hardware_outfx_lab_active(),
            output_write_block_reason: unsafe_playback_control_block_reason("Output Select")
                .map(str::to_owned),
        };
        Ok(compose_state(device, parts))
    }

    fn no_device() -> Self {
        Self::unavailable(
            DeviceStatusCode::NoDevice,
            "No compatible Sound BlasterX AE-5 was detected.".to_owned(),
        )
    }

    fn unavailable(status: DeviceStatusCode, status_message: String) -> Self {
        Self {
            schema_version: 6,
            device_name: "Sound BlasterX AE-5".to_owned(),
            connected: false,
            status_code: status.as_str().to_owned(),
            status_message: status_message.clone(),
            audio_format: "Unavailable".to_owned(),
            audio_format_available: false,
            sample_rate_policy: "Unavailable".to_owned(),
            sample_rate_policy_available: false,
            sample_rate_write_enabled: false,
            sample_rate_write_block_reason: status_message.clone(),
            master_volume: 0,
            volume_available: false,
            muted: true,
            mute_available: false,
            output: "Unavailable".to_owned(),
            output_available: false,
            headphone_gain: "Unavailable".to_owned(),
            headphone_gain_available: false,
            direct_mode: false,
            direct_mode_available: false,
            software_eq_state: "unavailable".to_owned(),
            software_eq_detail: "No compatible AE-5 is available for software EQ.".to_owned(),
            software_eq_active: false,
            eq_apply_available: false,
            eq_apply_block_reason: status_message.clone(),
            software_effects_state: "unavailable".to_owned(),
            software_effects_detail: "No compatible AE-5 is available for software Effects."
                .to_owned(),
            software_effects_active: false,
            software_effects_apply_available: false,
            software_effects_apply_block_reason: status_message.clone(),
            hardware_effects_state: "unavailable".to_owned(),
            hardware_effects_detail: "No compatible AE-5 is available for hardware Effects."
                .to_owned(),
            hardware_effects_active: false,
            effects_apply_available: false,
            effects_apply_block_reason: status_message.clone(),
            hardware_write_enabled: false,
            volume_write_enabled: false,
            mute_write_enabled: false,
            output_write_enabled: false,
            headphone_gain_write_enabled: false,
            headphone_gain_write_block_reason: status_message.clone(),
            direct_mode_write_enabled: false,
            hardware_write_block_reason: status_message.clone(),
            output_write_block_reason: status_message,
            card_index: -1,
            controls_count: 0,
        }
    }
}

fn compose_state(device: Ae5Device, parts: DeviceStateParts) -> DeviceOutputState {
    let mut issues = Vec::new();
    let mut specific_status = None;
    let controls = match parts.controls {
        Ok(controls) => controls,
        Err(error) => {
            specific_status = DeviceStatusCode::from_known_error(&error);
            issues.push(format!("ALSA controls unavailable: {error}."));
            Vec::new()
        }
    };
    let controls_count = u32::try_from(controls.len()).unwrap_or(u32::MAX);

    let output = selected_choice(&controls, "Output Select")
        .map(normalize_output)
        .unwrap_or_else(|| {
            issues.push("Output selection is unavailable.".to_owned());
            "Unavailable".to_owned()
        });
    let output_available = output != "Unavailable";

    let headphone_gain = selected_choice(&controls, "AE-5: Headphone Gain")
        .map(short_choice)
        .unwrap_or_else(|| {
            issues.push("Headphone gain is unavailable.".to_owned());
            "Unavailable".to_owned()
        });
    let headphone_gain_available = headphone_gain != "Unavailable";

    let direct_mode_control = controls
        .iter()
        .find(|control| control.name == DIRECT_MODE_CONTROL);
    let direct_mode_available =
        direct_mode_control.is_some_and(|control| control.playback_switch.is_some());
    let direct_mode = direct_mode_control
        .and_then(|control| control.playback_switch)
        .unwrap_or(false);
    let output_present = matches!(&parts.output_node, Ok(Some(_)));
    let headphone_route_matches = parts.route_state.as_ref().is_ok_and(|state| {
        state.output_route.as_deref() == Some("sound-blaster-ae5-output-headphones")
    });
    let headphone_gain_write_block_reason = headphone_gain_write_block_reason(
        &output,
        headphone_gain_available,
        output_present,
        headphone_route_matches,
    );
    let headphone_gain_write_enabled = headphone_gain_write_block_reason.is_empty();
    let software_eq = software_eq_summary(
        &parts.eq_config,
        &parts.eq_output,
        output_present,
        direct_mode,
    );
    let hardware_effects = hardware_effects_summary(
        &parts.hardware_effects_config,
        &controls,
        parts.hardware_effects_gate,
        output_present,
        direct_mode,
        matches!(&parts.effects_output, Ok(Some(_))),
    );
    let software_effects = software_effects_summary(
        &parts.effects_config,
        &parts.effects_output,
        &parts.effects_runtime_support,
        output_present,
        direct_mode,
        hardware_effects.active,
    );

    let (master_volume, volume_available, muted, mute_available) = match parts.output_node {
        Ok(Some(node)) => {
            if node.volume_percent.is_none() {
                issues.push("PipeWire volume is unavailable.".to_owned());
            }
            if node.muted.is_none() {
                issues.push("PipeWire mute state is unavailable.".to_owned());
            }
            (
                node.volume_percent.unwrap_or(0),
                node.volume_percent.is_some(),
                node.muted.unwrap_or(true),
                node.muted.is_some(),
            )
        }
        Ok(None) => {
            issues.push("PipeWire playback output is unavailable.".to_owned());
            (0, false, true, false)
        }
        Err(error) => {
            specific_status =
                specific_status.or_else(|| DeviceStatusCode::from_known_error(&error));
            issues.push(format!("PipeWire playback state unavailable: {error}."));
            (0, false, true, false)
        }
    };

    let (audio_format, audio_format_available) = match parts.audio_format {
        Ok(Some(format)) => (format_audio_format(&format), true),
        Ok(None) => ("Idle".to_owned(), false),
        Err(error) => {
            specific_status =
                specific_status.or_else(|| DeviceStatusCode::from_known_error(&error));
            issues.push(format!("Active PipeWire format unavailable: {error}."));
            ("Unavailable".to_owned(), false)
        }
    };
    let (sample_rate_policy, sample_rate_policy_available, sample_rate_policy_error) =
        match parts.sample_rate_policy {
            Ok(policy) => (policy.policy_name().to_owned(), true, None),
            Err(error) => {
                issues.push(format!("PipeWire sample-rate policy unavailable: {error}."));
                ("Unavailable".to_owned(), false, Some(error))
            }
        };
    let sample_rate_write_enabled = output_present && sample_rate_policy_available;
    let sample_rate_write_block_reason = if !output_present {
        "PipeWire has no AE-5 playback output for a sample-rate change.".to_owned()
    } else if let Some(error) = sample_rate_policy_error {
        format!("The live PipeWire sample-rate policy is unavailable: {error}")
    } else {
        String::new()
    };

    let status_code = if let Some(status) = specific_status {
        status
    } else if issues.is_empty() {
        DeviceStatusCode::Ready
    } else {
        DeviceStatusCode::Partial
    };
    let status_message = if issues.is_empty() {
        "Live device state from ae5d.".to_owned()
    } else {
        issues.join(" ")
    };
    let output_write_block_reason = parts
        .output_write_block_reason
        .unwrap_or_else(|| READ_ONLY_REASON.to_owned());

    DeviceOutputState {
        schema_version: 6,
        device_name: device
            .codec_name
            .unwrap_or_else(|| "Sound BlasterX AE-5".to_owned()),
        connected: true,
        status_code: status_code.as_str().to_owned(),
        status_message,
        audio_format,
        audio_format_available,
        sample_rate_policy,
        sample_rate_policy_available,
        sample_rate_write_enabled,
        sample_rate_write_block_reason,
        master_volume,
        volume_available,
        muted,
        mute_available,
        output,
        output_available,
        headphone_gain,
        headphone_gain_available,
        direct_mode,
        direct_mode_available,
        software_eq_state: software_eq.state,
        software_eq_detail: software_eq.detail,
        software_eq_active: software_eq.active,
        eq_apply_available: software_eq.apply_available,
        eq_apply_block_reason: software_eq.apply_block_reason,
        software_effects_state: software_effects.state,
        software_effects_detail: software_effects.detail,
        software_effects_active: software_effects.active,
        software_effects_apply_available: software_effects.apply_available,
        software_effects_apply_block_reason: software_effects.apply_block_reason,
        hardware_effects_state: hardware_effects.state,
        hardware_effects_detail: hardware_effects.detail,
        hardware_effects_active: hardware_effects.active,
        effects_apply_available: hardware_effects.apply_available,
        effects_apply_block_reason: hardware_effects.apply_block_reason,
        hardware_write_enabled: volume_available || mute_available || headphone_gain_write_enabled,
        volume_write_enabled: volume_available,
        mute_write_enabled: mute_available,
        output_write_enabled: false,
        headphone_gain_write_enabled,
        headphone_gain_write_block_reason,
        direct_mode_write_enabled: false,
        hardware_write_block_reason: READ_ONLY_REASON.to_owned(),
        output_write_block_reason,
        card_index: device.card_index,
        controls_count,
    }
}

struct SoftwareEqSummary {
    state: String,
    detail: String,
    active: bool,
    apply_available: bool,
    apply_block_reason: String,
}

fn software_eq_summary(
    config: &Result<EqChainConfig, String>,
    runtime: &Result<Option<SoftwareEqOutput>, String>,
    output_present: bool,
    direct_mode: bool,
) -> SoftwareEqSummary {
    let (state, detail, active) = match (config, runtime) {
        (Err(error), _) => (
            "unavailable",
            format!("Software EQ configuration is unavailable: {error}"),
            false,
        ),
        (_, Err(error)) => (
            "unavailable",
            format!("Software EQ runtime state is unavailable: {error}"),
            false,
        ),
        (Ok(config), Ok(runtime)) => {
            let active = runtime.is_some();
            let current = config.signature().as_deref()
                == runtime
                    .as_ref()
                    .and_then(|output| output.signature.as_deref());
            if !config.enabled && !active {
                (
                    "inactive",
                    "Desktop audio uses the physical AE-5 output without software EQ.".to_owned(),
                    false,
                )
            } else if config.enabled && !active {
                let processing = if config.preamp_db == 0.0 {
                    "No automatic attenuation is inserted; boosted curves can clip near full scale."
                        .to_owned()
                } else {
                    format!(
                        "Legacy {:+.2} dB preamp remains saved; apply the preset again to remove it.",
                        config.preamp_db
                    )
                };
                (
                    "configured",
                    format!(
                        "A software EQ is saved but not active in this PipeWire session. {processing}"
                    ),
                    false,
                )
            } else if config.enabled && current {
                let processing = if config.preamp_db == 0.0 {
                    "No automatic attenuation is inserted; boosted curves can clip near full scale."
                        .to_owned()
                } else {
                    format!(
                        "Legacy {:+.2} dB preamp is active; apply the preset again to remove it.",
                        config.preamp_db
                    )
                };
                (
                    "current",
                    format!("Software EQ is active in the existing AE-5 output. {processing}"),
                    true,
                )
            } else {
                (
                    "different",
                    "A different AE-5 software EQ graph is active; refresh or disable it before replacing it."
                        .to_owned(),
                    active,
                )
            }
        }
    };
    let apply_block_reason = if !output_present {
        "PipeWire has no AE-5 playback output for software EQ.".to_owned()
    } else if direct_mode {
        "Turn Direct Mode off before applying software EQ.".to_owned()
    } else if state == "unavailable" {
        detail.clone()
    } else if state == "different" {
        "A different AE-5 software EQ graph is active; disable it before applying another preset."
            .to_owned()
    } else {
        String::new()
    };
    SoftwareEqSummary {
        state: state.to_owned(),
        detail,
        active,
        apply_available: apply_block_reason.is_empty(),
        apply_block_reason,
    }
}

struct SoftwareEffectsSummary {
    state: String,
    detail: String,
    active: bool,
    apply_available: bool,
    apply_block_reason: String,
}

struct HardwareEffectsSummary {
    state: String,
    detail: String,
    active: bool,
    apply_available: bool,
    apply_block_reason: String,
}

fn hardware_effects_summary(
    config: &Result<HardwareEffectsConfig, String>,
    controls: &[ControlSnapshot],
    gate_active: bool,
    output_present: bool,
    direct_mode: bool,
    software_effects_active: bool,
) -> HardwareEffectsSummary {
    let master = controls
        .iter()
        .find(|control| control.name == HARDWARE_OUTFX_CONTROL)
        .and_then(|control| control.playback_switch);
    let active = master == Some(true);
    let (state, detail) = if master.is_none() {
        (
            "unavailable",
            "The current AE-5 driver does not expose the hardware OutFX master switch.".to_owned(),
        )
    } else if !gate_active {
        (
            "unavailable",
            "Hardware OutFX is locked. Boot the exact AE-5 OutFX lab kernel and start ae5d with explicit lab confirmation."
                .to_owned(),
        )
    } else {
        match config {
            Err(error) => (
                "unavailable",
                format!("Hardware Effects managed state is unavailable: {error}"),
            ),
            Ok(config) => match config.profile.as_ref() {
                Some(profile)
                    if hardware_effects_profile_matches(profile, controls, true) =>
                {
                    (
                        "current",
                        format!(
                            "Hardware OutFX is active and verified against the saved '{}' profile.",
                            profile.name
                        ),
                    )
                }
                Some(profile)
                    if !active
                        && hardware_effects_profile_matches(profile, controls, false) =>
                {
                    (
                        "configured",
                        format!(
                            "The saved '{}' hardware profile is intact; hardware OutFX is bypassed.",
                            profile.name
                        ),
                    )
                }
                Some(_) => (
                    "different",
                    "The live hardware Effects controls differ from the last profile applied by AE-5 Control."
                        .to_owned(),
                ),
                None if active => (
                    "different",
                    "Hardware OutFX is active but has not yet been adopted by AE-5 Control."
                        .to_owned(),
                ),
                None => (
                    "inactive",
                    "Hardware OutFX is inactive; applying a profile writes and verifies the complete hardware Effects group."
                        .to_owned(),
                ),
            },
        }
    };
    let apply_block_reason = if !output_present {
        "PipeWire has no AE-5 playback output to pause safely for a hardware Effects transaction."
            .to_owned()
    } else if direct_mode {
        "Turn Direct Mode off before applying hardware Effects.".to_owned()
    } else if software_effects_active {
        "Disable the software Effects fallback before enabling hardware OutFX; the two backends are never stacked."
            .to_owned()
    } else if state == "unavailable" {
        detail.clone()
    } else {
        String::new()
    };
    HardwareEffectsSummary {
        state: state.to_owned(),
        detail,
        active,
        apply_available: apply_block_reason.is_empty(),
        apply_block_reason,
    }
}

fn software_effects_summary(
    config: &Result<EffectsChainConfig, String>,
    runtime: &Result<Option<SoftwareEffectsOutput>, String>,
    runtime_support: &Result<(), String>,
    output_present: bool,
    direct_mode: bool,
    hardware_effects_active: bool,
) -> SoftwareEffectsSummary {
    let (state, detail, active) = match (config, runtime, runtime_support) {
        (_, _, Err(error)) => (
            "unavailable",
            format!("Software Effects are unavailable: {error}."),
            false,
        ),
        (Err(error), _, _) => (
            "unavailable",
            format!("Software Effects configuration is unavailable: {error}"),
            false,
        ),
        (_, Err(error), _) => (
            "unavailable",
            format!("Software Effects runtime state is unavailable: {error}"),
            false,
        ),
        (Ok(config), Ok(runtime), Ok(())) => {
            let active = runtime.is_some();
            let current = config.signature().as_deref()
                == runtime
                    .as_ref()
                    .and_then(|output| output.signature.as_deref());
            if !config.enabled && !active {
                (
                    "inactive",
                    "The software Effects fallback is inactive.".to_owned(),
                    false,
                )
            } else if config.enabled && !active {
                (
                    "configured",
                    "A software Effects profile is saved but not active in this PipeWire session."
                        .to_owned(),
                    false,
                )
            } else if config.enabled && current {
                (
                    "current",
                    "Linux software substitutes are active in the existing AE-5 output; the hardware OutFX path was not written."
                        .to_owned(),
                    true,
                )
            } else {
                (
                    "different",
                    "A different AE-5 software Effects graph is active; disable it before replacing it."
                        .to_owned(),
                    active,
                )
            }
        }
    };
    let apply_block_reason = if !output_present {
        "PipeWire has no AE-5 playback output for software Effects.".to_owned()
    } else if direct_mode {
        "Turn Direct Mode off before applying software Effects.".to_owned()
    } else if hardware_effects_active {
        "Disable hardware OutFX before enabling the software Effects fallback; the two backends are never stacked."
            .to_owned()
    } else if state == "unavailable" {
        detail.clone()
    } else if state == "different" {
        "A different AE-5 software Effects graph is active; disable it before applying another profile."
            .to_owned()
    } else {
        String::new()
    };
    SoftwareEffectsSummary {
        state: state.to_owned(),
        detail,
        active,
        apply_available: apply_block_reason.is_empty(),
        apply_block_reason,
    }
}

fn selected_choice<'a>(controls: &'a [ControlSnapshot], name: &str) -> Option<&'a str> {
    controls
        .iter()
        .find(|control| control.name == name)
        .and_then(|control| control.selected.as_deref())
}

fn normalize_output(output: &str) -> String {
    match output {
        "Headphone" => "Headphones".to_owned(),
        output => output.to_owned(),
    }
}

fn short_choice(choice: &str) -> String {
    choice
        .split_once(" (")
        .map_or(choice, |(short, _)| short)
        .to_owned()
}

fn headphone_gain_write_block_reason(
    output: &str,
    gain_available: bool,
    output_present: bool,
    headphone_route_matches: bool,
) -> String {
    if !gain_available {
        "The AE-5 headphone gain control is unavailable.".to_owned()
    } else if output != "Headphones" {
        "Select Headphones before changing headphone gain.".to_owned()
    } else if !output_present {
        "PipeWire has no AE-5 playback output to pause for a gain change.".to_owned()
    } else if !headphone_route_matches {
        "The PipeWire and ALSA headphone routes must match before changing gain.".to_owned()
    } else {
        String::new()
    }
}

fn format_audio_format(format: &AudioFormat) -> String {
    let rate = if format.sample_rate.is_multiple_of(1_000) {
        format!("{} kHz", format.sample_rate / 1_000)
    } else {
        format!("{} Hz", format.sample_rate)
    };
    format!("{} · {rate}", format.sample_format)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChannelLevel, EQ_FREQUENCIES, EffectsChainConfig, EffectsProfileEntry, EqBand,
        EqChainConfig, HardwareEffectsConfig, SoftwareEffectsOutput, SoftwareEqOutput,
    };

    fn device() -> Ae5Device {
        Ae5Device {
            card_index: 1,
            alsa_name: "HDA Creative".to_owned(),
            alsa_long_name: "HDA Creative at test".to_owned(),
            codec_name: Some("Creative Sound BlasterX AE-5".to_owned()),
            vendor_id: 0x1102,
            device_id: 0x0012,
            subsystem_vendor_id: 0x1102,
            subsystem_device_id: 0x0051,
        }
    }

    fn choice(name: &str, selected: &str) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: Some(selected.to_owned()),
            choices: vec![selected.to_owned()],
            playback_switch: None,
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::<ChannelLevel>::new(),
            capture_channels: Vec::<ChannelLevel>::new(),
        }
    }

    fn controls() -> Vec<ControlSnapshot> {
        let mut controls = vec![
            choice("Output Select", "Headphone"),
            choice("AE-5: Headphone Gain", "Medium (32-149 Ohms)"),
        ];
        controls.push(ControlSnapshot {
            name: DIRECT_MODE_CONTROL.to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: Some(false),
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        });
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

    fn node() -> PipeWireNode {
        PipeWireNode {
            id: 68,
            node_name: "alsa_output.ae5".to_owned(),
            description: "AE-5 Analog Stereo".to_owned(),
            is_default: true,
            volume_percent: Some(20),
            muted: Some(false),
        }
    }

    fn healthy_parts() -> DeviceStateParts {
        DeviceStateParts {
            controls: Ok(controls()),
            output_node: Ok(Some(node())),
            route_state: Ok(PipeWireRouteState {
                profile_set: Some("sound-blaster-ae5.conf".to_owned()),
                soft_mixer: Some(true),
                ignore_db: Some(true),
                persistent_playback: Some(true),
                active_profile: Some("output:analog-stereo+input:analog-stereo".to_owned()),
                input_route: Some("sound-blaster-ae5-input-microphone".to_owned()),
                output_route: Some("sound-blaster-ae5-output-headphones".to_owned()),
            }),
            audio_format: Ok(Some(AudioFormat {
                sample_format: "S16LE".to_owned(),
                sample_rate: 96_000,
            })),
            sample_rate_policy: Ok(RuntimeSampleRate::Hz96000),
            eq_config: Ok(EqChainConfig {
                path: "/tmp/ae5-test-eq".into(),
                enabled: false,
                bands: Vec::new(),
                target_node: None,
                preamp_db: 0.0,
            }),
            eq_output: Ok(None),
            effects_config: Ok(EffectsChainConfig {
                path: "/tmp/ae5-test-effects".into(),
                enabled: false,
                target_node: None,
                profile: None,
            }),
            effects_output: Ok(None),
            effects_runtime_support: Ok(()),
            hardware_effects_config: Ok(HardwareEffectsConfig {
                path: "/tmp/ae5-test-hardware-effects".into(),
                profile: None,
            }),
            hardware_effects_gate: true,
            output_write_block_reason: None,
        }
    }

    #[test]
    fn no_device_state_disables_every_hardware_value() {
        let state = DeviceOutputState::no_device();
        assert_eq!(
            (
                state.connected,
                state.status_code.as_str(),
                state.output_available,
                state.volume_available,
                state.headphone_gain_available,
            ),
            (false, "no-device", false, false, false)
        );
    }

    #[test]
    fn compose_state_maps_live_hardware_values() {
        let state = compose_state(device(), healthy_parts());
        assert_eq!(
            (
                state.status_code.as_str(),
                state.output.as_str(),
                state.headphone_gain.as_str(),
                state.master_volume,
                state.muted,
                state.audio_format.as_str(),
                state.sample_rate_policy.as_str(),
                state.sample_rate_write_enabled,
                state.volume_write_enabled,
                state.mute_write_enabled,
                state.software_eq_state.as_str(),
                state.eq_apply_available,
            ),
            (
                "ready",
                "Headphones",
                "Medium",
                20,
                false,
                "S16LE · 96 kHz",
                "96 kHz",
                true,
                true,
                true,
                "inactive",
                true,
            )
        );
        assert_eq!(
            (
                state.software_effects_state.as_str(),
                state.software_effects_apply_available,
                state.hardware_effects_state.as_str(),
                state.effects_apply_available,
            ),
            ("inactive", true, "inactive", true)
        );
    }

    #[test]
    fn headphone_gain_write_is_available_only_on_the_live_headphone_path() {
        assert_eq!(
            headphone_gain_write_block_reason("Headphones", true, true, true),
            ""
        );
    }

    #[test]
    fn headphone_gain_write_explains_a_non_headphone_output() {
        assert_eq!(
            headphone_gain_write_block_reason("Speakers", true, true, false),
            "Select Headphones before changing headphone gain."
        );
    }

    #[test]
    fn headphone_gain_write_requires_a_live_pipewire_output() {
        assert_eq!(
            headphone_gain_write_block_reason("Headphones", true, false, false),
            "PipeWire has no AE-5 playback output to pause for a gain change."
        );
    }

    #[test]
    fn headphone_gain_write_requires_matching_pipewire_and_alsa_routes() {
        assert_eq!(
            headphone_gain_write_block_reason("Headphones", true, true, false),
            "The PipeWire and ALSA headphone routes must match before changing gain."
        );
    }

    #[test]
    fn compose_state_allows_software_eq_while_outfx_is_active() {
        let mut parts = healthy_parts();
        parts
            .controls
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|control| control.name == "Enable OutFX")
            .unwrap()
            .playback_switch = Some(true);

        let state = compose_state(device(), parts);

        assert_eq!(
            (
                state.eq_apply_available,
                state.eq_apply_block_reason.as_str(),
            ),
            (true, "")
        );
    }

    #[test]
    fn compose_state_reports_the_verified_runtime_equalizer() {
        let mut parts = healthy_parts();
        let bands = EQ_FREQUENCIES
            .map(|frequency| EqBand {
                frequency,
                q: 1.4,
                gain_db: 0.0,
            })
            .to_vec();
        let config = EqChainConfig {
            path: "/tmp/ae5-test-eq".into(),
            enabled: true,
            bands,
            target_node: Some(node().node_name),
            preamp_db: 0.0,
        };
        parts.eq_output = Ok(Some(SoftwareEqOutput {
            node: node(),
            signature: config.signature(),
        }));
        parts.eq_config = Ok(config);

        let state = compose_state(device(), parts);

        assert_eq!(
            (
                state.software_eq_state.as_str(),
                state.software_eq_active,
                state.eq_apply_available,
            ),
            ("current", true, true)
        );
    }

    #[test]
    fn compose_state_reports_verified_hardware_effects() {
        let mut parts = healthy_parts();
        let profile = EffectsProfileEntry {
            id: "effects:hardware".to_owned(),
            name: "Hardware".to_owned(),
            source: "Test".to_owned(),
            read_only: false,
            outfx_enabled: true,
            surround_available: false,
            surround_enabled: false,
            surround_level: 0,
            crystalizer_available: false,
            crystalizer_enabled: false,
            crystalizer_level: 0,
            bass_available: false,
            bass_enabled: false,
            bass_level: 0,
            smart_volume_available: false,
            smart_volume_enabled: false,
            smart_volume_level: 0,
            smart_volume_mode: "Normal".to_owned(),
            dialog_available: false,
            dialog_enabled: false,
            dialog_level: 0,
        };
        parts
            .controls
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|control| control.name == HARDWARE_OUTFX_CONTROL)
            .unwrap()
            .playback_switch = Some(true);
        parts.hardware_effects_config = Ok(HardwareEffectsConfig {
            path: "/tmp/ae5-test-hardware-effects".into(),
            profile: Some(profile),
        });

        let state = compose_state(device(), parts);

        assert_eq!(
            (
                state.hardware_effects_state.as_str(),
                state.hardware_effects_active,
                state.effects_apply_available,
            ),
            ("current", true, true)
        );
        assert!(!state.software_effects_apply_available);
    }

    #[test]
    fn compose_state_reports_verified_software_effects_without_hardware_outfx() {
        let mut parts = healthy_parts();
        let profile = EffectsProfileEntry {
            id: "effects:test".to_owned(),
            name: "Test".to_owned(),
            source: "Test".to_owned(),
            read_only: false,
            outfx_enabled: true,
            surround_available: true,
            surround_enabled: true,
            surround_level: 35,
            crystalizer_available: false,
            crystalizer_enabled: false,
            crystalizer_level: 0,
            bass_available: false,
            bass_enabled: false,
            bass_level: 0,
            smart_volume_available: false,
            smart_volume_enabled: false,
            smart_volume_level: 0,
            smart_volume_mode: "Normal".to_owned(),
            dialog_available: false,
            dialog_enabled: false,
            dialog_level: 0,
        };
        let config = EffectsChainConfig {
            path: "/tmp/ae5-test-effects".into(),
            enabled: true,
            target_node: Some(node().node_name),
            profile: Some(profile),
        };
        parts.effects_output = Ok(Some(SoftwareEffectsOutput {
            node: node(),
            signature: config.signature(),
        }));
        parts.effects_config = Ok(config);

        let state = compose_state(device(), parts);

        assert_eq!(
            (
                state.software_effects_state.as_str(),
                state.software_effects_active,
                state.software_effects_apply_available,
            ),
            ("current", true, true)
        );
        assert!(!state.effects_apply_available);
        assert!(
            state
                .effects_apply_block_reason
                .contains("software Effects fallback")
        );
    }

    #[test]
    fn compose_state_reports_pipewire_failure_as_partial() {
        let mut parts = healthy_parts();
        parts.output_node = Err("wpctl unavailable".to_owned());
        let state = compose_state(device(), parts);
        assert_eq!(
            (
                state.connected,
                state.status_code.as_str(),
                state.volume_available,
                state.mute_available,
            ),
            (true, "partial", false, false)
        );
    }

    #[test]
    fn compose_state_treats_an_idle_pcm_as_healthy() {
        let mut parts = healthy_parts();
        parts.audio_format = Ok(None);

        let state = compose_state(device(), parts);

        assert_eq!(
            (
                state.status_code.as_str(),
                state.audio_format.as_str(),
                state.audio_format_available,
            ),
            ("ready", "Idle", false)
        );
    }

    #[test]
    fn classifies_user_actionable_device_failures() {
        assert_eq!(
            DeviceStatusCode::from_known_error("Permission denied opening hw:1"),
            Some(DeviceStatusCode::PermissionDenied)
        );
        assert_eq!(
            DeviceStatusCode::from_known_error("Device or resource busy"),
            Some(DeviceStatusCode::DeviceBusy)
        );
        assert_eq!(
            DeviceStatusCode::from_known_error("DSP firmware file is missing"),
            Some(DeviceStatusCode::FirmwareMissing)
        );
        assert_eq!(
            DeviceStatusCode::from_known_error("wpctl unavailable"),
            None
        );
    }

    #[test]
    fn compose_state_promotes_permission_failures_above_partial() {
        let mut parts = healthy_parts();
        parts.controls = Err("Permission denied opening ALSA controls".to_owned());

        let state = compose_state(device(), parts);

        assert_eq!(state.status_code, "permission-denied");
        assert!(state.connected);
        assert!(!state.output_available);
        assert!(state.status_message.contains("Permission denied"));
    }

    #[test]
    fn unavailable_state_explains_why_every_write_is_blocked() {
        let state = DeviceOutputState::unavailable(
            DeviceStatusCode::FirmwareMissing,
            "Required CA0132 firmware is missing.".to_owned(),
        );

        assert_eq!(state.status_code, "firmware-missing");
        assert_eq!(
            state.hardware_write_block_reason,
            "Required CA0132 firmware is missing."
        );
        assert_eq!(
            state.eq_apply_block_reason,
            "Required CA0132 firmware is missing."
        );
        assert!(!state.hardware_write_enabled);
    }

    #[test]
    fn compose_state_keeps_kernel_write_block_reason() {
        let mut parts = healthy_parts();
        parts.output_write_block_reason = Some("qualified kernel required".to_owned());
        let state = compose_state(device(), parts);
        assert_eq!(state.output_write_block_reason, "qualified kernel required");
        assert_eq!(state.hardware_write_block_reason, READ_ONLY_REASON);
    }
}
