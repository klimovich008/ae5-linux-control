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
        #[qproperty(QStringList, eq_preset_names, cxx_name = "eqPresetNames")]
        #[qproperty(QStringList, eq_band_gains_tenths_db, cxx_name = "eqBandGainsTenthsDb")]
        #[qproperty(bool, eq_enabled, cxx_name = "eqEnabled")]
        #[qproperty(i32, eq_selection_revision, cxx_name = "eqSelectionRevision")]
        #[qproperty(QString, effects_profile, cxx_name = "effectsProfile")]
        #[qproperty(QString, effects_state, cxx_name = "effectsState")]
        #[qproperty(QString, effects_source, cxx_name = "effectsSource")]
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
        #[cxx_name = "markEqModified"]
        fn mark_eq_modified(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "selectEqPreset"]
        fn select_eq_preset(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "markEffectsModified"]
        fn mark_effects_modified(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "selectEffectsProfile"]
        fn select_effects_profile(self: Pin<&mut Self>, name: &QString);

        #[qinvokable]
        #[cxx_name = "saveEqPreview"]
        fn save_eq_preview(self: Pin<&mut Self>);

        #[qinvokable]
        #[cxx_name = "saveEffectsPreview"]
        fn save_effects_preview(self: Pin<&mut Self>);
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
    eq_preset_names: QStringList,
    eq_band_gains_tenths_db: QStringList,
    eq_enabled: bool,
    eq_selection_revision: i32,
    effects_profile: QString,
    effects_state: QString,
    effects_source: QString,
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
    effects_id: String,
    eq_id: String,
    catalog_output: String,
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
            eq_preset_names: QStringList::default(),
            eq_band_gains_tenths_db: QStringList::default(),
            eq_enabled: false,
            eq_selection_revision: 0,
            effects_profile: QString::from("Loading…"),
            effects_state: QString::from("Loading"),
            effects_source: QString::default(),
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
            effects_id: String::new(),
            eq_id: String::new(),
            catalog_output: String::new(),
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
        self.as_mut().rust_mut().effects_id = entry.id.clone();
    }

    fn apply_eq_entry(mut self: Pin<&mut Self>, entry: &crate::EqPresetEntry) {
        self.as_mut().set_eq_preset(QString::from(&entry.name));
        self.as_mut().set_eq_state(QString::from("Preview"));
        self.as_mut().set_eq_source(QString::from(&entry.source));
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
        self.as_mut().rust_mut().eq_id = entry.id.clone();
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
            self.as_mut().set_eq_state(QString::from("Unavailable"));
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

    pub fn mark_eq_modified(mut self: Pin<&mut Self>) {
        if self.as_ref().eq_state().to_string() != "Modified" {
            self.as_mut().set_eq_state(QString::from("Modified"));
            let count = *self.as_ref().unsaved_count();
            self.as_mut().set_unsaved_count(count + 1);
        }
    }

    pub fn select_eq_preset(mut self: Pin<&mut Self>, name: &QString) {
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

    pub fn mark_effects_modified(mut self: Pin<&mut Self>) {
        if self.as_ref().effects_state().to_string() != "Modified" {
            self.as_mut().set_effects_state(QString::from("Modified"));
            let count = *self.as_ref().unsaved_count();
            self.as_mut().set_unsaved_count(count + 1);
        }
    }

    pub fn select_effects_profile(mut self: Pin<&mut Self>, name: &QString) {
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

    pub fn save_eq_preview(mut self: Pin<&mut Self>) {
        if self.as_ref().eq_state().to_string() == "Modified" {
            self.as_mut().set_eq_state(QString::from("Saved"));
            let count = *self.as_ref().unsaved_count();
            self.as_mut().set_unsaved_count((count - 1).max(0));
        }
    }

    pub fn save_effects_preview(mut self: Pin<&mut Self>) {
        if self.as_ref().effects_state().to_string() == "Modified" {
            self.as_mut().set_effects_state(QString::from("Saved"));
            let count = *self.as_ref().unsaved_count();
            self.as_mut().set_unsaved_count((count - 1).max(0));
        }
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
                .find(|entry| !entry.read_only && entry.name.eq_ignore_ascii_case("My profile"))
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
                .find(|entry| !entry.read_only && entry.name.eq_ignore_ascii_case("SHP Last"))
        })
        .or_else(|| entries.iter().find(|entry| !entry.read_only))
        .or_else(|| entries.first())
}
