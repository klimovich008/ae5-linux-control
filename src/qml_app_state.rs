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
        #[qproperty(bool, daemon_available, cxx_name = "daemonAvailable")]
        #[qproperty(bool, hardware_backed, cxx_name = "hardwareBacked")]
        #[qproperty(bool, profile_state_live, cxx_name = "profileStateLive")]
        #[qproperty(QString, audio_format, cxx_name = "audioFormat")]
        #[qproperty(bool, audio_format_available, cxx_name = "audioFormatAvailable")]
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
        #[cxx_name = "requestMasterVolume"]
        fn request_master_volume(self: Pin<&mut Self>, value: i32);

        #[qinvokable]
        #[cxx_name = "requestMuted"]
        fn request_muted(self: Pin<&mut Self>, muted: bool);

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
        #[cxx_name = "saveEffectsDraft"]
        fn save_effects_draft(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "saveEffectsDraftAs"]
        fn save_effects_draft_as(self: Pin<&mut Self>, name: &QString);
    }
}

use core::pin::Pin;
use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QStringList};

pub fn initialize() {}

pub struct AppStateRust {
    device_name: QString,
    connected: bool,
    device_status: QString,
    status_code: QString,
    status_detail: QString,
    daemon_available: bool,
    hardware_backed: bool,
    profile_state_live: bool,
    audio_format: QString,
    audio_format_available: bool,
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
}

impl Default for AppStateRust {
    fn default() -> Self {
        Self {
            device_name: QString::from("Sound BlasterX AE-5"),
            connected: false,
            device_status: QString::from("Connecting"),
            status_code: QString::from("connecting"),
            status_detail: QString::from("Connecting to the ae5d user service…"),
            daemon_available: false,
            hardware_backed: false,
            profile_state_live: false,
            audio_format: QString::from("Unavailable"),
            audio_format_available: false,
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
                "Hardware controls are read-only until their checked ae5d write path is connected.",
            ),
            output_write_block_reason: QString::from(
                "Hardware controls are read-only until their checked ae5d write path is connected.",
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
        }
    }
}

impl qobject::AppState {
    pub fn refresh_from_daemon(mut self: Pin<&mut Self>) {
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
                self.as_mut().set_connected(false);
                self.as_mut()
                    .set_device_status(QString::from("Daemon unavailable"));
                self.as_mut()
                    .set_status_code(QString::from("daemon-unavailable"));
                self.as_mut().set_status_detail(QString::from(format!(
                    "Cannot read live AE-5 state from ae5d: {error}"
                )));
                self.as_mut().set_daemon_available(false);
                self.as_mut().set_hardware_backed(false);
                self.as_mut().set_audio_format_available(false);
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

    pub fn request_master_volume(mut self: Pin<&mut Self>, value: i32) {
        let Ok(percent) = u16::try_from(value) else {
            self.as_mut()
                .set_write_failure("Master volume must be between 0 and 100 percent.");
            return;
        };
        match crate::device_service::write_master_volume(percent) {
            Ok(state) => self.as_mut().apply_device_state(&state),
            Err(error) => self
                .as_mut()
                .set_write_failure(&format!("Master volume was not changed: {error}")),
        }
    }

    pub fn request_muted(mut self: Pin<&mut Self>, muted: bool) {
        match crate::device_service::write_muted(muted) {
            Ok(state) => self.as_mut().apply_device_state(&state),
            Err(error) => self
                .as_mut()
                .set_write_failure(&format!("Mute was not changed: {error}")),
        }
    }

    pub fn apply_eq_draft(mut self: Pin<&mut Self>) {
        let Some(draft) = self.as_ref().rust().eq_draft.clone() else {
            self.as_mut()
                .set_eq_runtime_failure("No EQ draft is selected.");
            return;
        };
        self.as_mut()
            .set_software_eq_state(QString::from("applying"));
        self.as_mut().set_software_eq_detail(QString::from(
            "Applying the selected EQ draft and verifying PipeWire readback…",
        ));
        match crate::device_service::apply_eq_preset(&draft) {
            Ok(state) => self.as_mut().apply_device_state(&state),
            Err(error) => self
                .as_mut()
                .set_eq_runtime_failure(&format!("Software EQ was not applied: {error}")),
        }
    }

    pub fn disable_software_eq(mut self: Pin<&mut Self>) {
        self.as_mut()
            .set_software_eq_state(QString::from("applying"));
        self.as_mut().set_software_eq_detail(QString::from(
            "Disabling the AE-5 software EQ and verifying PipeWire readback…",
        ));
        match crate::device_service::disable_software_eq() {
            Ok(state) => self.as_mut().apply_device_state(&state),
            Err(error) => self
                .as_mut()
                .set_eq_runtime_failure(&format!("Software EQ was not disabled: {error}")),
        }
    }

    fn set_eq_runtime_failure(mut self: Pin<&mut Self>, detail: &str) {
        if let Ok(state) = crate::device_service::read_device_state() {
            self.as_mut().apply_device_state(&state);
        }
        self.as_mut().set_software_eq_state(QString::from("error"));
        self.as_mut().set_software_eq_detail(QString::from(detail));
        self.as_mut().set_write_failure(detail);
    }

    fn apply_device_state(mut self: Pin<&mut Self>, state: &crate::DeviceOutputState) {
        let status = match state.status_code.as_str() {
            "ready" => "Connected",
            "partial" => "Partial capabilities",
            "no-device" => "Not detected",
            _ => "Device error",
        };
        self.as_mut()
            .set_device_name(QString::from(&state.device_name));
        self.as_mut().set_connected(state.connected);
        self.as_mut().set_device_status(QString::from(status));
        self.as_mut()
            .set_status_code(QString::from(&state.status_code));
        self.as_mut()
            .set_status_detail(QString::from(&state.status_message));
        self.as_mut().set_daemon_available(true);
        self.as_mut().set_hardware_backed(true);
        self.as_mut()
            .set_audio_format(QString::from(&state.audio_format));
        self.as_mut()
            .set_audio_format_available(state.audio_format_available);
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
        self.as_mut()
            .set_software_eq_state(QString::from(&state.software_eq_state));
        self.as_mut()
            .set_software_eq_detail(QString::from(&state.software_eq_detail));
        self.as_mut()
            .set_software_eq_active(state.software_eq_active);
        self.as_mut()
            .set_eq_apply_available(state.eq_apply_available);
        self.as_mut()
            .set_eq_apply_block_reason(QString::from(&state.eq_apply_block_reason));
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

        if let Some(entry) = selected_effects {
            self.as_mut().apply_effects_entry(&entry);
        }
        if let Some(entry) = selected_eq {
            self.as_mut().apply_eq_entry(&entry);
        }
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
                "User Effects profile loaded. Live audio is unchanged until Effects apply is connected."
            },
        ));
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
        let revision = *self.as_ref().effects_selection_revision();
        self.as_mut()
            .set_effects_selection_revision(revision.saturating_add(1));
        {
            let mut rust = self.as_mut().rust_mut();
            rust.effects_id = entry.id.clone();
            rust.effects_draft = Some(entry.clone());
        }
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
        self.as_mut().set_eq_enabled(entry.enabled);
        self.as_mut().set_eq_band_gains_tenths_db(qstring_list(
            entry
                .gains_tenths_db
                .iter()
                .map(|gain| gain.to_string())
                .collect::<Vec<_>>()
                .iter()
                .map(String::as_str),
        ));
        let revision = *self.as_ref().eq_selection_revision();
        self.as_mut()
            .set_eq_selection_revision(revision.saturating_add(1));
        {
            let mut rust = self.as_mut().rust_mut();
            rust.eq_id = entry.id.clone();
            rust.eq_draft = Some(entry.clone());
        }
    }

    fn set_catalog_failure(mut self: Pin<&mut Self>, error: &str) {
        let has_catalog = !self.as_ref().rust().effects_entries.is_empty()
            && !self.as_ref().rust().eq_entries.is_empty();
        self.as_mut()
            .set_profile_catalog_status(QString::from(if has_catalog {
                "stale"
            } else {
                "unavailable"
            }));
        self.as_mut()
            .set_profile_catalog_detail(QString::from(format!(
                "Cannot read the profile library from ae5d: {error}"
            )));
        self.as_mut().set_profile_state_live(false);
        if !has_catalog {
            self.as_mut()
                .set_effects_state(QString::from("Unavailable"));
            self.as_mut()
                .set_effects_detail(QString::from("Effects profiles are unavailable."));
            self.as_mut().set_eq_state(QString::from("Unavailable"));
            self.as_mut()
                .set_eq_detail(QString::from("EQ presets are unavailable."));
        }
    }

    fn set_write_failure(mut self: Pin<&mut Self>, detail: &str) {
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
        let was_modified = self.as_ref().eq_state().to_string() == "Modified";
        let Some(mut draft) = self.as_ref().rust().eq_draft.clone() else {
            return;
        };
        if let Err(error) = draft.set_band_gain(index, gain_tenths_db) {
            self.as_mut()
                .set_eq_detail(QString::from(format!("EQ draft was not changed: {error}")));
            return;
        }
        self.as_mut().apply_eq_entry(&draft);
        self.as_mut().sync_eq_modified_state(was_modified);
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
        let was_modified = self.as_ref().effects_state().to_string() == "Modified";
        let Some(mut draft) = self.as_ref().rust().effects_draft.clone() else {
            return;
        };
        if let Err(error) = draft.set_control(&control.to_string(), enabled, level) {
            self.as_mut().set_effects_detail(QString::from(format!(
                "Effects draft was not changed: {error}"
            )));
            return;
        }
        self.as_mut().apply_effects_entry(&draft);
        self.as_mut().sync_effects_modified_state(was_modified);
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
        let was_modified = self.as_ref().eq_state().to_string() == "Modified";
        let id = self.as_ref().rust().eq_id.clone();
        let entry = self
            .as_ref()
            .rust()
            .eq_entries
            .iter()
            .find(|entry| entry.id == id)
            .cloned();
        if let Some(entry) = entry {
            self.as_mut().apply_eq_entry(&entry);
            if was_modified {
                self.as_mut().decrement_unsaved_count();
            }
        }
    }

    pub fn revert_effects_draft(mut self: Pin<&mut Self>) {
        let was_modified = self.as_ref().effects_state().to_string() == "Modified";
        let id = self.as_ref().rust().effects_id.clone();
        let entry = self
            .as_ref()
            .rust()
            .effects_entries
            .iter()
            .find(|entry| entry.id == id)
            .cloned();
        if let Some(entry) = entry {
            self.as_mut().apply_effects_entry(&entry);
            if was_modified {
                self.as_mut().decrement_unsaved_count();
            }
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
        match crate::device_service::write_eq_preset(&draft) {
            Ok(saved) => self.as_mut().accept_saved_eq(saved),
            Err(error) => self.as_mut().set_eq_save_failure(&error.to_string()),
        }
    }

    pub fn save_eq_draft_as(mut self: Pin<&mut Self>, name: &QString) {
        let Some(draft) = self.as_ref().rust().eq_draft.clone() else {
            return;
        };
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
        match crate::device_service::write_effects_profile(&draft) {
            Ok(saved) => self.as_mut().accept_saved_effects(saved),
            Err(error) => self.as_mut().set_effects_save_failure(&error.to_string()),
        }
    }

    pub fn save_effects_draft_as(mut self: Pin<&mut Self>, name: &QString) {
        let Some(draft) = self.as_ref().rust().effects_draft.clone() else {
            return;
        };
        match crate::device_service::write_effects_profile_as(&draft, &name.to_string()) {
            Ok(saved) => self.as_mut().accept_saved_effects(saved),
            Err(error) => self.as_mut().set_effects_save_failure(&error.to_string()),
        }
    }

    fn sync_eq_modified_state(mut self: Pin<&mut Self>, was_modified: bool) {
        let modified = {
            let this = self.as_ref();
            let rust = this.rust();
            rust.eq_draft.as_ref() != rust.eq_entries.iter().find(|entry| entry.id == rust.eq_id)
        };
        self.as_mut()
            .set_section_modified_count(was_modified, modified);
        self.as_mut()
            .set_eq_state(QString::from(if modified { "Modified" } else { "Preview" }));
        self.as_mut().set_eq_detail(QString::from(if modified {
            "Draft changed locally. Live audio and the saved EQ preset are unchanged."
        } else {
            "Draft matches the saved preset. Use Apply EQ to change live audio."
        }));
    }

    fn sync_effects_modified_state(mut self: Pin<&mut Self>, was_modified: bool) {
        let modified = {
            let this = self.as_ref();
            let rust = this.rust();
            rust.effects_draft.as_ref()
                != rust
                    .effects_entries
                    .iter()
                    .find(|entry| entry.id == rust.effects_id)
        };
        self.as_mut()
            .set_section_modified_count(was_modified, modified);
        self.as_mut().set_effects_state(QString::from(if modified {
            "Modified"
        } else {
            "Preview"
        }));
        self.as_mut().set_effects_detail(QString::from(if modified {
            "Draft changed locally. Live audio and the saved Effects profile are unchanged."
        } else {
            "Draft matches the saved profile. Live audio is unchanged."
        }));
    }

    fn set_section_modified_count(mut self: Pin<&mut Self>, was_modified: bool, modified: bool) {
        let count = *self.as_ref().unsaved_count();
        if modified && !was_modified {
            self.as_mut().set_unsaved_count(count.saturating_add(1));
        } else if was_modified && !modified {
            self.as_mut().set_unsaved_count((count - 1).max(0));
        }
    }

    fn decrement_unsaved_count(mut self: Pin<&mut Self>) {
        let count = *self.as_ref().unsaved_count();
        self.as_mut().set_unsaved_count((count - 1).max(0));
    }

    fn accept_saved_eq(mut self: Pin<&mut Self>, entry: crate::EqPresetEntry) {
        let was_modified = self.as_ref().eq_state().to_string() == "Modified";
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
        if was_modified {
            self.as_mut().decrement_unsaved_count();
        }
    }

    fn accept_saved_effects(mut self: Pin<&mut Self>, entry: crate::EffectsProfileEntry) {
        let was_modified = self.as_ref().effects_state().to_string() == "Modified";
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
            "Effects profile saved independently. Live audio is unchanged.",
        ));
        if was_modified {
            self.as_mut().decrement_unsaved_count();
        }
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
