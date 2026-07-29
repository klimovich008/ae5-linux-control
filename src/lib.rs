pub mod builtin_profiles;
pub mod controls;
pub mod device;
#[cfg(feature = "daemon")]
pub mod device_service;
pub mod device_state;
pub mod eq_chain;
pub mod feature_parity;
pub mod lighting;
pub mod pipewire;
pub mod profile;
pub mod profile_library;
#[cfg(feature = "qml-gui")]
pub mod qml_app_state;
pub mod sbcommand;
pub mod sound_objects;
pub mod volume_curve;

pub use builtin_profiles::{BuiltinProfile, COMMAND_DEFAULT_PROFILE_COUNT, builtin_profiles};
pub use controls::{
    Ae5Mixer, ChannelLevel, ControlError, ControlSnapshot, DIRECT_MODE_CONTROL, DecibelRange,
    HARDWARE_OUTFX_CONTROL, Level, capture_control_block_reason, direct_mode_block_reason,
    equalizer_band_block_reason, front_vmaster_clamp_warning, hardware_outfx_lab_active,
    headphone_playback_issue, playback_switch_block_reason, smart_volume_level_block_reason,
    snapshot_controls, unsafe_playback_control_block_reason,
};
pub use device::Ae5Device;
pub use device_state::{DeviceOutputState, DeviceStatusCode};
pub use eq_chain::{
    EQ_FREQUENCIES, EqBand, EqChainChange, EqChainConfig, EqChainError, bands_from_gains_tenths_db,
    bands_from_profile, disable_eq_chain, enable_eq_chain, enable_eq_chain_bands, eq_chain_config,
    restore_eq_chain_config, validate_eq_chain_activation,
};
pub use feature_parity::{FeatureParity, FeatureSupport, feature_parity};
pub use lighting::{
    Ae5Lighting, LightingConfig, ONBOARD_LED_COUNT, RgbColor, lighting_config_path,
    restore_saved_lighting, saved_lighting, set_saved_led, set_saved_lighting,
};
pub use pipewire::{
    AudioFormat, NativeRatesConfig, PipeWireNode, PipeWireRouteState, RuntimeSampleRate,
    SoftwareEqOutput, SoftwareVolumeOutput, ae5_audio_format, ae5_input, ae5_output,
    ae5_route_state, ae5_windows_volume_curve_active, apply_software_eq, native_rates_config,
    remove_software_eq, replace_software_eq, runtime_sample_rate, set_ae5_default_input,
    set_ae5_default_output, set_ae5_runtime_sample_rate, set_ae5_software_mute,
    set_ae5_software_volume, set_native_rates_enabled, software_eq_output, unload_software_eq,
};
pub use profile::{
    ApplyReport, LINUX_DRIVER_DEFAULTS_PRESERVED, Profile, ProfileControl, ProfileError,
    apply_linux_driver_defaults, linux_driver_defaults, linux_driver_defaults_for,
    validate_linux_driver_defaults,
};
pub use profile_library::{
    ProfileLibrary, StoredProfile, export_library_profile, library_profile, profile_library,
    profile_library_directory, rename_library_profile,
};
pub use sbcommand::{
    SbCommandError, SbCommandImport, SbCommandImportReport, SbCommandInstallation, SbCommandTarget,
    discover_installation as discover_sbcommand_installation,
    import_active_profile_with_report as import_active_sbcommand_profile_with_report,
    import_installation_profile_with_report as import_discovered_sbcommand_profile_with_report,
    import_profile as import_sbcommand_profile,
    import_profile_with_report as import_sbcommand_profile_with_report,
};
pub use sound_objects::{
    EffectsProfileEntry, EqPresetEntry, SoundObjectCatalog, save_effects_profile,
    save_effects_profile_as, save_eq_preset, save_eq_preset_as, sound_object_catalog,
};
pub use volume_curve::{
    VolumeCurveError, WindowsVolumeCurve, WindowsVolumePoint, windows_ae5_decibels,
    windows_ae5_pipewire_percent,
};

#[cfg(feature = "gui")]
pub mod gui;
