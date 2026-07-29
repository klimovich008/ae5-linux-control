use serde::{Deserialize, Serialize};

use crate::{
    Ae5Device, AudioFormat, ControlSnapshot, DIRECT_MODE_CONTROL, EqChainConfig, PipeWireNode,
    SoftwareEqOutput, ae5_audio_format, ae5_output, eq_chain_config, snapshot_controls,
    software_eq_output, unsafe_playback_control_block_reason,
};

const READ_ONLY_REASON: &str =
    "Hardware controls are read-only until their checked ae5d write path is connected.";

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
    pub hardware_write_enabled: bool,
    pub volume_write_enabled: bool,
    pub mute_write_enabled: bool,
    pub output_write_enabled: bool,
    pub headphone_gain_write_enabled: bool,
    pub direct_mode_write_enabled: bool,
    pub hardware_write_block_reason: String,
    pub output_write_block_reason: String,
    pub card_index: i32,
    pub controls_count: u32,
}

struct DeviceStateParts {
    controls: Result<Vec<ControlSnapshot>, String>,
    output_node: Result<Option<PipeWireNode>, String>,
    audio_format: Result<Option<AudioFormat>, String>,
    eq_config: Result<EqChainConfig, String>,
    eq_output: Result<Option<SoftwareEqOutput>, String>,
    output_write_block_reason: Option<String>,
}

impl DeviceOutputState {
    pub fn capture() -> std::io::Result<Self> {
        let Some(device) = Ae5Device::discover()? else {
            return Ok(Self::no_device());
        };
        let card_index = device.card_index;
        let parts = DeviceStateParts {
            controls: snapshot_controls(card_index).map_err(|error| error.to_string()),
            output_node: ae5_output(card_index).map_err(|error| error.to_string()),
            audio_format: ae5_audio_format(card_index).map_err(|error| error.to_string()),
            eq_config: eq_chain_config().map_err(|error| error.to_string()),
            eq_output: software_eq_output(card_index).map_err(|error| error.to_string()),
            output_write_block_reason: unsafe_playback_control_block_reason("Output Select")
                .map(str::to_owned),
        };
        Ok(compose_state(device, parts))
    }

    fn no_device() -> Self {
        Self {
            schema_version: 2,
            device_name: "Sound BlasterX AE-5".to_owned(),
            connected: false,
            status_code: "no-device".to_owned(),
            status_message: "No compatible Sound BlasterX AE-5 was detected.".to_owned(),
            audio_format: "Unavailable".to_owned(),
            audio_format_available: false,
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
            eq_apply_block_reason: "No compatible Sound BlasterX AE-5 was detected.".to_owned(),
            hardware_write_enabled: false,
            volume_write_enabled: false,
            mute_write_enabled: false,
            output_write_enabled: false,
            headphone_gain_write_enabled: false,
            direct_mode_write_enabled: false,
            hardware_write_block_reason: READ_ONLY_REASON.to_owned(),
            output_write_block_reason: READ_ONLY_REASON.to_owned(),
            card_index: -1,
            controls_count: 0,
        }
    }
}

fn compose_state(device: Ae5Device, parts: DeviceStateParts) -> DeviceOutputState {
    let mut issues = Vec::new();
    let controls = match parts.controls {
        Ok(controls) => controls,
        Err(error) => {
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
    let software_eq = software_eq_summary(
        &controls,
        &parts.eq_config,
        &parts.eq_output,
        output_present,
        direct_mode,
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
            issues.push(format!("PipeWire playback state unavailable: {error}."));
            (0, false, true, false)
        }
    };

    let (audio_format, audio_format_available) = match parts.audio_format {
        Ok(Some(format)) => (format_audio_format(&format), true),
        Ok(None) => {
            issues.push("Active PipeWire format is unavailable.".to_owned());
            ("Unavailable".to_owned(), false)
        }
        Err(error) => {
            issues.push(format!("Active PipeWire format unavailable: {error}."));
            ("Unavailable".to_owned(), false)
        }
    };

    let status_code = if issues.is_empty() {
        "ready"
    } else {
        "partial"
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
        schema_version: 2,
        device_name: device
            .codec_name
            .unwrap_or_else(|| "Sound BlasterX AE-5".to_owned()),
        connected: true,
        status_code: status_code.to_owned(),
        status_message,
        audio_format,
        audio_format_available,
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
        hardware_write_enabled: volume_available || mute_available,
        volume_write_enabled: volume_available,
        mute_write_enabled: mute_available,
        output_write_enabled: false,
        headphone_gain_write_enabled: false,
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
    controls: &[ControlSnapshot],
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
                (
                    "configured",
                    format!(
                        "A software EQ is saved but not active in this PipeWire session · automatic preamp {:+.2} dB.",
                        config.preamp_db
                    ),
                    false,
                )
            } else if config.enabled && current {
                (
                    "current",
                    format!(
                        "Software EQ is active in the existing AE-5 output · automatic preamp {:+.2} dB.",
                        config.preamp_db
                    ),
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
    let outfx = controls
        .iter()
        .find(|control| control.name == "Enable OutFX")
        .and_then(|control| control.playback_switch);
    let apply_block_reason = if !output_present {
        "PipeWire has no AE-5 playback output for software EQ.".to_owned()
    } else if direct_mode {
        "Turn Direct Mode off before applying software EQ.".to_owned()
    } else if outfx == Some(true) {
        "Turn OutFX off before applying software EQ to avoid two processing paths.".to_owned()
    } else if outfx.is_none() {
        "The live OutFX state is unavailable; software EQ cannot be applied safely.".to_owned()
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
    use crate::{ChannelLevel, EQ_FREQUENCIES, EqBand, EqChainConfig, SoftwareEqOutput};

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
            audio_format: Ok(Some(AudioFormat {
                sample_format: "S16LE".to_owned(),
                sample_rate: 96_000,
            })),
            eq_config: Ok(EqChainConfig {
                path: "/tmp/ae5-test-eq".into(),
                enabled: false,
                bands: Vec::new(),
                target_node: None,
                preamp_db: 0.0,
            }),
            eq_output: Ok(None),
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
                true,
                true,
                "inactive",
                true,
            )
        );
    }

    #[test]
    fn compose_state_blocks_software_eq_while_outfx_is_active() {
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
            (
                false,
                "Turn OutFX off before applying software EQ to avoid two processing paths.",
            )
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
    fn compose_state_keeps_kernel_write_block_reason() {
        let mut parts = healthy_parts();
        parts.output_write_block_reason = Some("qualified kernel required".to_owned());
        let state = compose_state(device(), parts);
        assert_eq!(state.output_write_block_reason, "qualified kernel required");
        assert_eq!(state.hardware_write_block_reason, READ_ONLY_REASON);
    }
}
