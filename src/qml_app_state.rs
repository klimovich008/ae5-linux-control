#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        include!("cxx-qt-lib/qstringlist.h");
        type QString = cxx_qt_lib::QString;
        type QStringList = cxx_qt_lib::QStringList;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, device_name, cxx_name = "deviceName")]
        #[qproperty(bool, connected)]
        #[qproperty(QString, device_status, cxx_name = "deviceStatus")]
        #[qproperty(QString, status_code, cxx_name = "statusCode")]
        #[qproperty(QString, status_detail, cxx_name = "statusDetail")]
        #[qproperty(bool, write_error_active, cxx_name = "writeErrorActive")]
        #[qproperty(QString, last_write_error, cxx_name = "lastWriteError")]
        #[qproperty(i32, hardware_state_revision, cxx_name = "hardwareStateRevision")]
        #[qproperty(bool, daemon_available, cxx_name = "daemonAvailable")]
        #[qproperty(bool, hardware_backed, cxx_name = "hardwareBacked")]
        #[qproperty(bool, qa_mode, cxx_name = "qaMode")]
        #[qproperty(QString, qa_scenario, cxx_name = "qaScenario")]
        #[qproperty(bool, profile_state_live, cxx_name = "profileStateLive")]
        #[qproperty(QString, audio_format, cxx_name = "audioFormat")]
        #[qproperty(bool, audio_format_available, cxx_name = "audioFormatAvailable")]
        #[qproperty(QString, sample_rate_policy, cxx_name = "sampleRatePolicy")]
        #[qproperty(
            bool,
            sample_rate_policy_available,
            cxx_name = "sampleRatePolicyAvailable"
        )]
        #[qproperty(bool, sample_rate_write_enabled, cxx_name = "sampleRateWriteEnabled")]
        #[qproperty(
            QString,
            sample_rate_write_block_reason,
            cxx_name = "sampleRateWriteBlockReason"
        )]
        #[qproperty(
            bool,
            sample_rate_write_in_flight,
            cxx_name = "sampleRateWriteInFlight"
        )]
        #[qproperty(i32, master_volume, cxx_name = "masterVolume")]
        #[qproperty(bool, volume_available, cxx_name = "volumeAvailable")]
        #[qproperty(bool, muted)]
        #[qproperty(bool, mute_available, cxx_name = "muteAvailable")]
        #[qproperty(QString, output)]
        #[qproperty(bool, output_available, cxx_name = "outputAvailable")]
        #[qproperty(QString, headphone_gain, cxx_name = "headphoneGain")]
        #[qproperty(bool, headphone_gain_available, cxx_name = "headphoneGainAvailable")]
        #[qproperty(QString, eq_preset, cxx_name = "eqPreset")]
        #[qproperty(QString, eq_state, cxx_name = "eqState")]
        #[qproperty(QString, eq_source, cxx_name = "eqSource")]
        #[qproperty(QString, eq_detail, cxx_name = "eqDetail")]
        #[qproperty(bool, eq_read_only, cxx_name = "eqReadOnly")]
        #[qproperty(QStringList, eq_preset_names, cxx_name = "eqPresetNames")]
        #[qproperty(QStringList, eq_band_gains_tenths_db, cxx_name = "eqBandGainsTenthsDb")]
        #[qproperty(bool, eq_enabled, cxx_name = "eqEnabled")]
        #[qproperty(i32, eq_selection_revision, cxx_name = "eqSelectionRevision")]
        #[qproperty(QString, software_eq_state, cxx_name = "softwareEqState")]
        #[qproperty(QString, software_eq_detail, cxx_name = "softwareEqDetail")]
        #[qproperty(bool, software_eq_active, cxx_name = "softwareEqActive")]
        #[qproperty(bool, eq_apply_available, cxx_name = "eqApplyAvailable")]
        #[qproperty(QString, eq_apply_block_reason, cxx_name = "eqApplyBlockReason")]
        #[qproperty(QString, effects_profile, cxx_name = "effectsProfile")]
        #[qproperty(QString, effects_state, cxx_name = "effectsState")]
        #[qproperty(QString, effects_source, cxx_name = "effectsSource")]
        #[qproperty(QString, effects_detail, cxx_name = "effectsDetail")]
        #[qproperty(bool, effects_read_only, cxx_name = "effectsReadOnly")]
        #[qproperty(QStringList, effects_profile_names, cxx_name = "effectsProfileNames")]
        #[qproperty(bool, effects_outfx_enabled, cxx_name = "effectsOutfxEnabled")]
        #[qproperty(QString, software_effects_state, cxx_name = "softwareEffectsState")]
        #[qproperty(QString, software_effects_detail, cxx_name = "softwareEffectsDetail")]
        #[qproperty(bool, software_effects_active, cxx_name = "softwareEffectsActive")]
        #[qproperty(QString, hardware_effects_state, cxx_name = "hardwareEffectsState")]
        #[qproperty(QString, hardware_effects_detail, cxx_name = "hardwareEffectsDetail")]
        #[qproperty(bool, hardware_effects_active, cxx_name = "hardwareEffectsActive")]
        #[qproperty(bool, effects_apply_available, cxx_name = "effectsApplyAvailable")]
        #[qproperty(
            QString,
            effects_apply_block_reason,
            cxx_name = "effectsApplyBlockReason"
        )]
        #[qproperty(bool, surround_available, cxx_name = "surroundAvailable")]
        #[qproperty(bool, surround_enabled, cxx_name = "surroundEnabled")]
        #[qproperty(i32, surround_level, cxx_name = "surroundLevel")]
        #[qproperty(bool, crystalizer_available, cxx_name = "crystalizerAvailable")]
        #[qproperty(bool, crystalizer_enabled, cxx_name = "crystalizerEnabled")]
        #[qproperty(i32, crystalizer_level, cxx_name = "crystalizerLevel")]
        #[qproperty(bool, bass_available, cxx_name = "bassAvailable")]
        #[qproperty(bool, bass_enabled, cxx_name = "bassEnabled")]
        #[qproperty(i32, bass_level, cxx_name = "bassLevel")]
        #[qproperty(bool, smart_volume_available, cxx_name = "smartVolumeAvailable")]
        #[qproperty(bool, smart_volume_enabled, cxx_name = "smartVolumeEnabled")]
        #[qproperty(i32, smart_volume_level, cxx_name = "smartVolumeLevel")]
        #[qproperty(QString, smart_volume_mode, cxx_name = "smartVolumeMode")]
        #[qproperty(bool, dialog_available, cxx_name = "dialogAvailable")]
        #[qproperty(bool, dialog_enabled, cxx_name = "dialogEnabled")]
        #[qproperty(i32, dialog_level, cxx_name = "dialogLevel")]
        #[qproperty(i32, effects_selection_revision, cxx_name = "effectsSelectionRevision")]
        #[qproperty(QString, profile_catalog_status, cxx_name = "profileCatalogStatus")]
        #[qproperty(QString, profile_catalog_detail, cxx_name = "profileCatalogDetail")]
        #[qproperty(i32, unsaved_count, cxx_name = "unsavedCount")]
        #[qproperty(bool, direct_mode, cxx_name = "directMode")]
        #[qproperty(bool, direct_mode_available, cxx_name = "directModeAvailable")]
        #[qproperty(bool, hardware_write_enabled, cxx_name = "hardwareWriteEnabled")]
        #[qproperty(bool, volume_write_enabled, cxx_name = "volumeWriteEnabled")]
        #[qproperty(bool, mute_write_enabled, cxx_name = "muteWriteEnabled")]
        #[qproperty(bool, output_write_enabled, cxx_name = "outputWriteEnabled")]
        #[qproperty(
            bool,
            headphone_gain_write_enabled,
            cxx_name = "headphoneGainWriteEnabled"
        )]
        #[qproperty(bool, direct_mode_write_enabled, cxx_name = "directModeWriteEnabled")]
        #[qproperty(
            QString,
            hardware_write_block_reason,
            cxx_name = "hardwareWriteBlockReason"
        )]
        #[qproperty(
            QString,
            output_write_block_reason,
            cxx_name = "outputWriteBlockReason"
        )]
        #[qproperty(i32, card_index, cxx_name = "cardIndex")]
        #[qproperty(i32, controls_count, cxx_name = "controlsCount")]
        type AppState = super::AppStateRust;

        #[qinvokable]
        #[cxx_name = "refreshFromDaemon"]
        fn refresh_from_daemon(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "retryStatus"]
        fn retry_status(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "requestMasterVolume"]
        fn request_master_volume(self: Pin<&mut Self>, value: i32);

        #[qinvokable]
        #[cxx_name = "requestMuted"]
        fn request_muted(self: Pin<&mut Self>, muted: bool);

        #[qinvokable]
        #[cxx_name = "requestSampleRatePolicy"]
        fn request_sample_rate_policy(self: Pin<&mut Self>, policy: &QString);

        #[qinvokable]
        #[cxx_name = "setPreviewVolume"]
        fn set_preview_volume(self: Pin<&mut Self>, value: i32);

        #[qinvokable]
        #[cxx_name = "togglePreviewMute"]
        fn toggle_preview_mute(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "selectPreviewOutput"]
        fn select_preview_output(self: Pin<&mut Self>, output: &QString);

        #[qinvokable]
        #[cxx_name = "setPreviewDirectMode"]
        fn set_preview_direct_mode(self: Pin<&mut Self>, enabled: bool);

        #[qinvokable]
        #[cxx_name = "updateEqBand"]
        fn update_eq_band(self: Pin<&mut Self>, index: i32, gain_tenths_db: i32);

        #[qinvokable]
        #[cxx_name = "selectEqPreset"]
        fn select_eq_preset(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "updateEffectsDraft"]
        fn update_effects_draft(self: Pin<&mut Self>, control: &QString, enabled: bool, level: i32);

        #[qinvokable]
        #[cxx_name = "selectEffectsProfile"]
        fn select_effects_profile(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "revertEqDraft"]
        fn revert_eq_draft(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "revertEffectsDraft"]
        fn revert_effects_draft(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "saveEqDraft"]
        fn save_eq_draft(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "saveEqDraftAs"]
        fn save_eq_draft_as(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "applyEqDraft"]
        fn apply_eq_draft(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "disableSoftwareEq"]
        fn disable_software_eq(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "applyEffectsDraft"]
        fn apply_effects_draft(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "disableHardwareEffects"]
        fn disable_hardware_effects(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "saveEffectsDraft"]
        fn save_effects_draft(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "saveEffectsDraftAs"]
        fn save_effects_draft_as(self: Pin<&mut Self>, name: &QString);
    }

    impl cxx_qt::Threading for AppState {}
}

use core::pin::Pin;
use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QString, QStringList};

pub fn initialize() {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum UiQaScenario {
    Ready,
    NoDevice,
    Partial,
    FirmwareMissing,
    PermissionDenied,
    DeviceBusy,
    WriteFailed,
    DaemonUnavailable,
    DirectMode,
    BothModified,
}

impl UiQaScenario {
    const VALID_NAMES: &'static str = "ready, no-device, partial, firmware-missing, \
        permission-denied, device-busy, write-failed, daemon-unavailable, direct-mode, \
        both-modified";

    fn parse(name: &str) -> Option<Self> {
        match name {
            "ready" => Some(Self::Ready),
            "no-device" => Some(Self::NoDevice),
            "partial" => Some(Self::Partial),
            "firmware-missing" => Some(Self::FirmwareMissing),
            "permission-denied" => Some(Self::PermissionDenied),
            "device-busy" => Some(Self::DeviceBusy),
            "write-failed" => Some(Self::WriteFailed),
            "daemon-unavailable" => Some(Self::DaemonUnavailable),
            "direct-mode" => Some(Self::DirectMode),
            "both-modified" => Some(Self::BothModified),
            _ => None,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::NoDevice => "no-device",
            Self::Partial => "partial",
            Self::FirmwareMissing => "firmware-missing",
            Self::PermissionDenied => "permission-denied",
            Self::DeviceBusy => "device-busy",
            Self::WriteFailed => "write-failed",
            Self::DaemonUnavailable => "daemon-unavailable",
            Self::DirectMode => "direct-mode",
            Self::BothModified => "both-modified",
        }
    }
}

pub fn validate_qa_arguments() -> Result<(), String> {
    for argument in std::env::args() {
        if let Some(name) = argument.strip_prefix("--qa-state=")
            && UiQaScenario::parse(name).is_none()
        {
            return Err(format!(
                "unknown QA state '{name}'; expected one of: {}",
                UiQaScenario::VALID_NAMES
            ));
        }
    }
    Ok(())
}

fn qa_scenario_from_arguments() -> Option<UiQaScenario> {
    std::env::args().find_map(|argument| {
        argument
            .strip_prefix("--qa-state=")
            .and_then(UiQaScenario::parse)
    })
}

pub struct AppStateRust {
    device_name: QString,
    connected: bool,
    device_status: QString,
    status_code: QString,
    status_detail: QString,
    write_error_active: bool,
    last_write_error: QString,
    hardware_state_revision: i32,
    daemon_available: bool,
    hardware_backed: bool,
    qa_mode: bool,
    qa_scenario: QString,
    profile_state_live: bool,
    audio_format: QString,
    audio_format_available: bool,
    sample_rate_policy: QString,
    sample_rate_policy_available: bool,
    sample_rate_write_enabled: bool,
    sample_rate_write_block_reason: QString,
    sample_rate_write_in_flight: bool,
    master_volume: i32,
    volume_available: bool,
    muted: bool,
    mute_available: bool,
    output: QString,
    output_available: bool,
    headphone_gain: QString,
    headphone_gain_available: bool,
    eq_preset: QString,
    eq_state: QString,
    eq_source: QString,
    eq_detail: QString,
    eq_read_only: bool,
    eq_preset_names: QStringList,
    eq_band_gains_tenths_db: QStringList,
    eq_enabled: bool,
    eq_selection_revision: i32,
    software_eq_state: QString,
    software_eq_detail: QString,
    software_eq_active: bool,
    eq_apply_available: bool,
    eq_apply_block_reason: QString,
    effects_profile: QString,
    effects_state: QString,
    effects_source: QString,
    effects_detail: QString,
    effects_read_only: bool,
    effects_profile_names: QStringList,
    effects_outfx_enabled: bool,
    software_effects_state: QString,
    software_effects_detail: QString,
    software_effects_active: bool,
    hardware_effects_state: QString,
    hardware_effects_detail: QString,
    hardware_effects_active: bool,
    effects_apply_available: bool,
    effects_apply_block_reason: QString,
    surround_available: bool,
    surround_enabled: bool,
    surround_level: i32,
    crystalizer_available: bool,
    crystalizer_enabled: bool,
    crystalizer_level: i32,
    bass_available: bool,
    bass_enabled: bool,
    bass_level: i32,
    smart_volume_available: bool,
    smart_volume_enabled: bool,
    smart_volume_level: i32,
    smart_volume_mode: QString,
    dialog_available: bool,
    dialog_enabled: bool,
    dialog_level: i32,
    effects_selection_revision: i32,
    profile_catalog_status: QString,
    profile_catalog_detail: QString,
    unsaved_count: i32,
    direct_mode: bool,
    direct_mode_available: bool,
    hardware_write_enabled: bool,
    volume_write_enabled: bool,
    mute_write_enabled: bool,
    output_write_enabled: bool,
    headphone_gain_write_enabled: bool,
    direct_mode_write_enabled: bool,
    hardware_write_block_reason: QString,
    output_write_block_reason: QString,
    card_index: i32,
    controls_count: i32,
    effects_entries: Vec<crate::EffectsProfileEntry>,
    eq_entries: Vec<crate::EqPresetEntry>,
    effects_draft: Option<crate::EffectsProfileEntry>,
    eq_draft: Option<crate::EqPresetEntry>,
    effects_id: String,
    eq_id: String,
    catalog_output: String,
    catalog_warning_count: usize,
    eq_operation_in_flight: bool,
    eq_operation_generation: u64,
    effects_operation_in_flight: bool,
    effects_operation_generation: u64,
}

impl Default for AppStateRust {
    fn default() -> Self {
        let qa_fixture = qa_scenario_from_arguments();
        let mut state = Self {
            device_name: QString::from("Sound BlasterX AE-5"),
            connected: false,
            device_status: QString::from("Connecting"),
            status_code: QString::from("connecting"),
            status_detail: QString::from("Connecting to the ae5d user service…"),
            write_error_active: false,
            last_write_error: QString::default(),
            hardware_state_revision: 0,
            daemon_available: false,
            hardware_backed: false,
            qa_mode: qa_fixture.is_some(),
            qa_scenario: qa_fixture.map_or_else(QString::default, |scenario| {
                QString::from(scenario.as_str())
            }),
            profile_state_live: false,
            audio_format: QString::from("Unavailable"),
            audio_format_available: false,
            sample_rate_policy: QString::from("Unavailable"),
            sample_rate_policy_available: false,
            sample_rate_write_enabled: false,
            sample_rate_write_block_reason: QString::from(
                "ae5d is unavailable; reconnect before changing the sample rate.",
            ),
            sample_rate_write_in_flight: false,
            master_volume: 20,
            volume_available: false,
            muted: true,
            mute_available: false,
            output: QString::from("Unavailable"),
            output_available: false,
            headphone_gain: QString::from("Unavailable"),
            headphone_gain_available: false,
            eq_preset: QString::from("Loading…"),
            eq_state: QString::from("Loading"),
            eq_source: QString::default(),
            eq_detail: QString::from("Loading EQ presets from ae5d…"),
            eq_read_only: true,
            eq_preset_names: QStringList::default(),
            eq_band_gains_tenths_db: QStringList::default(),
            eq_enabled: false,
            eq_selection_revision: 0,
            software_eq_state: QString::from("unavailable"),
            software_eq_detail: QString::from(
                "Live software EQ state is unavailable until ae5d connects.",
            ),
            software_eq_active: false,
            eq_apply_available: false,
            eq_apply_block_reason: QString::from(
                "ae5d is unavailable; reconnect before applying software EQ.",
            ),
            effects_profile: QString::from("Loading…"),
            effects_state: QString::from("Loading"),
            effects_source: QString::default(),
            effects_detail: QString::from("Loading Effects profiles from ae5d…"),
            effects_read_only: true,
            effects_profile_names: QStringList::default(),
            effects_outfx_enabled: false,
            software_effects_state: QString::from("unavailable"),
            software_effects_detail: QString::from(
                "Live software Effects state is unavailable until ae5d connects.",
            ),
            software_effects_active: false,
            hardware_effects_state: QString::from("unavailable"),
            hardware_effects_detail: QString::from(
                "Live hardware Effects state is unavailable until ae5d connects.",
            ),
            hardware_effects_active: false,
            effects_apply_available: false,
            effects_apply_block_reason: QString::from(
                "ae5d is unavailable; reconnect before applying hardware Effects.",
            ),
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
            smart_volume_mode: QString::default(),
            dialog_available: false,
            dialog_enabled: false,
            dialog_level: 0,
            effects_selection_revision: 0,
            profile_catalog_status: QString::from("loading"),
            profile_catalog_detail: QString::from("Loading the profile library from ae5d…"),
            unsaved_count: 0,
            direct_mode: false,
            direct_mode_available: false,
            hardware_write_enabled: false,
            volume_write_enabled: false,
            mute_write_enabled: false,
            output_write_enabled: false,
            headphone_gain_write_enabled: false,
            direct_mode_write_enabled: false,
            hardware_write_block_reason: QString::from(
                "Output, gain, and Direct Mode writes remain read-only until their checked ae5d paths are connected.",
            ),
            output_write_block_reason: QString::from(
                "Output, gain, and Direct Mode writes remain read-only until their checked ae5d paths are connected.",
            ),
            card_index: -1,
            controls_count: 0,
            effects_entries: Vec::new(),
            eq_entries: Vec::new(),
            effects_draft: None,
            eq_draft: None,
            effects_id: String::new(),
            eq_id: String::new(),
            catalog_output: String::new(),
            catalog_warning_count: 0,
            eq_operation_in_flight: false,
            eq_operation_generation: 0,
            effects_operation_in_flight: false,
            effects_operation_generation: 0,
        };
        if let Some(scenario) = qa_fixture {
            state.apply_qa_scenario(scenario);
        }
        state
    }
}

impl AppStateRust {
    fn apply_qa_scenario(&mut self, scenario: UiQaScenario) {
        let effects = qa_effects_entry();
        let eq = qa_eq_entry();

        self.device_name = QString::from("Creative Sound BlasterX AE-5");
        self.connected = true;
        self.device_status = QString::from("Connected");
        self.status_code = QString::from("ready");
        self.status_detail = QString::from(
            "Deterministic QA preview. Hardware and session audio writes are disabled.",
        );
        self.write_error_active = false;
        self.last_write_error = QString::default();
        self.hardware_state_revision = 1;
        self.daemon_available = true;
        self.hardware_backed = false;
        self.profile_state_live = false;
        self.audio_format = QString::from("S16LE · 96 kHz");
        self.audio_format_available = true;
        self.sample_rate_policy = QString::from("96 kHz");
        self.sample_rate_policy_available = true;
        self.sample_rate_write_enabled = true;
        self.sample_rate_write_block_reason = QString::default();
        self.sample_rate_write_in_flight = false;
        self.master_volume = 20;
        self.volume_available = true;
        self.muted = false;
        self.mute_available = true;
        self.output = QString::from("Headphones");
        self.output_available = true;
        self.headphone_gain = QString::from("Medium");
        self.headphone_gain_available = true;
        self.eq_preset = QString::from(&eq.name);
        self.eq_state = QString::from("Preview");
        self.eq_source = QString::from(&eq.source);
        self.eq_detail =
            QString::from("QA preset loaded. Draft editing is local and cannot change live audio.");
        self.eq_read_only = false;
        self.eq_preset_names = qstring_list(std::iter::once(eq.name.as_str()));
        self.eq_band_gains_tenths_db = qstring_gains(&eq.gains_tenths_db);
        self.eq_enabled = true;
        self.eq_selection_revision = 1;
        self.software_eq_state = QString::from("inactive");
        self.software_eq_detail =
            QString::from("QA preview never installs or changes a PipeWire filter graph.");
        self.software_eq_active = false;
        self.eq_apply_available = false;
        self.eq_apply_block_reason =
            QString::from("Live EQ writes are disabled in deterministic QA preview.");
        self.effects_profile = QString::from(&effects.name);
        self.effects_state = QString::from("Preview");
        self.effects_source = QString::from(&effects.source);
        self.effects_detail = QString::from(
            "QA Effects profile loaded. Draft editing is local and cannot change live audio.",
        );
        self.effects_read_only = false;
        self.effects_profile_names = qstring_list(std::iter::once(effects.name.as_str()));
        self.effects_outfx_enabled = effects.outfx_enabled;
        self.software_effects_state = QString::from("inactive");
        self.software_effects_detail =
            QString::from("QA preview never installs or changes a PipeWire Effects graph.");
        self.software_effects_active = false;
        self.hardware_effects_state = QString::from("inactive");
        self.hardware_effects_detail =
            QString::from("QA preview never writes the AE-5 hardware Effects controls.");
        self.hardware_effects_active = false;
        self.effects_apply_available = false;
        self.effects_apply_block_reason =
            QString::from("Live Effects writes are disabled in deterministic QA preview.");
        self.surround_available = effects.surround_available;
        self.surround_enabled = effects.surround_enabled;
        self.surround_level = i32::from(effects.surround_level);
        self.crystalizer_available = effects.crystalizer_available;
        self.crystalizer_enabled = effects.crystalizer_enabled;
        self.crystalizer_level = i32::from(effects.crystalizer_level);
        self.bass_available = effects.bass_available;
        self.bass_enabled = effects.bass_enabled;
        self.bass_level = i32::from(effects.bass_level);
        self.smart_volume_available = effects.smart_volume_available;
        self.smart_volume_enabled = effects.smart_volume_enabled;
        self.smart_volume_level = i32::from(effects.smart_volume_level);
        self.smart_volume_mode = QString::from(&effects.smart_volume_mode);
        self.dialog_available = effects.dialog_available;
        self.dialog_enabled = effects.dialog_enabled;
        self.dialog_level = i32::from(effects.dialog_level);
        self.effects_selection_revision = 1;
        self.profile_catalog_status = QString::from("ready");
        self.profile_catalog_detail =
            QString::from("One Effects profile and one EQ preset loaded for QA.");
        self.unsaved_count = 0;
        self.direct_mode = false;
        self.direct_mode_available = true;
        self.hardware_write_enabled = true;
        self.volume_write_enabled = true;
        self.mute_write_enabled = true;
        self.output_write_enabled = true;
        self.headphone_gain_write_enabled = false;
        self.direct_mode_write_enabled = true;
        self.hardware_write_block_reason =
            QString::from("This deterministic QA preview cannot write hardware.");
        self.output_write_block_reason = QString::default();
        self.card_index = 1;
        self.controls_count = 42;
        self.effects_entries = vec![effects.clone()];
        self.eq_entries = vec![eq.clone()];
        self.effects_draft = Some(effects);
        self.eq_draft = Some(eq);
        self.effects_id = "effects:qa".to_owned();
        self.eq_id = "eq:qa".to_owned();
        self.catalog_output = "Headphones".to_owned();
        self.catalog_warning_count = 0;
        self.eq_operation_in_flight = false;

        match scenario {
            UiQaScenario::Ready => {}
            UiQaScenario::BothModified => {
                self.effects_state = QString::from("Modified");
                self.effects_detail = QString::from(
                    "Draft differs from the saved Effects profile. Use Apply Effects to change live audio.",
                );
                self.bass_level += 1;
                self.effects_draft
                    .as_mut()
                    .expect("QA Effects draft")
                    .bass_level += 1;
                self.eq_state = QString::from("Modified");
                self.eq_detail = QString::from(
                    "Draft changed locally. Live audio and the saved EQ preset are unchanged.",
                );
                self.eq_draft.as_mut().expect("QA EQ draft").gains_tenths_db[0] = 40;
                self.eq_band_gains_tenths_db =
                    qstring_gains(&self.eq_draft.as_ref().expect("QA EQ draft").gains_tenths_db);
                self.unsaved_count = 2;
            }
            UiQaScenario::DirectMode => {
                self.direct_mode = true;
                self.status_detail = QString::from(
                    "Direct Mode is active in QA preview; EQ and enhancements are bypassed.",
                );
            }
            UiQaScenario::Partial => {
                self.device_status = QString::from("Partial capabilities");
                self.status_code = QString::from("partial");
                self.status_detail = QString::from(
                    "The driver is loaded, but Direct Mode and guarded output switching are unavailable.",
                );
                self.output_write_enabled = false;
                self.direct_mode_available = false;
                self.direct_mode_write_enabled = false;
                self.output_write_block_reason = QString::from(
                    "The current kernel does not expose a verified output-write path.",
                );
            }
            UiQaScenario::WriteFailed => {
                self.device_status = QString::from("Write failed");
                self.status_code = QString::from("write-failed");
                self.status_detail = QString::from(
                    "The requested value was not confirmed. The previously verified value remains authoritative.",
                );
                self.write_error_active = true;
                self.last_write_error = self.status_detail.clone();
            }
            UiQaScenario::DeviceBusy => {
                self.device_status = QString::from("Device busy");
                self.status_code = QString::from("device-busy");
                self.status_detail = QString::from(
                    "Another process is using the AE-5 controls. Close it, then retry.",
                );
                self.disable_qa_hardware_writes();
            }
            UiQaScenario::PermissionDenied => {
                self.device_status = QString::from("Permission denied");
                self.status_code = QString::from("permission-denied");
                self.status_detail = QString::from(
                    "The AE-5 was detected, but this user cannot read its ALSA controls.",
                );
                self.output = QString::from("Unavailable");
                self.output_available = false;
                self.headphone_gain = QString::from("Unavailable");
                self.headphone_gain_available = false;
                self.disable_qa_hardware_writes();
            }
            UiQaScenario::FirmwareMissing => {
                self.device_status = QString::from("Firmware missing");
                self.status_code = QString::from("firmware-missing");
                self.status_detail = QString::from(
                    "The driver is loaded, but required CA0132 firmware is unavailable.",
                );
                self.connected = false;
                self.disable_qa_hardware_values();
            }
            UiQaScenario::NoDevice => {
                self.device_status = QString::from("Not detected");
                self.status_code = QString::from("no-device");
                self.status_detail =
                    QString::from("No compatible Sound BlasterX AE-5 was detected.");
                self.connected = false;
                self.disable_qa_hardware_values();
            }
            UiQaScenario::DaemonUnavailable => {
                self.device_status = QString::from("Daemon unavailable");
                self.status_code = QString::from("daemon-unavailable");
                self.status_detail = QString::from("The ae5d user service is not responding.");
                self.connected = false;
                self.daemon_available = false;
                self.disable_qa_hardware_values();
                self.profile_catalog_status = QString::from("unavailable");
                self.profile_catalog_detail =
                    QString::from("Profiles are unavailable until ae5d reconnects.");
                self.effects_state = QString::from("Unavailable");
                self.effects_detail =
                    QString::from("Effects profiles are unavailable until ae5d reconnects.");
                self.eq_state = QString::from("Unavailable");
                self.eq_detail = QString::from("EQ presets are unavailable until ae5d reconnects.");
            }
        }
    }

    fn disable_qa_hardware_writes(&mut self) {
        self.hardware_write_enabled = false;
        self.volume_write_enabled = false;
        self.mute_write_enabled = false;
        self.output_write_enabled = false;
        self.headphone_gain_write_enabled = false;
        self.direct_mode_write_enabled = false;
        self.sample_rate_write_enabled = false;
        let reason = self.status_detail.clone();
        self.hardware_write_block_reason = reason.clone();
        self.output_write_block_reason = reason.clone();
        self.sample_rate_write_block_reason = reason;
    }

    fn disable_qa_hardware_values(&mut self) {
        self.audio_format = QString::from("Unavailable");
        self.audio_format_available = false;
        self.sample_rate_policy = QString::from("Unavailable");
        self.sample_rate_policy_available = false;
        self.sample_rate_write_in_flight = false;
        self.volume_available = false;
        self.muted = true;
        self.mute_available = false;
        self.output = QString::from("Unavailable");
        self.output_available = false;
        self.headphone_gain = QString::from("Unavailable");
        self.headphone_gain_available = false;
        self.direct_mode = false;
        self.direct_mode_available = false;
        self.software_eq_state = QString::from("unavailable");
        self.software_eq_detail =
            QString::from("Live software EQ is unavailable in this device state.");
        self.eq_apply_available = false;
        self.eq_apply_block_reason = self.status_detail.clone();
        self.software_effects_state = QString::from("unavailable");
        self.software_effects_detail =
            QString::from("Live software Effects are unavailable in this device state.");
        self.hardware_effects_state = QString::from("unavailable");
        self.hardware_effects_detail =
            QString::from("Live hardware Effects are unavailable in this device state.");
        self.hardware_effects_active = false;
        self.effects_apply_available = false;
        self.effects_apply_block_reason = self.status_detail.clone();
        self.disable_qa_hardware_writes();
        self.card_index = -1;
        self.controls_count = 0;
    }

    fn effects_modified(&self) -> bool {
        self.effects_draft.as_ref()
            != self
                .effects_entries
                .iter()
                .find(|entry| entry.id == self.effects_id)
    }

    fn eq_modified(&self) -> bool {
        self.eq_draft.as_ref() != self.eq_entries.iter().find(|entry| entry.id == self.eq_id)
    }

    fn begin_eq_operation(&mut self) -> Option<u64> {
        if self.eq_operation_in_flight {
            return None;
        }
        self.eq_operation_generation = self.eq_operation_generation.saturating_add(1);
        self.eq_operation_in_flight = true;
        Some(self.eq_operation_generation)
    }

    fn finish_eq_operation(&mut self, generation: u64) -> bool {
        if !self.eq_operation_in_flight || self.eq_operation_generation != generation {
            return false;
        }
        self.eq_operation_in_flight = false;
        true
    }

    fn begin_effects_operation(&mut self) -> Option<u64> {
        if self.effects_operation_in_flight {
            return None;
        }
        self.effects_operation_generation = self.effects_operation_generation.saturating_add(1);
        self.effects_operation_in_flight = true;
        Some(self.effects_operation_generation)
    }

    fn finish_effects_operation(&mut self, generation: u64) -> bool {
        if !self.effects_operation_in_flight || self.effects_operation_generation != generation {
            return false;
        }
        self.effects_operation_in_flight = false;
        true
    }
}

fn qa_effects_entry() -> crate::EffectsProfileEntry {
    crate::EffectsProfileEntry {
        id: "effects:qa".to_owned(),
        name: "QA Effects".to_owned(),
        source: "Deterministic QA".to_owned(),
        read_only: false,
        outfx_enabled: false,
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

fn qa_eq_entry() -> crate::EqPresetEntry {
    crate::EqPresetEntry {
        id: "eq:qa".to_owned(),
        name: "QA Curve".to_owned(),
        source: "Deterministic QA".to_owned(),
        read_only: false,
        enabled: true,
        gains_tenths_db: vec![30, 10, 20, 0, -10, -20, -20, 0, 20, 20],
    }
}

impl qobject::AppState {
    pub fn refresh_from_daemon(mut self: Pin<&mut Self>) {
        if *self.as_ref().qa_mode() {
            return;
        }
        match crate::device_service::read_device_state() {
            Ok(state) => {
                let output_changed = self.as_ref().rust().catalog_output != state.output;
                let catalog_empty = self.as_ref().rust().effects_entries.is_empty()
                    || self.as_ref().rust().eq_entries.is_empty();
                self.as_mut().apply_device_state(&state);
                if output_changed || catalog_empty {
                    match crate::device_service::read_sound_object_catalog() {
                        Ok(catalog) => self
                            .as_mut()
                            .apply_sound_object_catalog(catalog, &state.output),
                        Err(error) => self.as_mut().set_catalog_failure(&error.to_string()),
                    }
                }
            }
            Err(error) => {
                let status = status_for_daemon_error(&error.to_string());
                self.as_mut().set_connected(false);
                self.as_mut()
                    .set_device_status(QString::from(status.display_name()));
                self.as_mut()
                    .set_status_code(QString::from(status.as_str()));
                self.as_mut().set_status_detail(QString::from(format!(
                    "Cannot read live AE-5 state from ae5d: {error}"
                )));
                self.as_mut()
                    .set_daemon_available(status != crate::DeviceStatusCode::DaemonUnavailable);
                self.as_mut().set_hardware_backed(false);
                self.as_mut().set_audio_format_available(false);
                self.as_mut()
                    .set_sample_rate_policy(QString::from("Unavailable"));
                self.as_mut().set_sample_rate_policy_available(false);
                self.as_mut().set_sample_rate_write_enabled(false);
                self.as_mut()
                    .set_sample_rate_write_block_reason(QString::from(
                        "ae5d is unavailable; reconnect before changing the sample rate.",
                    ));
                self.as_mut().set_sample_rate_write_in_flight(false);
                self.as_mut().set_volume_available(false);
                self.as_mut().set_mute_available(false);
                self.as_mut().set_output_available(false);
                self.as_mut().set_headphone_gain_available(false);
                self.as_mut().set_direct_mode_available(false);
                self.as_mut()
                    .set_software_eq_state(QString::from("unavailable"));
                self.as_mut().set_software_eq_detail(QString::from(
                    "Live software EQ state is unavailable because ae5d is not responding.",
                ));
                self.as_mut().set_software_eq_active(false);
                self.as_mut().set_eq_apply_available(false);
                self.as_mut().set_eq_apply_block_reason(QString::from(
                    "ae5d is unavailable; reconnect before applying software EQ.",
                ));
                self.as_mut()
                    .set_software_effects_state(QString::from("unavailable"));
                self.as_mut().set_software_effects_detail(QString::from(
                    "Live software Effects state is unavailable because ae5d is not responding.",
                ));
                self.as_mut().set_software_effects_active(false);
                self.as_mut()
                    .set_hardware_effects_state(QString::from("unavailable"));
                self.as_mut().set_hardware_effects_detail(QString::from(
                    "Live hardware Effects state is unavailable because ae5d is not responding.",
                ));
                self.as_mut().set_hardware_effects_active(false);
                self.as_mut().set_effects_apply_available(false);
                self.as_mut().set_effects_apply_block_reason(QString::from(
                    "ae5d is unavailable; reconnect before applying hardware Effects.",
                ));
                self.as_mut().set_hardware_write_enabled(false);
                self.as_mut().set_volume_write_enabled(false);
                self.as_mut().set_mute_write_enabled(false);
                self.as_mut().set_output_write_enabled(false);
                self.as_mut().set_headphone_gain_write_enabled(false);
                self.as_mut().set_direct_mode_write_enabled(false);
                self.as_mut().set_hardware_write_block_reason(QString::from(
                    "ae5d is unavailable; reconnect before changing hardware state.",
                ));
                self.as_mut().set_output_write_block_reason(QString::from(
                    "ae5d is unavailable; reconnect before changing the output.",
                ));
                self.as_mut().set_catalog_failure(&error.to_string());
            }
        }
    }

    pub fn retry_status(mut self: Pin<&mut Self>) {
        if *self.as_ref().qa_mode() {
            self.as_mut().clear_write_error();
            self.as_mut().set_device_status(QString::from("Connected"));
            self.as_mut().set_status_code(QString::from("ready"));
            self.as_mut().set_status_detail(QString::from(
                "Deterministic QA preview. Hardware and session audio writes are disabled.",
            ));
            self.as_mut().bump_hardware_state_revision();
            return;
        }
        self.as_mut().clear_write_error();
        self.as_mut().refresh_from_daemon();
    }

    fn clear_write_error(mut self: Pin<&mut Self>) {
        self.as_mut().set_write_error_active(false);
        self.as_mut().set_last_write_error(QString::default());
    }

    fn bump_hardware_state_revision(mut self: Pin<&mut Self>) {
        let revision = *self.as_ref().hardware_state_revision();
        self.as_mut()
            .set_hardware_state_revision(revision.saturating_add(1));
    }

    fn apply_successful_write_state(
        mut self: Pin<&mut Self>,
        state: &crate::DeviceOutputState,
        sync_hardware_controls: bool,
    ) {
        self.as_mut().clear_write_error();
        self.as_mut().apply_device_state(state);
        if sync_hardware_controls {
            self.as_mut().bump_hardware_state_revision();
        }
    }

    pub fn request_master_volume(mut self: Pin<&mut Self>, value: i32) {
        if *self.as_ref().qa_mode() {
            self.as_mut().set_preview_volume(value);
            self.as_mut().bump_hardware_state_revision();
            return;
        }
        let Ok(percent) = u16::try_from(value) else {
            self.as_mut()
                .set_write_failure("Master volume must be between 0 and 100 percent.");
            return;
        };
        match crate::device_service::write_master_volume(percent) {
            Ok(state) => self.as_mut().apply_successful_write_state(&state, true),
            Err(error) => self
                .as_mut()
                .set_write_failure(&format!("Master volume was not changed: {error}")),
        }
    }

    pub fn request_muted(mut self: Pin<&mut Self>, muted: bool) {
        if *self.as_ref().qa_mode() {
            self.as_mut().set_muted(muted);
            self.as_mut().bump_hardware_state_revision();
            return;
        }
        match crate::device_service::write_muted(muted) {
            Ok(state) => self.as_mut().apply_successful_write_state(&state, true),
            Err(error) => self
                .as_mut()
                .set_write_failure(&format!("Mute was not changed: {error}")),
        }
    }

    pub fn request_sample_rate_policy(mut self: Pin<&mut Self>, policy: &QString) {
        let policy = policy.to_string();
        let Some(requested) = crate::RuntimeSampleRate::from_policy_name(&policy) else {
            self.as_mut()
                .set_write_failure("Sample rate must be Automatic, 48 kHz, or 96 kHz.");
            return;
        };
        if *self.as_ref().qa_mode() {
            self.as_mut()
                .set_sample_rate_policy(QString::from(requested.policy_name()));
            if let Some(format) = match requested {
                crate::RuntimeSampleRate::Auto => None,
                crate::RuntimeSampleRate::Hz48000 => Some("S16LE · 48 kHz"),
                crate::RuntimeSampleRate::Hz96000 => Some("S16LE · 96 kHz"),
            } {
                self.as_mut().set_audio_format(QString::from(format));
            }
            self.as_mut().bump_hardware_state_revision();
            return;
        }
        if !*self.as_ref().sample_rate_write_enabled() {
            let reason = self.as_ref().sample_rate_write_block_reason().to_string();
            self.as_mut().set_write_failure(if reason.is_empty() {
                "The sample-rate policy is unavailable."
            } else {
                &reason
            });
            return;
        }
        if *self.as_ref().sample_rate_write_in_flight() {
            return;
        }

        self.as_mut().clear_write_error();
        self.as_mut().set_sample_rate_write_in_flight(true);
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            let result = crate::device_service::write_sample_rate_policy(&policy)
                .map_err(|error| format!("Sample rate was not changed: {error}"));
            qt_thread
                .queue(move |mut app_state| {
                    app_state.as_mut().set_sample_rate_write_in_flight(false);
                    match result {
                        Ok(state) => app_state
                            .as_mut()
                            .apply_successful_write_state(&state, true),
                        Err(detail) => app_state.as_mut().set_write_failure(&detail),
                    }
                })
                .ok();
        });
    }

    pub fn apply_eq_draft(mut self: Pin<&mut Self>) {
        if *self.as_ref().qa_mode() {
            self.as_mut()
                .set_software_eq_state(QString::from("unavailable"));
            self.as_mut().set_software_eq_detail(QString::from(
                "Deterministic QA preview cannot apply a live PipeWire graph.",
            ));
            return;
        }
        let Some(draft) = self.as_ref().rust().eq_draft.clone() else {
            self.as_mut().set_software_eq_state(QString::from("error"));
            self.as_mut()
                .set_software_eq_detail(QString::from("No EQ draft is selected."));
            return;
        };
        let Some(generation) = self.as_mut().rust_mut().begin_eq_operation() else {
            return;
        };
        self.as_mut()
            .set_software_eq_state(QString::from("applying"));
        self.as_mut().set_software_eq_detail(QString::from(
            "Applying the selected EQ draft and verifying PipeWire readback…",
        ));
        let qt_thread = self.qt_thread();
        let watchdog_thread = qt_thread.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(30));
            watchdog_thread
                .queue(move |mut app_state| {
                    if app_state
                        .as_mut()
                        .rust_mut()
                        .finish_eq_operation(generation)
                    {
                        app_state
                            .as_mut()
                            .set_software_eq_state(QString::from("error"));
                        let detail =
                            "Software EQ timed out. Live state will refresh automatically.";
                        app_state
                            .as_mut()
                            .set_software_eq_detail(QString::from(detail));
                        app_state.as_mut().present_write_failure(detail);
                    }
                })
                .ok();
        });
        std::thread::spawn(move || {
            let result = crate::device_service::apply_eq_preset(&draft).map_err(|error| {
                (
                    format!("Software EQ was not applied: {error}"),
                    crate::device_service::read_device_state().ok(),
                )
            });
            qt_thread
                .queue(move |mut app_state| {
                    if !app_state
                        .as_mut()
                        .rust_mut()
                        .finish_eq_operation(generation)
                    {
                        return;
                    }
                    match result {
                        Ok(state) => app_state
                            .as_mut()
                            .apply_successful_write_state(&state, false),
                        Err((detail, confirmed_state)) => app_state
                            .as_mut()
                            .set_async_eq_runtime_failure(&detail, confirmed_state.as_ref()),
                    }
                })
                .ok();
        });
    }

    pub fn disable_software_eq(mut self: Pin<&mut Self>) {
        if *self.as_ref().qa_mode() {
            self.as_mut()
                .set_software_eq_state(QString::from("inactive"));
            self.as_mut().set_software_eq_detail(QString::from(
                "Deterministic QA preview has no live PipeWire graph.",
            ));
            self.as_mut().set_software_eq_active(false);
            return;
        }
        let Some(generation) = self.as_mut().rust_mut().begin_eq_operation() else {
            return;
        };
        self.as_mut()
            .set_software_eq_state(QString::from("applying"));
        self.as_mut().set_software_eq_detail(QString::from(
            "Disabling the AE-5 software EQ and verifying PipeWire readback…",
        ));
        let qt_thread = self.qt_thread();
        let watchdog_thread = qt_thread.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(30));
            watchdog_thread
                .queue(move |mut app_state| {
                    if app_state
                        .as_mut()
                        .rust_mut()
                        .finish_eq_operation(generation)
                    {
                        app_state
                            .as_mut()
                            .set_software_eq_state(QString::from("error"));
                        let detail =
                            "Disabling software EQ timed out. Live state will refresh automatically.";
                        app_state
                            .as_mut()
                            .set_software_eq_detail(QString::from(detail));
                        app_state.as_mut().present_write_failure(detail);
                    }
                })
                .ok();
        });
        std::thread::spawn(move || {
            let result = crate::device_service::disable_software_eq().map_err(|error| {
                (
                    format!("Software EQ was not disabled: {error}"),
                    crate::device_service::read_device_state().ok(),
                )
            });
            qt_thread
                .queue(move |mut app_state| {
                    if !app_state
                        .as_mut()
                        .rust_mut()
                        .finish_eq_operation(generation)
                    {
                        return;
                    }
                    match result {
                        Ok(state) => app_state
                            .as_mut()
                            .apply_successful_write_state(&state, false),
                        Err((detail, confirmed_state)) => app_state
                            .as_mut()
                            .set_async_eq_runtime_failure(&detail, confirmed_state.as_ref()),
                    }
                })
                .ok();
        });
    }

    pub fn apply_effects_draft(mut self: Pin<&mut Self>) {
        if *self.as_ref().qa_mode() {
            self.as_mut()
                .set_hardware_effects_state(QString::from("unavailable"));
            self.as_mut().set_hardware_effects_detail(QString::from(
                "Deterministic QA preview cannot write live AE-5 hardware Effects.",
            ));
            return;
        }
        let Some(draft) = self.as_ref().rust().effects_draft.clone() else {
            self.as_mut()
                .set_hardware_effects_state(QString::from("error"));
            self.as_mut()
                .set_hardware_effects_detail(QString::from("No Effects draft is selected."));
            return;
        };
        if !draft.outfx_enabled {
            self.as_mut()
                .set_hardware_effects_state(QString::from("error"));
            self.as_mut().set_hardware_effects_detail(QString::from(
                "Enable the Effects master before applying this profile.",
            ));
            return;
        }
        let Some(generation) = self.as_mut().rust_mut().begin_effects_operation() else {
            return;
        };
        self.as_mut()
            .set_hardware_effects_state(QString::from("applying"));
        self.as_mut().set_hardware_effects_detail(QString::from(
            "Parking active streams, applying the complete hardware Effects profile, and verifying ALSA readback…",
        ));
        let qt_thread = self.qt_thread();
        let watchdog_thread = qt_thread.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(30));
            watchdog_thread
                .queue(move |mut app_state| {
                    if app_state
                        .as_mut()
                        .rust_mut()
                        .finish_effects_operation(generation)
                    {
                        app_state
                            .as_mut()
                            .set_hardware_effects_state(QString::from("error"));
                        let detail =
                            "Hardware Effects timed out. Live state will refresh automatically.";
                        app_state
                            .as_mut()
                            .set_hardware_effects_detail(QString::from(detail));
                        app_state.as_mut().present_write_failure(detail);
                    }
                })
                .ok();
        });
        std::thread::spawn(move || {
            let result = crate::device_service::apply_effects_profile(&draft).map_err(|error| {
                (
                    format!("Hardware Effects were not applied: {error}"),
                    crate::device_service::read_device_state().ok(),
                )
            });
            qt_thread
                .queue(move |mut app_state| {
                    if !app_state
                        .as_mut()
                        .rust_mut()
                        .finish_effects_operation(generation)
                    {
                        return;
                    }
                    match result {
                        Ok(state) => app_state
                            .as_mut()
                            .apply_successful_write_state(&state, false),
                        Err((detail, confirmed_state)) => app_state
                            .as_mut()
                            .set_async_effects_runtime_failure(&detail, confirmed_state.as_ref()),
                    }
                })
                .ok();
        });
    }

    pub fn disable_hardware_effects(mut self: Pin<&mut Self>) {
        if *self.as_ref().qa_mode() {
            self.as_mut()
                .set_hardware_effects_state(QString::from("inactive"));
            self.as_mut().set_hardware_effects_detail(QString::from(
                "Deterministic QA preview has no live hardware Effects state.",
            ));
            self.as_mut().set_hardware_effects_active(false);
            return;
        }
        let Some(generation) = self.as_mut().rust_mut().begin_effects_operation() else {
            return;
        };
        self.as_mut()
            .set_hardware_effects_state(QString::from("applying"));
        self.as_mut().set_hardware_effects_detail(QString::from(
            "Parking active streams, bypassing hardware OutFX, and verifying ALSA readback…",
        ));
        let qt_thread = self.qt_thread();
        let watchdog_thread = qt_thread.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(30));
            watchdog_thread
                .queue(move |mut app_state| {
                    if app_state
                        .as_mut()
                        .rust_mut()
                        .finish_effects_operation(generation)
                    {
                        app_state
                            .as_mut()
                            .set_hardware_effects_state(QString::from("error"));
                        let detail =
                            "Disabling hardware Effects timed out. Live state will refresh automatically.";
                        app_state
                            .as_mut()
                            .set_hardware_effects_detail(QString::from(detail));
                        app_state.as_mut().present_write_failure(detail);
                    }
                })
                .ok();
        });
        std::thread::spawn(move || {
            let result = crate::device_service::disable_hardware_effects().map_err(|error| {
                (
                    format!("Hardware Effects were not disabled: {error}"),
                    crate::device_service::read_device_state().ok(),
                )
            });
            qt_thread
                .queue(move |mut app_state| {
                    if !app_state
                        .as_mut()
                        .rust_mut()
                        .finish_effects_operation(generation)
                    {
                        return;
                    }
                    match result {
                        Ok(state) => app_state
                            .as_mut()
                            .apply_successful_write_state(&state, false),
                        Err((detail, confirmed_state)) => app_state
                            .as_mut()
                            .set_async_effects_runtime_failure(&detail, confirmed_state.as_ref()),
                    }
                })
                .ok();
        });
    }

    fn set_async_eq_runtime_failure(
        mut self: Pin<&mut Self>,
        detail: &str,
        confirmed_state: Option<&crate::DeviceOutputState>,
    ) {
        if let Some(state) = confirmed_state {
            self.as_mut().apply_device_state(state);
        }
        self.as_mut().set_software_eq_state(QString::from("error"));
        self.as_mut().set_software_eq_detail(QString::from(detail));
        self.as_mut().present_write_failure(detail);
    }

    fn set_async_effects_runtime_failure(
        mut self: Pin<&mut Self>,
        detail: &str,
        confirmed_state: Option<&crate::DeviceOutputState>,
    ) {
        if let Some(state) = confirmed_state {
            self.as_mut().apply_device_state(state);
        }
        self.as_mut()
            .set_hardware_effects_state(QString::from("error"));
        self.as_mut()
            .set_hardware_effects_detail(QString::from(detail));
        self.as_mut().present_write_failure(detail);
    }

    fn apply_device_state(mut self: Pin<&mut Self>, state: &crate::DeviceOutputState) {
        let status = crate::DeviceStatusCode::from_code(&state.status_code);
        let write_error_active = *self.as_ref().write_error_active();
        let last_write_error = self.as_ref().last_write_error().clone();
        let show_write_error = should_show_write_error(write_error_active, status);
        if write_error_active && !show_write_error {
            self.as_mut().clear_write_error();
        }
        self.as_mut()
            .set_device_name(QString::from(&state.device_name));
        self.as_mut().set_connected(state.connected);
        self.as_mut()
            .set_device_status(QString::from(if show_write_error {
                crate::DeviceStatusCode::WriteFailed.display_name()
            } else {
                status.display_name()
            }));
        self.as_mut()
            .set_status_code(QString::from(if show_write_error {
                crate::DeviceStatusCode::WriteFailed.as_str()
            } else {
                &state.status_code
            }));
        self.as_mut().set_status_detail(if show_write_error {
            last_write_error
        } else {
            QString::from(&state.status_message)
        });
        self.as_mut().set_daemon_available(true);
        self.as_mut().set_hardware_backed(true);
        self.as_mut()
            .set_audio_format(QString::from(&state.audio_format));
        self.as_mut()
            .set_audio_format_available(state.audio_format_available);
        self.as_mut()
            .set_sample_rate_policy(QString::from(&state.sample_rate_policy));
        self.as_mut()
            .set_sample_rate_policy_available(state.sample_rate_policy_available);
        self.as_mut()
            .set_sample_rate_write_enabled(state.sample_rate_write_enabled);
        self.as_mut()
            .set_sample_rate_write_block_reason(QString::from(
                &state.sample_rate_write_block_reason,
            ));
        self.as_mut()
            .set_master_volume(i32::from(state.master_volume));
        self.as_mut().set_volume_available(state.volume_available);
        self.as_mut().set_muted(state.muted);
        self.as_mut().set_mute_available(state.mute_available);
        self.as_mut().set_output(QString::from(&state.output));
        self.as_mut().set_output_available(state.output_available);
        self.as_mut()
            .set_headphone_gain(QString::from(&state.headphone_gain));
        self.as_mut()
            .set_headphone_gain_available(state.headphone_gain_available);
        if !self.as_ref().rust().eq_operation_in_flight {
            self.as_mut()
                .set_software_eq_state(QString::from(&state.software_eq_state));
            self.as_mut()
                .set_software_eq_detail(QString::from(&state.software_eq_detail));
            self.as_mut()
                .set_software_eq_active(state.software_eq_active);
        }
        self.as_mut()
            .set_eq_apply_available(state.eq_apply_available);
        self.as_mut()
            .set_eq_apply_block_reason(QString::from(&state.eq_apply_block_reason));
        self.as_mut()
            .set_software_effects_state(QString::from(&state.software_effects_state));
        self.as_mut()
            .set_software_effects_detail(QString::from(&state.software_effects_detail));
        self.as_mut()
            .set_software_effects_active(state.software_effects_active);
        if !self.as_ref().rust().effects_operation_in_flight {
            self.as_mut()
                .set_hardware_effects_state(QString::from(&state.hardware_effects_state));
            self.as_mut()
                .set_hardware_effects_detail(QString::from(&state.hardware_effects_detail));
            self.as_mut()
                .set_hardware_effects_active(state.hardware_effects_active);
        }
        self.as_mut()
            .set_effects_apply_available(state.effects_apply_available);
        self.as_mut()
            .set_effects_apply_block_reason(QString::from(&state.effects_apply_block_reason));
        self.as_mut().set_direct_mode(state.direct_mode);
        self.as_mut()
            .set_direct_mode_available(state.direct_mode_available);
        self.as_mut()
            .set_hardware_write_enabled(state.hardware_write_enabled);
        self.as_mut()
            .set_volume_write_enabled(state.volume_write_enabled);
        self.as_mut()
            .set_mute_write_enabled(state.mute_write_enabled);
        self.as_mut()
            .set_output_write_enabled(state.output_write_enabled);
        self.as_mut()
            .set_headphone_gain_write_enabled(state.headphone_gain_write_enabled);
        self.as_mut()
            .set_direct_mode_write_enabled(state.direct_mode_write_enabled);
        self.as_mut()
            .set_hardware_write_block_reason(QString::from(&state.hardware_write_block_reason));
        self.as_mut()
            .set_output_write_block_reason(QString::from(&state.output_write_block_reason));
        self.as_mut().set_card_index(state.card_index);
        self.as_mut()
            .set_controls_count(i32::try_from(state.controls_count).unwrap_or(i32::MAX));
    }

    fn apply_sound_object_catalog(
        mut self: Pin<&mut Self>,
        catalog: crate::SoundObjectCatalog,
        output: &str,
    ) {
        let current_effects_id = self.as_ref().rust().effects_id.clone();
        let current_eq_id = self.as_ref().rust().eq_id.clone();
        let effects_modified = self.as_ref().rust().effects_modified();
        let eq_modified = self.as_ref().rust().eq_modified();
        let output_changed = self.as_ref().rust().catalog_output != output;
        let warning_count = catalog.warnings.len();
        let selected_effects =
            preferred_effects_entry(&catalog.effects_profiles, &current_effects_id).cloned();
        let selected_eq = preferred_eq_entry(&catalog.eq_presets, &current_eq_id).cloned();
        let effects_names = qstring_list(
            catalog
                .effects_profiles
                .iter()
                .map(|entry| entry.name.as_str()),
        );
        let eq_names = qstring_list(catalog.eq_presets.iter().map(|entry| entry.name.as_str()));
        let detail = if catalog.warnings.is_empty() {
            format!(
                "{} Effects profiles and {} EQ presets loaded for {output}.",
                catalog.effects_profiles.len(),
                catalog.eq_presets.len()
            )
        } else {
            format!(
                "{} Effects profiles and {} EQ presets loaded for {output}; {} library warning(s).",
                catalog.effects_profiles.len(),
                catalog.eq_presets.len(),
                catalog.warnings.len()
            )
        };

        self.as_mut().set_effects_profile_names(effects_names);
        self.as_mut().set_eq_preset_names(eq_names);
        self.as_mut()
            .set_profile_catalog_status(QString::from("ready"));
        self.as_mut()
            .set_profile_catalog_detail(QString::from(detail));
        self.as_mut().set_profile_state_live(false);
        {
            let mut rust = self.as_mut().rust_mut();
            rust.effects_entries = catalog.effects_profiles;
            rust.eq_entries = catalog.eq_presets;
            rust.catalog_output = output.to_owned();
            rust.catalog_warning_count = warning_count;
        }

        let effects_still_modified = self.as_ref().rust().effects_modified();
        let eq_still_modified = self.as_ref().rust().eq_modified();

        if effects_modified && effects_still_modified {
            let selected_still_exists = self
                .as_ref()
                .rust()
                .effects_entries
                .iter()
                .any(|entry| entry.id == current_effects_id);
            if !selected_still_exists {
                self.as_mut().set_effects_read_only(true);
            }
            self.as_mut().set_effects_state(QString::from("Modified"));
            self.as_mut().set_effects_detail(QString::from(if output_changed {
                format!(
                    "Draft preserved after the live output changed to {output}. Use Save as or Revert before selecting another profile."
                )
            } else {
                "Draft preserved while the Effects library refreshed.".to_owned()
            }));
        } else if let Some(entry) = selected_effects {
            self.as_mut().apply_effects_entry(&entry);
        } else {
            self.as_mut()
                .set_effects_state(QString::from("Unavailable"));
            self.as_mut().set_effects_detail(QString::from(
                "No Effects profile is available for the current output.",
            ));
        }

        if eq_modified && eq_still_modified {
            let selected_still_exists = self
                .as_ref()
                .rust()
                .eq_entries
                .iter()
                .any(|entry| entry.id == current_eq_id);
            if !selected_still_exists {
                self.as_mut().set_eq_read_only(true);
            }
            self.as_mut().set_eq_state(QString::from("Modified"));
            self.as_mut().set_eq_detail(QString::from(if output_changed {
                format!(
                    "Draft preserved after the live output changed to {output}. Use Save as or Revert before selecting another preset."
                )
            } else {
                "Draft preserved while the EQ library refreshed.".to_owned()
            }));
        } else if let Some(entry) = selected_eq {
            self.as_mut().apply_eq_entry(&entry);
        } else {
            self.as_mut().set_eq_state(QString::from("Unavailable"));
            self.as_mut().set_eq_detail(QString::from(
                "No EQ preset is available for the current output.",
            ));
        }
        self.as_mut().refresh_unsaved_count();
    }

    fn apply_effects_entry(mut self: Pin<&mut Self>, entry: &crate::EffectsProfileEntry) {
        self.as_mut()
            .set_effects_profile(QString::from(&entry.name));
        self.as_mut().set_effects_state(QString::from("Preview"));
        self.as_mut()
            .set_effects_source(QString::from(&entry.source));
        self.as_mut().set_effects_read_only(entry.read_only);
        self.as_mut().set_effects_detail(QString::from(
            if entry.read_only && entry.source == "Factory" {
                "Factory profile loaded. Edit a draft, then use Save as to create your own profile."
            } else if entry.read_only {
                "Combined imported profile loaded. Use Save as to keep Effects independent from EQ."
            } else {
                "User Effects profile loaded. Use Apply Effects to change live audio."
            },
        ));
        self.as_mut().apply_effects_draft_values(entry);
        let revision = *self.as_ref().effects_selection_revision();
        self.as_mut()
            .set_effects_selection_revision(revision.saturating_add(1));
        self.as_mut().rust_mut().effects_id = entry.id.clone();
        self.as_mut().refresh_unsaved_count();
    }

    fn apply_effects_draft_values(mut self: Pin<&mut Self>, entry: &crate::EffectsProfileEntry) {
        self.as_mut().set_effects_outfx_enabled(entry.outfx_enabled);
        self.as_mut()
            .set_surround_available(entry.surround_available);
        self.as_mut().set_surround_enabled(entry.surround_enabled);
        self.as_mut()
            .set_surround_level(i32::from(entry.surround_level));
        self.as_mut()
            .set_crystalizer_available(entry.crystalizer_available);
        self.as_mut()
            .set_crystalizer_enabled(entry.crystalizer_enabled);
        self.as_mut()
            .set_crystalizer_level(i32::from(entry.crystalizer_level));
        self.as_mut().set_bass_available(entry.bass_available);
        self.as_mut().set_bass_enabled(entry.bass_enabled);
        self.as_mut().set_bass_level(i32::from(entry.bass_level));
        self.as_mut()
            .set_smart_volume_available(entry.smart_volume_available);
        self.as_mut()
            .set_smart_volume_enabled(entry.smart_volume_enabled);
        self.as_mut()
            .set_smart_volume_level(i32::from(entry.smart_volume_level));
        self.as_mut()
            .set_smart_volume_mode(QString::from(&entry.smart_volume_mode));
        self.as_mut().set_dialog_available(entry.dialog_available);
        self.as_mut().set_dialog_enabled(entry.dialog_enabled);
        self.as_mut()
            .set_dialog_level(i32::from(entry.dialog_level));
        self.as_mut().rust_mut().effects_draft = Some(entry.clone());
    }

    fn apply_eq_entry(mut self: Pin<&mut Self>, entry: &crate::EqPresetEntry) {
        self.as_mut().set_eq_preset(QString::from(&entry.name));
        self.as_mut().set_eq_state(QString::from("Preview"));
        self.as_mut().set_eq_source(QString::from(&entry.source));
        self.as_mut().set_eq_read_only(entry.read_only);
        self.as_mut().set_eq_detail(QString::from(
            if entry.read_only && entry.source == "Factory" {
                "Factory preset loaded. Edit a draft, then use Save as to create your own preset."
            } else if entry.read_only {
                "Combined imported profile loaded. Use Save as to keep EQ independent from Effects."
            } else {
                "User EQ preset loaded. Use Apply EQ to change live audio."
            },
        ));
        self.as_mut().apply_eq_draft_values(entry);
        let revision = *self.as_ref().eq_selection_revision();
        self.as_mut()
            .set_eq_selection_revision(revision.saturating_add(1));
        self.as_mut().rust_mut().eq_id = entry.id.clone();
        self.as_mut().refresh_unsaved_count();
    }

    fn apply_eq_draft_values(mut self: Pin<&mut Self>, entry: &crate::EqPresetEntry) {
        self.as_mut().set_eq_enabled(entry.enabled);
        self.as_mut()
            .set_eq_band_gains_tenths_db(qstring_gains(&entry.gains_tenths_db));
        self.as_mut().rust_mut().eq_draft = Some(entry.clone());
    }

    fn set_catalog_failure(mut self: Pin<&mut Self>, error: &str) {
        let effects_modified = self.as_ref().rust().effects_modified();
        let eq_modified = self.as_ref().rust().eq_modified();
        let has_effects_catalog = !self.as_ref().rust().effects_entries.is_empty();
        let has_eq_catalog = !self.as_ref().rust().eq_entries.is_empty();
        let has_cached_catalog = has_effects_catalog || has_eq_catalog;
        self.as_mut()
            .set_profile_catalog_status(QString::from(if has_cached_catalog {
                "stale"
            } else {
                "unavailable"
            }));
        self.as_mut()
            .set_profile_catalog_detail(QString::from(format!(
                "Cannot read the profile library from ae5d: {error}"
            )));
        self.as_mut().set_profile_state_live(false);
        if !has_effects_catalog && !effects_modified {
            self.as_mut()
                .set_effects_state(QString::from("Unavailable"));
            self.as_mut()
                .set_effects_detail(QString::from("Effects profiles are unavailable."));
        } else if effects_modified {
            self.as_mut().set_effects_detail(QString::from(
                "Unsaved Effects draft preserved; the profile library is currently unavailable.",
            ));
        }
        if !has_eq_catalog && !eq_modified {
            self.as_mut().set_eq_state(QString::from("Unavailable"));
            self.as_mut()
                .set_eq_detail(QString::from("EQ presets are unavailable."));
        } else if eq_modified {
            self.as_mut().set_eq_detail(QString::from(
                "Unsaved EQ draft preserved; the preset library is currently unavailable.",
            ));
        }
        self.as_mut().refresh_unsaved_count();
    }

    fn set_write_failure(mut self: Pin<&mut Self>, detail: &str) {
        if !*self.as_ref().qa_mode()
            && let Ok(state) = crate::device_service::read_device_state()
        {
            self.as_mut().apply_device_state(&state);
        }
        self.as_mut().bump_hardware_state_revision();
        self.as_mut().present_write_failure(detail);
    }

    fn present_write_failure(mut self: Pin<&mut Self>, detail: &str) {
        self.as_mut().set_write_error_active(true);
        self.as_mut().set_last_write_error(QString::from(detail));
        self.as_mut()
            .set_device_status(QString::from("Write failed"));
        self.as_mut().set_status_code(QString::from("write-failed"));
        self.as_mut().set_status_detail(QString::from(detail));
    }

    pub fn set_preview_volume(mut self: Pin<&mut Self>, value: i32) {
        if *self.as_ref().hardware_backed() {
            return;
        }
        self.as_mut().set_master_volume(value.clamp(0, 100));
    }

    pub fn toggle_preview_mute(mut self: Pin<&mut Self>) {
        if *self.as_ref().hardware_backed() {
            return;
        }
        let muted = *self.as_ref().muted();
        self.as_mut().set_muted(!muted);
    }

    pub fn select_preview_output(mut self: Pin<&mut Self>, output: &QString) {
        if *self.as_ref().hardware_backed() {
            return;
        }
        self.as_mut().set_output(output.clone());
    }

    pub fn set_preview_direct_mode(mut self: Pin<&mut Self>, enabled: bool) {
        if *self.as_ref().hardware_backed() {
            return;
        }
        self.as_mut().set_direct_mode(enabled);
    }

    pub fn update_eq_band(mut self: Pin<&mut Self>, index: i32, gain_tenths_db: i32) {
        let Some(mut draft) = self.as_ref().rust().eq_draft.clone() else {
            return;
        };
        if let Err(error) = draft.set_band_gain(index, gain_tenths_db) {
            self.as_mut()
                .set_eq_detail(QString::from(format!("EQ draft was not changed: {error}")));
            return;
        }
        self.as_mut().apply_eq_draft_values(&draft);
        self.as_mut().sync_eq_modified_state();
    }

    pub fn select_eq_preset(mut self: Pin<&mut Self>, name: &QString) {
        if self.as_ref().eq_state().to_string() == "Modified" {
            self.as_mut().set_eq_detail(QString::from(
                "Save or revert this EQ draft before selecting another preset.",
            ));
            return;
        }
        let name = name.to_string();
        let entry = self
            .as_ref()
            .rust()
            .eq_entries
            .iter()
            .find(|entry| entry.name == name)
            .cloned();
        if let Some(entry) = entry {
            self.as_mut().apply_eq_entry(&entry);
        }
    }

    pub fn update_effects_draft(
        mut self: Pin<&mut Self>,
        control: &QString,
        enabled: bool,
        level: i32,
    ) {
        let Some(mut draft) = self.as_ref().rust().effects_draft.clone() else {
            return;
        };
        if let Err(error) = draft.set_control(&control.to_string(), enabled, level) {
            self.as_mut().set_effects_detail(QString::from(format!(
                "Effects draft was not changed: {error}"
            )));
            return;
        }
        self.as_mut().apply_effects_draft_values(&draft);
        self.as_mut().sync_effects_modified_state();
    }

    pub fn select_effects_profile(mut self: Pin<&mut Self>, name: &QString) {
        if self.as_ref().effects_state().to_string() == "Modified" {
            self.as_mut().set_effects_detail(QString::from(
                "Save or revert this Effects draft before selecting another profile.",
            ));
            return;
        }
        let name = name.to_string();
        let entry = self
            .as_ref()
            .rust()
            .effects_entries
            .iter()
            .find(|entry| entry.name == name)
            .cloned();
        if let Some(entry) = entry {
            self.as_mut().apply_effects_entry(&entry);
        }
    }

    pub fn revert_eq_draft(mut self: Pin<&mut Self>) {
        let id = self.as_ref().rust().eq_id.clone();
        let entries = self.as_ref().rust().eq_entries.clone();
        let entry = entries
            .iter()
            .find(|entry| entry.id == id)
            .or_else(|| preferred_eq_entry(&entries, &id))
            .cloned();
        if let Some(entry) = entry {
            self.as_mut().apply_eq_entry(&entry);
        }
    }

    pub fn revert_effects_draft(mut self: Pin<&mut Self>) {
        let id = self.as_ref().rust().effects_id.clone();
        let entries = self.as_ref().rust().effects_entries.clone();
        let entry = entries
            .iter()
            .find(|entry| entry.id == id)
            .or_else(|| preferred_effects_entry(&entries, &id))
            .cloned();
        if let Some(entry) = entry {
            self.as_mut().apply_effects_entry(&entry);
        }
    }

    pub fn save_eq_draft(mut self: Pin<&mut Self>) {
        if *self.as_ref().eq_read_only() {
            self.as_mut().set_eq_detail(QString::from(
                "This preset is read-only. Use Save as to create an independent EQ preset.",
            ));
            return;
        }
        let Some(draft) = self.as_ref().rust().eq_draft.clone() else {
            return;
        };
        if *self.as_ref().qa_mode() {
            self.as_mut().accept_saved_eq(draft);
            return;
        }
        match crate::device_service::write_eq_preset(&draft) {
            Ok(saved) => self.as_mut().accept_saved_eq(saved),
            Err(error) => self.as_mut().set_eq_save_failure(&error.to_string()),
        }
    }

    pub fn save_eq_draft_as(mut self: Pin<&mut Self>, name: &QString) {
        let Some(draft) = self.as_ref().rust().eq_draft.clone() else {
            return;
        };
        if *self.as_ref().qa_mode() {
            let mut saved = draft;
            saved.id = format!("eq:qa:{}", name.to_string().to_lowercase());
            saved.name = name.to_string();
            saved.read_only = false;
            self.as_mut().accept_saved_eq(saved);
            return;
        }
        match crate::device_service::write_eq_preset_as(&draft, &name.to_string()) {
            Ok(saved) => self.as_mut().accept_saved_eq(saved),
            Err(error) => self.as_mut().set_eq_save_failure(&error.to_string()),
        }
    }

    pub fn save_effects_draft(mut self: Pin<&mut Self>) {
        if *self.as_ref().effects_read_only() {
            self.as_mut().set_effects_detail(QString::from(
                "This profile is read-only. Use Save as to create an independent Effects profile.",
            ));
            return;
        }
        let Some(draft) = self.as_ref().rust().effects_draft.clone() else {
            return;
        };
        if *self.as_ref().qa_mode() {
            self.as_mut().accept_saved_effects(draft);
            return;
        }
        match crate::device_service::write_effects_profile(&draft) {
            Ok(saved) => self.as_mut().accept_saved_effects(saved),
            Err(error) => self.as_mut().set_effects_save_failure(&error.to_string()),
        }
    }

    pub fn save_effects_draft_as(mut self: Pin<&mut Self>, name: &QString) {
        let Some(draft) = self.as_ref().rust().effects_draft.clone() else {
            return;
        };
        if *self.as_ref().qa_mode() {
            let mut saved = draft;
            saved.id = format!("effects:qa:{}", name.to_string().to_lowercase());
            saved.name = name.to_string();
            saved.read_only = false;
            self.as_mut().accept_saved_effects(saved);
            return;
        }
        match crate::device_service::write_effects_profile_as(&draft, &name.to_string()) {
            Ok(saved) => self.as_mut().accept_saved_effects(saved),
            Err(error) => self.as_mut().set_effects_save_failure(&error.to_string()),
        }
    }

    fn sync_eq_modified_state(mut self: Pin<&mut Self>) {
        let modified = self.as_ref().rust().eq_modified();
        self.as_mut()
            .set_eq_state(QString::from(if modified { "Modified" } else { "Preview" }));
        self.as_mut().set_eq_detail(QString::from(if modified {
            "Draft changed locally. Live audio and the saved EQ preset are unchanged."
        } else {
            "Draft matches the saved preset. Use Apply EQ to change live audio."
        }));
        self.as_mut().refresh_unsaved_count();
    }

    fn sync_effects_modified_state(mut self: Pin<&mut Self>) {
        let modified = self.as_ref().rust().effects_modified();
        self.as_mut().set_effects_state(QString::from(if modified {
            "Modified"
        } else {
            "Preview"
        }));
        self.as_mut().set_effects_detail(QString::from(if modified {
            "Draft differs from the saved Effects profile. Use Apply Effects to change live audio."
        } else {
            "Draft matches the saved profile. Use Apply Effects to change live audio."
        }));
        self.as_mut().refresh_unsaved_count();
    }

    fn refresh_unsaved_count(mut self: Pin<&mut Self>) {
        let count = {
            let this = self.as_ref();
            let rust = this.rust();
            i32::from(rust.effects_modified()) + i32::from(rust.eq_modified())
        };
        self.as_mut().set_unsaved_count(count);
    }

    fn accept_saved_eq(mut self: Pin<&mut Self>, entry: crate::EqPresetEntry) {
        {
            let mut rust = self.as_mut().rust_mut();
            if let Some(existing) = rust
                .eq_entries
                .iter_mut()
                .find(|existing| existing.id == entry.id)
            {
                *existing = entry.clone();
            } else {
                rust.eq_entries.push(entry.clone());
            }
            rust.eq_entries.sort_by_cached_key(|entry| {
                (entry.read_only, entry.name.to_lowercase(), entry.id.clone())
            });
        }
        let names = {
            let this = self.as_ref();
            qstring_list(
                this.rust()
                    .eq_entries
                    .iter()
                    .map(|entry| entry.name.as_str()),
            )
        };
        self.as_mut().set_eq_preset_names(names);
        self.as_mut().refresh_profile_catalog_detail();
        self.as_mut().apply_eq_entry(&entry);
        self.as_mut().set_eq_state(QString::from("Saved"));
        self.as_mut().set_eq_detail(QString::from(
            "EQ preset saved independently. Use Apply EQ to change live audio.",
        ));
        self.as_mut().refresh_unsaved_count();
    }

    fn accept_saved_effects(mut self: Pin<&mut Self>, entry: crate::EffectsProfileEntry) {
        {
            let mut rust = self.as_mut().rust_mut();
            if let Some(existing) = rust
                .effects_entries
                .iter_mut()
                .find(|existing| existing.id == entry.id)
            {
                *existing = entry.clone();
            } else {
                rust.effects_entries.push(entry.clone());
            }
            rust.effects_entries.sort_by_cached_key(|entry| {
                (entry.read_only, entry.name.to_lowercase(), entry.id.clone())
            });
        }
        let names = {
            let this = self.as_ref();
            qstring_list(
                this.rust()
                    .effects_entries
                    .iter()
                    .map(|entry| entry.name.as_str()),
            )
        };
        self.as_mut().set_effects_profile_names(names);
        self.as_mut().refresh_profile_catalog_detail();
        self.as_mut().apply_effects_entry(&entry);
        self.as_mut().set_effects_state(QString::from("Saved"));
        self.as_mut().set_effects_detail(QString::from(
            "Effects profile saved independently. Use Apply Effects to change live audio.",
        ));
        self.as_mut().refresh_unsaved_count();
    }

    fn set_eq_save_failure(mut self: Pin<&mut Self>, error: &str) {
        self.as_mut().set_eq_detail(QString::from(format!(
            "EQ preset was not saved; the previous file is unchanged: {error}"
        )));
    }

    fn set_effects_save_failure(mut self: Pin<&mut Self>, error: &str) {
        self.as_mut().set_effects_detail(QString::from(format!(
            "Effects profile was not saved; the previous file is unchanged: {error}"
        )));
    }

    fn refresh_profile_catalog_detail(mut self: Pin<&mut Self>) {
        let detail = {
            let this = self.as_ref();
            let rust = this.rust();
            if rust.catalog_warning_count == 0 {
                format!(
                    "{} Effects profiles and {} EQ presets loaded for {}.",
                    rust.effects_entries.len(),
                    rust.eq_entries.len(),
                    rust.catalog_output
                )
            } else {
                format!(
                    "{} Effects profiles and {} EQ presets loaded for {}; {} library warning(s).",
                    rust.effects_entries.len(),
                    rust.eq_entries.len(),
                    rust.catalog_output,
                    rust.catalog_warning_count
                )
            }
        };
        self.as_mut()
            .set_profile_catalog_detail(QString::from(detail));
    }
}

fn qstring_list<'a>(items: impl Iterator<Item = &'a str>) -> QStringList {
    items.map(QString::from).collect()
}

fn qstring_gains(gains: &[i16]) -> QStringList {
    gains
        .iter()
        .map(|gain| QString::from(gain.to_string()))
        .collect()
}

fn status_for_daemon_error(error: &str) -> crate::DeviceStatusCode {
    if let Some(status) = crate::DeviceStatusCode::from_known_error(error) {
        return status;
    }
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("serviceunknown")
        || normalized.contains("namehasnoowner")
        || normalized.contains("no reply")
        || normalized.contains("disconnected")
        || normalized.contains("failed to connect")
        || normalized.contains("session bus")
    {
        crate::DeviceStatusCode::DaemonUnavailable
    } else {
        crate::DeviceStatusCode::DeviceError
    }
}

fn should_show_write_error(active: bool, status: crate::DeviceStatusCode) -> bool {
    active
        && matches!(
            status,
            crate::DeviceStatusCode::Ready
                | crate::DeviceStatusCode::Partial
                | crate::DeviceStatusCode::WriteFailed
        )
}

fn preferred_effects_entry<'a>(
    entries: &'a [crate::EffectsProfileEntry],
    current_id: &str,
) -> Option<&'a crate::EffectsProfileEntry> {
    entries
        .iter()
        .find(|entry| entry.id == current_id)
        .or_else(|| {
            entries
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case("My profile"))
        })
        .or_else(|| entries.iter().find(|entry| !entry.read_only))
        .or_else(|| entries.first())
}

fn preferred_eq_entry<'a>(
    entries: &'a [crate::EqPresetEntry],
    current_id: &str,
) -> Option<&'a crate::EqPresetEntry> {
    entries
        .iter()
        .find(|entry| entry.id == current_id)
        .or_else(|| {
            entries
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case("SHP Last"))
        })
        .or_else(|| entries.iter().find(|entry| !entry.read_only))
        .or_else(|| entries.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qa_scenarios_cover_every_required_failure_family_without_hardware_backing() {
        let scenarios = [
            (UiQaScenario::Ready, "ready"),
            (UiQaScenario::NoDevice, "no-device"),
            (UiQaScenario::Partial, "partial"),
            (UiQaScenario::FirmwareMissing, "firmware-missing"),
            (UiQaScenario::PermissionDenied, "permission-denied"),
            (UiQaScenario::DeviceBusy, "device-busy"),
            (UiQaScenario::WriteFailed, "write-failed"),
            (UiQaScenario::DaemonUnavailable, "daemon-unavailable"),
            (UiQaScenario::DirectMode, "ready"),
            (UiQaScenario::BothModified, "ready"),
        ];

        for (scenario, expected_status) in scenarios {
            let mut state = AppStateRust::default();
            state.apply_qa_scenario(scenario);

            assert!(!state.hardware_backed, "scenario {}", scenario.as_str());
            assert_eq!(
                state.status_code.to_string(),
                expected_status,
                "scenario {}",
                scenario.as_str()
            );
            assert_eq!(state.master_volume, 20, "scenario {}", scenario.as_str());
            assert_eq!(state.effects_entries.len(), 1);
            assert_eq!(state.eq_entries.len(), 1);
        }
    }

    #[test]
    fn unavailable_qa_states_disable_every_hardware_write() {
        for scenario in [
            UiQaScenario::NoDevice,
            UiQaScenario::FirmwareMissing,
            UiQaScenario::PermissionDenied,
            UiQaScenario::DeviceBusy,
            UiQaScenario::DaemonUnavailable,
        ] {
            let mut state = AppStateRust::default();
            state.apply_qa_scenario(scenario);

            assert!(!state.hardware_write_enabled);
            assert!(!state.volume_write_enabled);
            assert!(!state.mute_write_enabled);
            assert!(!state.output_write_enabled);
            assert!(!state.headphone_gain_write_enabled);
            assert!(!state.direct_mode_write_enabled);
            assert!(!state.sample_rate_write_enabled);
        }
    }

    #[test]
    fn daemon_error_mapping_preserves_actionable_device_causes() {
        assert_eq!(
            status_for_daemon_error("org.freedesktop.DBus.Error.ServiceUnknown"),
            crate::DeviceStatusCode::DaemonUnavailable
        );
        assert_eq!(
            status_for_daemon_error("Permission denied opening hw:1"),
            crate::DeviceStatusCode::PermissionDenied
        );
        assert_eq!(
            status_for_daemon_error("Device or resource busy"),
            crate::DeviceStatusCode::DeviceBusy
        );
        assert_eq!(
            status_for_daemon_error("unexpected backend failure"),
            crate::DeviceStatusCode::DeviceError
        );
    }

    #[test]
    fn sticky_write_errors_never_hide_a_more_severe_device_failure() {
        assert!(should_show_write_error(
            true,
            crate::DeviceStatusCode::Ready
        ));
        assert!(should_show_write_error(
            true,
            crate::DeviceStatusCode::Partial
        ));
        for status in [
            crate::DeviceStatusCode::NoDevice,
            crate::DeviceStatusCode::FirmwareMissing,
            crate::DeviceStatusCode::PermissionDenied,
            crate::DeviceStatusCode::DeviceBusy,
            crate::DeviceStatusCode::DaemonUnavailable,
            crate::DeviceStatusCode::DeviceError,
        ] {
            assert!(!should_show_write_error(true, status), "{status:?}");
        }
    }

    #[test]
    fn unsaved_state_is_derived_from_draft_contents() {
        let mut state = AppStateRust::default();
        state.apply_qa_scenario(UiQaScenario::Ready);

        assert!(!state.effects_modified());
        assert!(!state.eq_modified());

        state
            .effects_draft
            .as_mut()
            .expect("QA Effects draft")
            .bass_level += 1;
        state
            .eq_draft
            .as_mut()
            .expect("QA EQ draft")
            .gains_tenths_db[0] += 1;

        assert!(state.effects_modified());
        assert!(state.eq_modified());

        state.effects_draft = state.effects_entries.first().cloned();
        state.eq_draft = state.eq_entries.first().cloned();
        assert!(!state.effects_modified());
        assert!(!state.eq_modified());
    }

    #[test]
    fn catalog_replacement_cannot_make_an_existing_draft_look_saved() {
        let mut state = AppStateRust::default();
        state.apply_qa_scenario(UiQaScenario::Ready);
        state
            .eq_draft
            .as_mut()
            .expect("QA EQ draft")
            .gains_tenths_db[4] -= 10;

        state.eq_entries.clear();

        assert!(state.eq_modified());
        assert_eq!(
            state
                .eq_draft
                .as_ref()
                .expect("preserved EQ draft")
                .gains_tenths_db[4],
            -20
        );
    }

    #[test]
    fn eq_operation_ownership_rejects_overlap_and_stale_completion() {
        let mut state = AppStateRust::default();

        let first = state.begin_eq_operation().expect("first operation");
        assert!(state.begin_eq_operation().is_none());
        assert!(!state.finish_eq_operation(first.saturating_add(1)));
        assert!(state.eq_operation_in_flight);
        assert!(state.finish_eq_operation(first));
        assert!(!state.eq_operation_in_flight);

        let second = state.begin_eq_operation().expect("second operation");
        assert!(second > first);
        assert!(!state.finish_eq_operation(first));
        assert!(state.finish_eq_operation(second));
    }

    #[test]
    fn effects_operation_ownership_rejects_overlap_and_stale_completion() {
        let mut state = AppStateRust::default();

        let first = state.begin_effects_operation().expect("first operation");
        assert!(state.begin_effects_operation().is_none());
        assert!(!state.finish_effects_operation(first.saturating_add(1)));
        assert!(state.effects_operation_in_flight);
        assert!(state.finish_effects_operation(first));
        assert!(!state.effects_operation_in_flight);

        let second = state.begin_effects_operation().expect("second operation");
        assert!(second > first);
        assert!(!state.finish_effects_operation(first));
        assert!(state.finish_effects_operation(second));
    }
}
