use crate::{
    Ae5Device, DeviceOutputState, EffectsProfileEntry, EqPresetEntry, SoundObjectCatalog,
    save_effects_profile, save_effects_profile_as, save_eq_preset, save_eq_preset_as,
    set_ae5_software_mute, set_ae5_software_volume, sound_object_catalog,
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
    fn set_master_volume(&self, percent: u16) -> zbus::Result<DeviceOutputState>;
    fn set_muted(&self, muted: bool) -> zbus::Result<DeviceOutputState>;
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

pub fn write_master_volume(percent: u16) -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.set_master_volume(percent)
}

pub fn write_muted(muted: bool) -> zbus::Result<DeviceOutputState> {
    let connection = zbus::blocking::Connection::session()?;
    Ae5DeviceProxy::new(&connection)?.set_muted(muted)
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
}
