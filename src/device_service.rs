use crate::{
    Ae5Device, DeviceOutputState, EffectsChainConfig, EffectsProfileEntry, EqChainConfig,
    EqPresetEntry, RuntimeSampleRate, SoftwareEffectsOutput, SoftwareEqOutput, SoundObjectCatalog,
    ae5_output, apply_hardware_effects as apply_hardware_effects_controls,
    bands_from_gains_tenths_db, disable_effects_chain, disable_eq_chain,
    disable_hardware_effects as disable_hardware_effects_controls, effects_chain_config,
    enable_effects_chain, enable_eq_chain_bands, eq_chain_config, remove_software_effects,
    remove_software_eq, replace_software_effects, replace_software_eq,
    restore_effects_chain_config, restore_eq_chain_config, save_effects_profile,
    save_effects_profile_as, save_eq_preset, save_eq_preset_as, set_ae5_runtime_sample_rate,
    set_ae5_software_mute, set_ae5_software_volume, software_effects_output, software_eq_output,
    sound_object_catalog,
};

pub const SERVICE_NAME: &str = "io.github.klimovich008.Ae5Control";
pub const OBJECT_PATH: &str = "/io/github/klimovich008/Ae5Control";
pub const INTERFACE_NAME: &str = "io.github.klimovich008.Ae5Control.Device1";

pub struct Ae5DeviceService;

#[zbus::interface(name = "io.github.klimovich008.Ae5Control.Device1")]
impl Ae5DeviceService {
    fn get_device_state(&self) -> zbus::fdo::Result<DeviceOutputState> {
        capture_state()
    }

    fn get_sound_object_catalog(&self) -> zbus::fdo::Result<SoundObjectCatalog> {
        let state = capture_state()?;
        sound_object_catalog(&state.output, None)
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }

    fn save_effects_profile(
        &self,
        draft: EffectsProfileEntry,
    ) -> zbus::fdo::Result<EffectsProfileEntry> {
        let output = capture_state()?.output;
        log_profile_write(
            "effects-save",
            &draft.name,
            save_effects_profile(&draft, &output)
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string())),
        )
    }

    fn save_effects_profile_as(
        &self,
        draft: EffectsProfileEntry,
        name: String,
    ) -> zbus::fdo::Result<EffectsProfileEntry> {
        let output = capture_state()?.output;
        log_profile_write(
            "effects-save-as",
            &name,
            save_effects_profile_as(&draft, &name, &output)
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string())),
        )
    }

    fn save_eq_preset(&self, draft: EqPresetEntry) -> zbus::fdo::Result<EqPresetEntry> {
        let output = capture_state()?.output;
        log_profile_write(
            "eq-save",
            &draft.name,
            save_eq_preset(&draft, &output)
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string())),
        )
    }

    fn save_eq_preset_as(
        &self,
        draft: EqPresetEntry,
        name: String,
    ) -> zbus::fdo::Result<EqPresetEntry> {
        let output = capture_state()?.output;
        log_profile_write(
            "eq-save-as",
            &name,
            save_eq_preset_as(&draft, &name, &output)
                .map_err(|error| zbus::fdo::Error::Failed(error.to_string())),
        )
    }

    fn apply_eq_preset(&self, draft: EqPresetEntry) -> zbus::fdo::Result<DeviceOutputState> {
        log_write("software-eq", "apply", apply_eq_preset_checked(&draft))
    }

    fn disable_software_eq(&self) -> zbus::fdo::Result<DeviceOutputState> {
        log_write("software-eq", "disable", disable_software_eq_checked())
    }

    fn apply_effects_profile(
        &self,
        draft: EffectsProfileEntry,
    ) -> zbus::fdo::Result<DeviceOutputState> {
        log_write(
            "hardware-effects",
            "apply",
            apply_hardware_effects_profile_checked(&draft),
        )
    }

    fn disable_hardware_effects(&self) -> zbus::fdo::Result<DeviceOutputState> {
        log_write(
            "hardware-effects",
            "disable",
            disable_hardware_effects_checked(),
        )
    }

    fn apply_software_effects_profile(
        &self,
        draft: EffectsProfileEntry,
    ) -> zbus::fdo::Result<DeviceOutputState> {
        log_write(
            "software-effects",
            "apply",
            apply_software_effects_profile_checked(&draft),
        )
    }

    fn disable_software_effects(&self) -> zbus::fdo::Result<DeviceOutputState> {
        log_write(
            "software-effects",
            "disable",
            disable_software_effects_checked(),
        )
    }

    fn set_master_volume(&self, percent: u16) -> zbus::fdo::Result<DeviceOutputState> {
        log_write(
            "master-volume",
            &percent.to_string(),
            (|| {
                if percent > 100 {
                    return Err(zbus::fdo::Error::InvalidArgs(
                        "Master volume must be between 0 and 100 percent.".to_owned(),
                    ));
                }
                let card_index = live_card_index()?;
                set_ae5_software_volume(card_index, f64::from(percent))
                    .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
                let state = capture_state()?;
                if !state.volume_available || state.master_volume != percent {
                    return Err(zbus::fdo::Error::Failed(format!(
                        "Master volume read back as {}%, expected {percent}%.",
                        state.master_volume
                    )));
                }
                Ok(state)
            })(),
        )
    }

    fn set_muted(&self, muted: bool) -> zbus::fdo::Result<DeviceOutputState> {
        log_write(
            "mute",
            if muted { "on" } else { "off" },
            (|| {
                let card_index = live_card_index()?;
                set_ae5_software_mute(card_index, muted)
                    .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
                let state = capture_state()?;
                if !state.mute_available || state.muted != muted {
                    return Err(zbus::fdo::Error::Failed(format!(
                        "Mute read back as {}, expected {muted}.",
                        state.muted
                    )));
                }
                Ok(state)
            })(),
        )
    }

    fn set_sample_rate_policy(&self, policy: String) -> zbus::fdo::Result<DeviceOutputState> {
        log_write(
            "sample-rate-policy",
            &policy,
            (|| {
                let requested =
                    RuntimeSampleRate::from_policy_name(&policy).ok_or_else(|| {
                        zbus::fdo::Error::InvalidArgs(format!(
                            "Unsupported sample-rate policy '{policy}'; expected Automatic, 48 kHz, or 96 kHz."
                        ))
                    })?;
                let card_index = live_card_index()?;
                let applied = set_ae5_runtime_sample_rate(card_index, requested)
                    .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
                if applied != requested {
                    return Err(zbus::fdo::Error::Failed(format!(
                        "Sample-rate policy read back as '{}', expected '{}'.",
                        applied.policy_name(),
                        requested.policy_name()
                    )));
                }
                let state = capture_state()?;
                if !state.sample_rate_policy_available
                    || state.sample_rate_policy != requested.policy_name()
                {
                    return Err(zbus::fdo::Error::Failed(format!(
                        "Sample-rate policy state read back as '{}', expected '{}'.",
                        state.sample_rate_policy,
                        requested.policy_name()
                    )));
                }
                Ok(state)
            })(),
        )
    }
}

fn log_write<T>(
    operation: &str,
    requested: &str,
    result: zbus::fdo::Result<T>,
) -> zbus::fdo::Result<T> {
    match &result {
        Ok(_) => eprintln!(
            "ae5d event=write-complete operation={operation} requested={requested} result=verified"
        ),
        Err(error) => eprintln!(
            "ae5d event=write-failed operation={operation} requested={requested} error={error}"
        ),
    }
    result
}

fn log_profile_write<T>(
    operation: &str,
    object: &str,
    result: zbus::fdo::Result<T>,
) -> zbus::fdo::Result<T> {
    match &result {
        Ok(_) => eprintln!(
            "ae5d event=profile-write-complete operation={operation} object={object:?} result=verified"
        ),
        Err(error) => eprintln!(
            "ae5d event=profile-write-failed operation={operation} object={object:?} error={error}"
        ),
    }
    result
}

fn capture_state() -> zbus::fdo::Result<DeviceOutputState> {
    DeviceOutputState::capture().map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
}

fn apply_eq_preset_checked(draft: &EqPresetEntry) -> zbus::fdo::Result<DeviceOutputState> {
    draft.validate().map_err(zbus::fdo::Error::InvalidArgs)?;
    if !draft.enabled {
        return Err(zbus::fdo::Error::InvalidArgs(
            "Enable this EQ preset before applying it.".to_owned(),
        ));
    }

    let initial_state = capture_state()?;
    if !initial_state.eq_apply_available {
        return Err(zbus::fdo::Error::Failed(
            initial_state.eq_apply_block_reason,
        ));
    }
    let card_index = initial_state.card_index;
    let output = ae5_output(card_index)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?
        .ok_or_else(|| {
            zbus::fdo::Error::Failed(
                "PipeWire has no AE-5 playback output for software EQ.".to_owned(),
            )
        })?;
    let previous_config =
        eq_chain_config().map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let previous_output = software_eq_output(card_index)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let (previous_graph, previous_signature) =
        known_previous_eq_runtime(&previous_config, previous_output.as_ref())
            .map_err(zbus::fdo::Error::Failed)?;
    let bands = bands_from_gains_tenths_db(&draft.gains_tenths_db)
        .map_err(|error| zbus::fdo::Error::InvalidArgs(error.to_string()))?;
    let change = enable_eq_chain_bands(&bands, &output.node_name)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let graph = change
        .config
        .filter_graph()
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?
        .ok_or_else(|| {
            zbus::fdo::Error::Failed(
                "The saved software equalizer did not produce a filter graph.".to_owned(),
            )
        })?;
    let signature = change.config.signature().ok_or_else(|| {
        zbus::fdo::Error::Failed(
            "The saved software equalizer did not produce a runtime marker.".to_owned(),
        )
    })?;

    if let Err(error) = replace_software_eq(
        card_index,
        &graph,
        &signature,
        previous_graph.as_deref(),
        previous_signature.as_deref(),
    ) {
        return match restore_eq_chain_config(&previous_config) {
            Ok(_) => Err(zbus::fdo::Error::Failed(format!(
                "Software EQ was not applied and the previous configuration was restored: {error}"
            ))),
            Err(rollback_error) => Err(zbus::fdo::Error::Failed(format!(
                "Software EQ apply failed: {error}; configuration rollback also failed: {rollback_error}"
            ))),
        };
    }

    let verification = capture_state().and_then(|state| {
        if state.software_eq_active && state.software_eq_state == "current" {
            Ok(state)
        } else {
            Err(zbus::fdo::Error::Failed(format!(
                "Software EQ runtime verification returned '{}': {}",
                state.software_eq_state, state.software_eq_detail
            )))
        }
    });
    match verification {
        Ok(state) => Ok(state),
        Err(error) => rollback_applied_eq(
            card_index,
            &graph,
            &signature,
            &previous_config,
            previous_graph.as_deref(),
            previous_signature.as_deref(),
            error,
        ),
    }
}

fn disable_software_eq_checked() -> zbus::fdo::Result<DeviceOutputState> {
    let card_index = live_card_index()?;
    let previous_config =
        eq_chain_config().map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let previous_output = software_eq_output(card_index)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let (previous_graph, previous_signature) =
        known_previous_eq_runtime(&previous_config, previous_output.as_ref())
            .map_err(zbus::fdo::Error::Failed)?;

    if previous_output.is_some() {
        let (Some(graph), Some(signature)) =
            (previous_graph.as_deref(), previous_signature.as_deref())
        else {
            return Err(zbus::fdo::Error::Failed(
                "The active software EQ cannot be restored safely.".to_owned(),
            ));
        };
        remove_software_eq(card_index, graph, signature)
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    }
    if let Err(error) = disable_eq_chain() {
        let rollback = match (previous_graph.as_deref(), previous_signature.as_deref()) {
            (Some(graph), Some(signature)) => {
                replace_software_eq(card_index, graph, signature, None, None).map(|_| ())
            }
            _ => Ok(()),
        };
        return match rollback {
            Ok(()) => Err(zbus::fdo::Error::Failed(format!(
                "Software EQ was not disabled and the previous runtime was restored: {error}"
            ))),
            Err(rollback_error) => Err(zbus::fdo::Error::Failed(format!(
                "Software EQ disable failed: {error}; runtime rollback also failed: {rollback_error}"
            ))),
        };
    }

    let verification = capture_state().and_then(|state| {
        if !state.software_eq_active && state.software_eq_state == "inactive" {
            Ok(state)
        } else {
            Err(zbus::fdo::Error::Failed(format!(
                "Software EQ disable readback returned '{}': {}",
                state.software_eq_state, state.software_eq_detail
            )))
        }
    });
    match verification {
        Ok(state) => Ok(state),
        Err(error) => rollback_disabled_eq(
            card_index,
            &previous_config,
            previous_graph.as_deref(),
            previous_signature.as_deref(),
            error,
        ),
    }
}

fn apply_hardware_effects_profile_checked(
    draft: &EffectsProfileEntry,
) -> zbus::fdo::Result<DeviceOutputState> {
    draft.validate().map_err(zbus::fdo::Error::InvalidArgs)?;
    if !draft.outfx_enabled {
        return Err(zbus::fdo::Error::InvalidArgs(
            "Enable the Effects master before applying this profile.".to_owned(),
        ));
    }

    let initial_state = capture_state()?;
    if !initial_state.effects_apply_available {
        return Err(zbus::fdo::Error::Failed(
            initial_state.effects_apply_block_reason,
        ));
    }
    apply_hardware_effects_controls(initial_state.card_index, draft)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let state = capture_state()?;
    if state.hardware_effects_active && state.hardware_effects_state == "current" {
        Ok(state)
    } else {
        Err(zbus::fdo::Error::Failed(format!(
            "Hardware Effects readback returned '{}': {}",
            state.hardware_effects_state, state.hardware_effects_detail
        )))
    }
}

fn disable_hardware_effects_checked() -> zbus::fdo::Result<DeviceOutputState> {
    let initial_state = capture_state()?;
    if !initial_state.hardware_effects_active {
        return Ok(initial_state);
    }
    disable_hardware_effects_controls(initial_state.card_index)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let state = capture_state()?;
    if !state.hardware_effects_active
        && matches!(
            state.hardware_effects_state.as_str(),
            "configured" | "inactive"
        )
    {
        Ok(state)
    } else {
        Err(zbus::fdo::Error::Failed(format!(
            "Hardware Effects disable readback returned '{}': {}",
            state.hardware_effects_state, state.hardware_effects_detail
        )))
    }
}

fn apply_software_effects_profile_checked(
    draft: &EffectsProfileEntry,
) -> zbus::fdo::Result<DeviceOutputState> {
    draft.validate().map_err(zbus::fdo::Error::InvalidArgs)?;
    if !draft.outfx_enabled {
        return Err(zbus::fdo::Error::InvalidArgs(
            "Enable the Effects master before applying this profile.".to_owned(),
        ));
    }

    let initial_state = capture_state()?;
    if !initial_state.software_effects_apply_available {
        return Err(zbus::fdo::Error::Failed(
            initial_state.software_effects_apply_block_reason,
        ));
    }
    let card_index = initial_state.card_index;
    let output = ae5_output(card_index)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?
        .ok_or_else(|| {
            zbus::fdo::Error::Failed(
                "PipeWire has no AE-5 playback output for software Effects.".to_owned(),
            )
        })?;
    let previous_config =
        effects_chain_config().map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let previous_output = software_effects_output(card_index)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let (previous_graph, previous_signature) =
        known_previous_effects_runtime(&previous_config, previous_output.as_ref())
            .map_err(zbus::fdo::Error::Failed)?;
    let change = enable_effects_chain(draft, &output.node_name)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let graph = change
        .config
        .filter_graph()
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?
        .ok_or_else(|| {
            zbus::fdo::Error::Failed(
                "The saved software Effects profile did not produce a filter graph.".to_owned(),
            )
        })?;
    let signature = change.config.signature().ok_or_else(|| {
        zbus::fdo::Error::Failed(
            "The saved software Effects profile did not produce a runtime marker.".to_owned(),
        )
    })?;

    if let Err(error) = replace_software_effects(
        card_index,
        &graph,
        &signature,
        previous_graph.as_deref(),
        previous_signature.as_deref(),
    ) {
        return match restore_effects_chain_config(&previous_config) {
            Ok(_) => Err(zbus::fdo::Error::Failed(format!(
                "Software Effects were not applied and the previous configuration was restored: {error}"
            ))),
            Err(rollback_error) => Err(zbus::fdo::Error::Failed(format!(
                "Software Effects apply failed: {error}; configuration rollback also failed: {rollback_error}"
            ))),
        };
    }

    let verification = capture_state().and_then(|state| {
        if state.software_effects_active && state.software_effects_state == "current" {
            Ok(state)
        } else {
            Err(zbus::fdo::Error::Failed(format!(
                "Software Effects runtime verification returned '{}': {}",
                state.software_effects_state, state.software_effects_detail
            )))
        }
    });
    match verification {
        Ok(state) => Ok(state),
        Err(error) => rollback_applied_effects(
            card_index,
            &graph,
            &signature,
            &previous_config,
            previous_graph.as_deref(),
            previous_signature.as_deref(),
            error,
        ),
    }
}

fn disable_software_effects_checked() -> zbus::fdo::Result<DeviceOutputState> {
    let card_index = live_card_index()?;
    let previous_config =
        effects_chain_config().map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let previous_output = software_effects_output(card_index)
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    let (previous_graph, previous_signature) =
        known_previous_effects_runtime(&previous_config, previous_output.as_ref())
            .map_err(zbus::fdo::Error::Failed)?;

    if previous_output.is_some() {
        let (Some(graph), Some(signature)) =
            (previous_graph.as_deref(), previous_signature.as_deref())
        else {
            return Err(zbus::fdo::Error::Failed(
                "The active software Effects graph cannot be restored safely.".to_owned(),
            ));
        };
        remove_software_effects(card_index, graph, signature)
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?;
    }
    if let Err(error) = disable_effects_chain() {
        let rollback = match (previous_graph.as_deref(), previous_signature.as_deref()) {
            (Some(graph), Some(signature)) => {
                replace_software_effects(card_index, graph, signature, None, None).map(|_| ())
            }
            _ => Ok(()),
        };
        return match rollback {
            Ok(()) => Err(zbus::fdo::Error::Failed(format!(
                "Software Effects were not disabled and the previous runtime was restored: {error}"
            ))),
            Err(rollback_error) => Err(zbus::fdo::Error::Failed(format!(
                "Software Effects disable failed: {error}; runtime rollback also failed: {rollback_error}"
            ))),
        };
    }

    let verification = capture_state().and_then(|state| {
        if !state.software_effects_active && state.software_effects_state == "inactive" {
            Ok(state)
        } else {
            Err(zbus::fdo::Error::Failed(format!(
                "Software Effects disable readback returned '{}': {}",
                state.software_effects_state, state.software_effects_detail
            )))
        }
    });
    match verification {
        Ok(state) => Ok(state),
        Err(error) => rollback_disabled_effects(
            card_index,
            &previous_config,
            previous_graph.as_deref(),
            previous_signature.as_deref(),
            error,
        ),
    }
}

fn rollback_applied_eq(
    card_index: i32,
    applied_graph: &str,
    applied_signature: &str,
    previous_config: &EqChainConfig,
    previous_graph: Option<&str>,
    previous_signature: Option<&str>,
    failure: zbus::fdo::Error,
) -> zbus::fdo::Result<DeviceOutputState> {
    let runtime = match (previous_graph, previous_signature) {
        (Some(graph), Some(signature)) => replace_software_eq(
            card_index,
            graph,
            signature,
            Some(applied_graph),
            Some(applied_signature),
        )
        .map(|_| ()),
        (None, None) => {
            remove_software_eq(card_index, applied_graph, applied_signature).map(|_| ())
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "previous software EQ graph and signature do not match",
        )),
    };
    let config = restore_eq_chain_config(previous_config);
    match (runtime, config) {
        (Ok(()), Ok(_)) => Err(zbus::fdo::Error::Failed(format!(
            "{failure}; the previous software EQ state was restored"
        ))),
        (runtime, config) => Err(zbus::fdo::Error::Failed(format!(
            "{failure}; rollback failed (runtime: {}; configuration: {})",
            runtime
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "restored".to_owned()),
            config
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "restored".to_owned())
        ))),
    }
}

fn rollback_disabled_eq(
    card_index: i32,
    previous_config: &EqChainConfig,
    previous_graph: Option<&str>,
    previous_signature: Option<&str>,
    failure: zbus::fdo::Error,
) -> zbus::fdo::Result<DeviceOutputState> {
    let config = restore_eq_chain_config(previous_config);
    let runtime = match (previous_graph, previous_signature) {
        (Some(graph), Some(signature)) => {
            replace_software_eq(card_index, graph, signature, None, None).map(|_| ())
        }
        (None, None) => Ok(()),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "previous software EQ graph and signature do not match",
        )),
    };
    match (config, runtime) {
        (Ok(_), Ok(())) => Err(zbus::fdo::Error::Failed(format!(
            "{failure}; the previous software EQ state was restored"
        ))),
        (config, runtime) => Err(zbus::fdo::Error::Failed(format!(
            "{failure}; rollback failed (configuration: {}; runtime: {})",
            config
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "restored".to_owned()),
            runtime
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "restored".to_owned())
        ))),
    }
}

fn rollback_applied_effects(
    card_index: i32,
    applied_graph: &str,
    applied_signature: &str,
    previous_config: &EffectsChainConfig,
    previous_graph: Option<&str>,
    previous_signature: Option<&str>,
    failure: zbus::fdo::Error,
) -> zbus::fdo::Result<DeviceOutputState> {
    let runtime = match (previous_graph, previous_signature) {
        (Some(graph), Some(signature)) => replace_software_effects(
            card_index,
            graph,
            signature,
            Some(applied_graph),
            Some(applied_signature),
        )
        .map(|_| ()),
        (None, None) => {
            remove_software_effects(card_index, applied_graph, applied_signature).map(|_| ())
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "previous software Effects graph and signature do not match",
        )),
    };
    let config = restore_effects_chain_config(previous_config);
    match (runtime, config) {
        (Ok(()), Ok(_)) => Err(zbus::fdo::Error::Failed(format!(
            "{failure}; the previous software Effects state was restored"
        ))),
        (runtime, config) => Err(zbus::fdo::Error::Failed(format!(
            "{failure}; rollback failed (runtime: {}; configuration: {})",
            runtime
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "restored".to_owned()),
            config
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "restored".to_owned())
        ))),
    }
}

fn rollback_disabled_effects(
    card_index: i32,
    previous_config: &EffectsChainConfig,
    previous_graph: Option<&str>,
    previous_signature: Option<&str>,
    failure: zbus::fdo::Error,
) -> zbus::fdo::Result<DeviceOutputState> {
    let config = restore_effects_chain_config(previous_config);
    let runtime = match (previous_graph, previous_signature) {
        (Some(graph), Some(signature)) => {
            replace_software_effects(card_index, graph, signature, None, None).map(|_| ())
        }
        (None, None) => Ok(()),
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "previous software Effects graph and signature do not match",
        )),
    };
    match (config, runtime) {
        (Ok(_), Ok(())) => Err(zbus::fdo::Error::Failed(format!(
            "{failure}; the previous software Effects state was restored"
        ))),
        (config, runtime) => Err(zbus::fdo::Error::Failed(format!(
            "{failure}; rollback failed (configuration: {}; runtime: {})",
            config
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "restored".to_owned()),
            runtime
                .err()
                .map(|error| error.to_string())
                .unwrap_or_else(|| "restored".to_owned())
        ))),
    }
}

fn known_previous_eq_runtime(
    config: &EqChainConfig,
    output: Option<&SoftwareEqOutput>,
) -> Result<(Option<String>, Option<String>), String> {
    let Some(output) = output else {
        return Ok((None, None));
    };
    let expected_signature = config.signature().ok_or_else(|| {
        "An AE-5 software EQ graph is active without a matching managed configuration.".to_owned()
    })?;
    if output.signature.as_deref() != Some(expected_signature.as_str()) {
        return Err(
            "The active AE-5 software EQ graph changed outside ae5d; disable it before applying another preset."
                .to_owned(),
        );
    }
    let graph = config
        .filter_graph()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "The managed software EQ has no restorable filter graph.".to_owned())?;
    Ok((Some(graph), Some(expected_signature)))
}

fn known_previous_effects_runtime(
    config: &EffectsChainConfig,
    output: Option<&SoftwareEffectsOutput>,
) -> Result<(Option<String>, Option<String>), String> {
    let Some(output) = output else {
        return Ok((None, None));
    };
    let expected_signature = config.signature().ok_or_else(|| {
        "An AE-5 software Effects graph is active without a matching managed configuration."
            .to_owned()
    })?;
    if output.signature.as_deref() != Some(expected_signature.as_str()) {
        return Err(
            "The active AE-5 software Effects graph changed outside ae5d; disable it before applying another profile."
                .to_owned(),
        );
    }
    let graph = config
        .filter_graph()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            "The managed software Effects profile has no restorable graph.".to_owned()
        })?;
    Ok((Some(graph), Some(expected_signature)))
}

fn live_card_index() -> zbus::fdo::Result<i32> {
    Ae5Device::discover()
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))?
        .map(|device| device.card_index)
        .ok_or_else(|| {
            zbus::fdo::Error::Failed("No compatible Sound BlasterX AE-5 was detected.".to_owned())
        })
}

#[zbus::proxy(
    interface = "io.github.klimovich008.Ae5Control.Device1",
    default_service = "io.github.klimovich008.Ae5Control",
    default_path = "/io/github/klimovich008/Ae5Control",
    gen_async = false,
    blocking_name = "Ae5DeviceProxy"
)]
trait Ae5Device {
    fn get_device_state(&self) -> zbus::Result<DeviceOutputState>;
    fn get_sound_object_catalog(&self) -> zbus::Result<SoundObjectCatalog>;
    fn save_effects_profile(&self, draft: EffectsProfileEntry)
    -> zbus::Result<EffectsProfileEntry>;
    fn save_effects_profile_as(
        &self,
        draft: EffectsProfileEntry,
        name: &str,
    ) -> zbus::Result<EffectsProfileEntry>;
    fn save_eq_preset(&self, draft: EqPresetEntry) -> zbus::Result<EqPresetEntry>;
    fn save_eq_preset_as(&self, draft: EqPresetEntry, name: &str) -> zbus::Result<EqPresetEntry>;
    fn apply_eq_preset(&self, draft: EqPresetEntry) -> zbus::Result<DeviceOutputState>;
    fn disable_software_eq(&self) -> zbus::Result<DeviceOutputState>;
    fn apply_effects_profile(&self, draft: EffectsProfileEntry) -> zbus::Result<DeviceOutputState>;
    fn disable_hardware_effects(&self) -> zbus::Result<DeviceOutputState>;
    fn apply_software_effects_profile(
        &self,
        draft: EffectsProfileEntry,
    ) -> zbus::Result<DeviceOutputState>;
    fn disable_software_effects(&self) -> zbus::Result<DeviceOutputState>;
    fn set_master_volume(&self, percent: u16) -> zbus::Result<DeviceOutputState>;
    fn set_muted(&self, muted: bool) -> zbus::Result<DeviceOutputState>;
    fn set_sample_rate_policy(&self, policy: &str) -> zbus::Result<DeviceOutputState>;
}

pub fn serve() -> zbus::Result<zbus::blocking::Connection> {
    zbus::blocking::connection::Builder::session()?
        .name(SERVICE_NAME)?
        .serve_at(OBJECT_PATH, Ae5DeviceService)?
        .build()
}

pub fn read_device_state() -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.get_device_state()
}

pub fn read_sound_object_catalog() -> zbus::Result<SoundObjectCatalog> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.get_sound_object_catalog()
}

pub fn write_effects_profile(draft: &EffectsProfileEntry) -> zbus::Result<EffectsProfileEntry> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.save_effects_profile(draft.clone())
}

pub fn write_effects_profile_as(
    draft: &EffectsProfileEntry,
    name: &str,
) -> zbus::Result<EffectsProfileEntry> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.save_effects_profile_as(draft.clone(), name)
}

pub fn write_eq_preset(draft: &EqPresetEntry) -> zbus::Result<EqPresetEntry> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.save_eq_preset(draft.clone())
}

pub fn write_eq_preset_as(draft: &EqPresetEntry, name: &str) -> zbus::Result<EqPresetEntry> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.save_eq_preset_as(draft.clone(), name)
}

pub fn apply_eq_preset(draft: &EqPresetEntry) -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.apply_eq_preset(draft.clone())
}

pub fn disable_software_eq() -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.disable_software_eq()
}

pub fn apply_effects_profile(draft: &EffectsProfileEntry) -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.apply_effects_profile(draft.clone())
}

pub fn disable_hardware_effects() -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.disable_hardware_effects()
}

pub fn apply_software_effects_profile(
    draft: &EffectsProfileEntry,
) -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.apply_software_effects_profile(draft.clone())
}

pub fn disable_software_effects() -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.disable_software_effects()
}

pub fn write_master_volume(percent: u16) -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.set_master_volume(percent)
}

pub fn write_muted(muted: bool) -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.set_muted(muted)
}

pub fn write_sample_rate_policy(policy: &str) -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.set_sample_rate_policy(policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_names_share_one_stable_namespace() {
        assert_eq!(
            (SERVICE_NAME, OBJECT_PATH, INTERFACE_NAME),
            (
                "io.github.klimovich008.Ae5Control",
                "/io/github/klimovich008/Ae5Control",
                "io.github.klimovich008.Ae5Control.Device1",
            )
        );
    }

    #[test]
    fn known_runtime_rebuilds_the_previous_graph() {
        let config = EqChainConfig {
            path: "/tmp/software-eq.state".into(),
            enabled: true,
            bands: crate::EQ_FREQUENCIES
                .map(|frequency| crate::EqBand {
                    frequency,
                    q: 1.4,
                    gain_db: 0.0,
                })
                .to_vec(),
            target_node: Some("alsa_output.pci-ae5.analog-stereo".to_owned()),
            preamp_db: 0.0,
        };
        let signature = config.signature().unwrap();
        let output = SoftwareEqOutput {
            node: crate::PipeWireNode {
                id: 42,
                node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
                description: "AE-5".to_owned(),
                is_default: true,
                volume_percent: Some(20),
                muted: Some(false),
            },
            signature: Some(signature.clone()),
        };

        let (graph, restored_signature) =
            known_previous_eq_runtime(&config, Some(&output)).unwrap();

        assert!(graph.unwrap().contains("bq_peaking"));
        assert_eq!(restored_signature.as_deref(), Some(signature.as_str()));
    }

    #[test]
    fn unknown_runtime_is_refused() {
        let config = EqChainConfig {
            path: "/tmp/software-eq.state".into(),
            enabled: false,
            bands: Vec::new(),
            target_node: None,
            preamp_db: 0.0,
        };
        let output = SoftwareEqOutput {
            node: crate::PipeWireNode {
                id: 42,
                node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
                description: "AE-5".to_owned(),
                is_default: true,
                volume_percent: None,
                muted: None,
            },
            signature: Some("foreign".to_owned()),
        };

        assert!(known_previous_eq_runtime(&config, Some(&output)).is_err());
    }

    #[test]
    fn known_effects_runtime_rebuilds_the_previous_graph() {
        let profile = effects_profile();
        let config = EffectsChainConfig {
            path: "/tmp/software-effects.json".into(),
            enabled: true,
            target_node: Some("alsa_output.pci-ae5.analog-stereo".to_owned()),
            profile: Some(profile),
        };
        let signature = config.signature().unwrap();
        let output = SoftwareEffectsOutput {
            node: crate::PipeWireNode {
                id: 42,
                node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
                description: "AE-5".to_owned(),
                is_default: true,
                volume_percent: Some(5),
                muted: Some(true),
            },
            signature: Some(signature.clone()),
        };

        let (graph, restored_signature) =
            known_previous_effects_runtime(&config, Some(&output)).unwrap();

        let graph = graph.unwrap();
        assert!(graph.contains("matrixSpatialiser"));
        assert!(graph.contains("fastLookaheadLimiter"));
        assert_eq!(restored_signature.as_deref(), Some(signature.as_str()));
    }

    #[test]
    fn unknown_effects_runtime_is_refused() {
        let config = EffectsChainConfig {
            path: "/tmp/software-effects.json".into(),
            enabled: false,
            target_node: None,
            profile: None,
        };
        let output = SoftwareEffectsOutput {
            node: crate::PipeWireNode {
                id: 42,
                node_name: "alsa_output.pci-ae5.analog-stereo".to_owned(),
                description: "AE-5".to_owned(),
                is_default: true,
                volume_percent: None,
                muted: None,
            },
            signature: Some("foreign".to_owned()),
        };

        assert!(known_previous_effects_runtime(&config, Some(&output)).is_err());
    }

    fn effects_profile() -> EffectsProfileEntry {
        EffectsProfileEntry {
            id: "effects:test".to_owned(),
            name: "Test Effects".to_owned(),
            source: "Test".to_owned(),
            read_only: false,
            outfx_enabled: true,
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
}
