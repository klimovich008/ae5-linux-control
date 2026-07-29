use crate::ControlSnapshot;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

const FORMAT_VERSION: u32 = 1;
const MAX_CAPTURE_BYTES: u64 = 128 * 1024;
const SCALAR_TOLERANCE: f64 = 0.001;
const DB_TOLERANCE: f64 = 0.05;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WindowsVolumePoint {
    pub percent: u8,
    pub requested_scalar: f64,
    pub readback_scalar: f64,
    pub readback_db: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WindowsVolumeCurve {
    pub format_version: u32,
    pub endpoint_id: String,
    pub endpoint_name: String,
    pub role: String,
    pub output: String,
    pub range_min_db: f64,
    pub range_max_db: f64,
    pub range_increment_db: f64,
    pub hardware_support_mask: u32,
    pub restore_verified: bool,
    pub points: Vec<WindowsVolumePoint>,
}

impl WindowsVolumeCurve {
    pub fn load(path: &Path) -> Result<Self, VolumeCurveError> {
        let file = fs::File::open(path)?;
        if file.metadata()?.len() > MAX_CAPTURE_BYTES {
            return Err(VolumeCurveError::Invalid(
                "Windows volume-curve capture is too large".to_owned(),
            ));
        }
        let mut contents = Vec::new();
        file.take(MAX_CAPTURE_BYTES + 1)
            .read_to_end(&mut contents)?;
        if contents.len() as u64 > MAX_CAPTURE_BYTES {
            return Err(VolumeCurveError::Invalid(
                "Windows volume-curve capture is too large".to_owned(),
            ));
        }
        let curve = serde_json::from_slice::<Self>(&contents)?;
        curve.validate()?;
        Ok(curve)
    }

    pub fn pipewire_percent(&self, windows_percent: f64) -> Result<f64, VolumeCurveError> {
        validate_percent(windows_percent, "Windows")?;
        self.validate()?;
        if windows_percent == 0.0 {
            return Ok(0.0);
        }
        let attenuation_db = interpolate(
            &self.points,
            windows_percent,
            |point| f64::from(point.percent),
            |point| point.readback_db,
        );
        Ok((100.0 * 10.0_f64.powf(attenuation_db / 60.0)).min(100.0))
    }

    pub fn windows_percent(&self, pipewire_percent: f64) -> Result<f64, VolumeCurveError> {
        validate_percent(pipewire_percent, "PipeWire")?;
        self.validate()?;
        if pipewire_percent == 0.0 {
            return Ok(0.0);
        }
        let attenuation_db = 60.0 * (pipewire_percent / 100.0).log10();
        let first = self.points.first().expect("validated curve has points");
        let last = self.points.last().expect("validated curve has points");
        if attenuation_db <= first.readback_db {
            return Ok(f64::from(first.percent));
        }
        if attenuation_db >= last.readback_db {
            return Ok(f64::from(last.percent));
        }
        Ok(interpolate(
            &self.points,
            attenuation_db,
            |point| point.readback_db,
            |point| f64::from(point.percent),
        ))
    }

    pub fn validate(&self) -> Result<(), VolumeCurveError> {
        if self.format_version != FORMAT_VERSION {
            return Err(VolumeCurveError::Invalid(format!(
                "unsupported Windows volume-curve format version {}",
                self.format_version
            )));
        }
        if self.endpoint_id.trim().is_empty() {
            return Err(VolumeCurveError::Invalid(
                "Windows endpoint ID is empty".to_owned(),
            ));
        }
        if !self.endpoint_name.to_ascii_lowercase().contains("ae-5") {
            return Err(VolumeCurveError::Invalid(format!(
                "Windows endpoint '{}' is not identified as an AE-5",
                self.endpoint_name
            )));
        }
        if self.role != "multimedia" {
            return Err(VolumeCurveError::Invalid(format!(
                "Windows endpoint role is '{}', expected 'multimedia'",
                self.role
            )));
        }
        if !matches!(self.output.as_str(), "Headphone" | "Speakers") {
            return Err(VolumeCurveError::Invalid(format!(
                "Windows capture output is '{}', expected 'Headphone' or 'Speakers'",
                self.output
            )));
        }
        if !self.restore_verified {
            return Err(VolumeCurveError::Invalid(
                "Windows capture did not verify that volume and mute were restored".to_owned(),
            ));
        }
        if !self.range_min_db.is_finite()
            || !self.range_max_db.is_finite()
            || !self.range_increment_db.is_finite()
            || self.range_min_db >= self.range_max_db
            || self.range_increment_db <= 0.0
            || self.range_max_db > DB_TOLERANCE
        {
            return Err(VolumeCurveError::Invalid(
                "Windows endpoint returned an invalid or amplifying dB range".to_owned(),
            ));
        }
        if self.points.len() != 101 {
            return Err(VolumeCurveError::Invalid(
                "Windows volume curve must contain every integer point from 0% through 100%"
                    .to_owned(),
            ));
        }
        if self.points.first().map(|point| point.percent) != Some(0)
            || self.points.last().map(|point| point.percent) != Some(100)
        {
            return Err(VolumeCurveError::Invalid(
                "Windows volume curve must include 0% and 100% endpoints".to_owned(),
            ));
        }

        let endpoint_tolerance = self.range_increment_db + DB_TOLERANCE;
        if (self.points[0].readback_db - self.range_min_db).abs() > endpoint_tolerance
            || (self
                .points
                .last()
                .expect("validated curve has a last point")
                .readback_db
                - self.range_max_db)
                .abs()
                > endpoint_tolerance
        {
            return Err(VolumeCurveError::Invalid(
                "Windows curve endpoints do not match the reported dB range".to_owned(),
            ));
        }

        for (index, point) in self.points.iter().enumerate() {
            if usize::from(point.percent) != index {
                return Err(VolumeCurveError::Invalid(
                    "Windows volume curve must contain each integer percentage exactly once"
                        .to_owned(),
                ));
            }
            let expected_scalar = f64::from(point.percent) / 100.0;
            if !point.requested_scalar.is_finite()
                || !point.readback_scalar.is_finite()
                || !point.readback_db.is_finite()
                || (point.requested_scalar - expected_scalar).abs() > SCALAR_TOLERANCE
                || !(0.0..=1.0).contains(&point.readback_scalar)
                || (point.readback_scalar - expected_scalar).abs() > SCALAR_TOLERANCE
                || point.readback_db < self.range_min_db - endpoint_tolerance
                || point.readback_db > self.range_max_db + endpoint_tolerance
            {
                return Err(VolumeCurveError::Invalid(format!(
                    "Windows volume point {index} has an invalid scalar or dB value"
                )));
            }
            if let Some(previous) = index.checked_sub(1).map(|index| &self.points[index])
                && (point.percent <= previous.percent
                    || point.requested_scalar <= previous.requested_scalar
                    || point.readback_scalar <= previous.readback_scalar
                    || point.readback_db < previous.readback_db)
            {
                return Err(VolumeCurveError::Invalid(
                    "Windows volume points are not monotonic".to_owned(),
                ));
            }
        }
        Ok(())
    }

    pub fn validate_linux_path(
        &self,
        controls: &[ControlSnapshot],
    ) -> Result<(), VolumeCurveError> {
        self.validate()?;
        let selected_output = controls
            .iter()
            .find(|control| control.name == "Output Select")
            .and_then(|control| control.selected.as_deref())
            .ok_or_else(|| {
                VolumeCurveError::Invalid(
                    "live AE-5 output selection is unavailable; volume was not changed".to_owned(),
                )
            })?;
        if selected_output != self.output {
            return Err(VolumeCurveError::Invalid(format!(
                "the Windows curve was captured for {}, but Linux currently selects {selected_output}",
                self.output
            )));
        }
        require_switch(controls, "Enable OutFX", false)?;
        require_fixed_stage(controls, "Master", 99, 99, true, false)?;
        require_fixed_stage(controls, "Front", 90, 99, true, true)?;
        require_fixed_stage(controls, "PCM", 255, 255, false, true)
    }
}

#[derive(Debug)]
pub enum VolumeCurveError {
    Io(io::Error),
    Json(serde_json::Error),
    Invalid(String),
}

impl fmt::Display for VolumeCurveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => error.fmt(formatter),
            Self::Json(error) => write!(formatter, "invalid Windows volume-curve JSON: {error}"),
            Self::Invalid(error) => formatter.write_str(error),
        }
    }
}

impl std::error::Error for VolumeCurveError {}

impl From<io::Error> for VolumeCurveError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for VolumeCurveError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

fn validate_percent(percent: f64, scale: &str) -> Result<(), VolumeCurveError> {
    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
        return Err(VolumeCurveError::Invalid(format!(
            "{scale} volume must be between 0 and 100 percent"
        )));
    }
    Ok(())
}

fn require_switch(
    controls: &[ControlSnapshot],
    name: &str,
    expected: bool,
) -> Result<(), VolumeCurveError> {
    let actual = controls
        .iter()
        .find(|control| control.name == name)
        .and_then(|control| control.playback_switch)
        .ok_or_else(|| {
            VolumeCurveError::Invalid(format!(
                "live {name} playback switch is unavailable; volume was not changed"
            ))
        })?;
    if actual != expected {
        return Err(VolumeCurveError::Invalid(format!(
            "{name} must be {} before applying the calibrated software-volume curve",
            if expected { "on" } else { "off" }
        )));
    }
    Ok(())
}

fn require_fixed_stage(
    controls: &[ControlSnapshot],
    name: &str,
    expected_value: i64,
    expected_max: i64,
    require_unmuted: bool,
    require_channels: bool,
) -> Result<(), VolumeCurveError> {
    let control = controls
        .iter()
        .find(|control| control.name == name)
        .ok_or_else(|| {
            VolumeCurveError::Invalid(format!(
                "live {name} playback stage is unavailable; volume was not changed"
            ))
        })?;
    let level = control.playback_level.as_ref().ok_or_else(|| {
        VolumeCurveError::Invalid(format!(
            "live {name} playback level is unavailable; volume was not changed"
        ))
    })?;
    if level.value != expected_value || level.max != expected_max {
        return Err(VolumeCurveError::Invalid(format!(
            "{name} is {}/{}, expected the fixed 0 dB stage {expected_value}/{expected_max}; \
             volume was not changed",
            level.value, level.max
        )));
    }
    if require_unmuted && control.playback_switch != Some(true) {
        return Err(VolumeCurveError::Invalid(format!(
            "{name} must be unmuted at its fixed 0 dB stage; volume was not changed"
        )));
    }
    if require_channels
        && (control.playback_channels.is_empty()
            || control
                .playback_channels
                .iter()
                .any(|channel| channel.value != expected_value))
    {
        return Err(VolumeCurveError::Invalid(format!(
            "every {name} playback channel must be fixed at {expected_value}; volume was not changed"
        )));
    }
    Ok(())
}

fn interpolate(
    points: &[WindowsVolumePoint],
    input: f64,
    input_of: impl Fn(&WindowsVolumePoint) -> f64,
    output_of: impl Fn(&WindowsVolumePoint) -> f64,
) -> f64 {
    let upper_index = points
        .iter()
        .position(|point| input_of(point) >= input)
        .unwrap_or(points.len() - 1);
    if upper_index == 0 {
        return output_of(&points[0]);
    }
    let lower = &points[upper_index - 1];
    let upper = &points[upper_index];
    let lower_input = input_of(lower);
    let upper_input = input_of(upper);
    if upper_input == lower_input {
        return output_of(upper);
    }
    let position = (input - lower_input) / (upper_input - lower_input);
    output_of(lower) + position * (output_of(upper) - output_of(lower))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn curve() -> WindowsVolumeCurve {
        WindowsVolumeCurve {
            format_version: FORMAT_VERSION,
            endpoint_id: "{0.0.0.00000000}.{ae5}".to_owned(),
            endpoint_name: "Speakers (Sound BlasterX AE-5)".to_owned(),
            role: "multimedia".to_owned(),
            output: "Headphone".to_owned(),
            range_min_db: -96.0,
            range_max_db: 0.0,
            range_increment_db: 0.25,
            hardware_support_mask: 0,
            restore_verified: true,
            points: (0..=100)
                .map(|percent| {
                    let scalar = f64::from(percent) / 100.0;
                    WindowsVolumePoint {
                        percent,
                        requested_scalar: scalar,
                        readback_scalar: scalar,
                        readback_db: if percent == 0 {
                            -96.0
                        } else {
                            40.0 * scalar.log10()
                        },
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn validates_a_restored_ae5_curve_with_ordered_points() {
        curve().validate().unwrap();
    }

    #[test]
    fn rejects_a_capture_that_did_not_restore_windows_state() {
        let mut curve = curve();
        curve.restore_verified = false;
        assert!(
            curve
                .validate()
                .unwrap_err()
                .to_string()
                .contains("restore")
        );
    }

    #[test]
    fn rejects_non_monotonic_decibel_points() {
        let mut curve = curve();
        curve.points[2].readback_db = -30.0;
        assert!(
            curve
                .validate()
                .unwrap_err()
                .to_string()
                .contains("monotonic")
        );
    }

    #[test]
    fn rejects_an_incomplete_integer_curve() {
        let mut curve = curve();
        curve.points.remove(50);
        assert!(
            curve
                .validate()
                .unwrap_err()
                .to_string()
                .contains("every integer point")
        );
    }

    #[test]
    fn maps_windows_attenuation_to_pipewire_cubic_volume() {
        let curve = curve();
        let expected = 100.0 * 10.0_f64.powf(curve.points[20].readback_db / 60.0);
        assert!((curve.pipewire_percent(20.0).unwrap() - expected).abs() < 1e-9);
        assert_eq!(curve.pipewire_percent(0.0).unwrap(), 0.0);
        assert_eq!(curve.pipewire_percent(100.0).unwrap(), 100.0);
    }

    #[test]
    fn interpolates_in_decibels_and_maps_back_to_windows_percent() {
        let curve = curve();
        let expected_db = (curve.points[25].readback_db + curve.points[26].readback_db) / 2.0;
        let expected_pipewire = 100.0 * 10.0_f64.powf(expected_db / 60.0);
        let pipewire = curve.pipewire_percent(25.5).unwrap();
        assert!((pipewire - expected_pipewire).abs() < 1e-9);
        assert!((curve.windows_percent(pipewire).unwrap() - 25.5).abs() < 1e-9);
    }

    #[test]
    fn rejects_percentages_outside_the_user_scale() {
        let curve = curve();
        assert!(curve.pipewire_percent(-0.1).is_err());
        assert!(curve.pipewire_percent(100.1).is_err());
        assert!(curve.windows_percent(-0.1).is_err());
        assert!(curve.windows_percent(100.1).is_err());
    }

    #[test]
    fn loads_and_validates_a_json_capture() {
        let path = std::env::temp_dir().join(format!(
            "ae5-volume-curve-{}-{}.json",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let json = serde_json::to_vec(&curve()).unwrap();
        fs::write(&path, json).unwrap();

        let loaded = WindowsVolumeCurve::load(&path).unwrap();

        fs::remove_file(path).unwrap();
        assert_eq!(loaded.endpoint_id, curve().endpoint_id);
        assert_eq!(loaded.output, "Headphone");
        assert_eq!(loaded.points.len(), 101);
        assert!(
            (loaded.pipewire_percent(20.0).unwrap() - curve().pipewire_percent(20.0).unwrap())
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn accepts_only_the_matching_single_stage_linux_path() {
        let controls = vec![
            choice("Output Select", "Headphone"),
            switch("Enable OutFX", false),
            level("Master", 99, 99, true, false),
            level("Front", 90, 99, true, true),
            level("PCM", 255, 255, false, true),
        ];

        curve().validate_linux_path(&controls).unwrap();
    }

    #[test]
    fn rejects_stacked_hardware_attenuation() {
        let controls = vec![
            choice("Output Select", "Headphone"),
            switch("Enable OutFX", false),
            level("Master", 19, 99, true, false),
            level("Front", 19, 99, true, true),
            level("PCM", 51, 255, false, true),
        ];

        assert!(
            curve()
                .validate_linux_path(&controls)
                .unwrap_err()
                .to_string()
                .contains("fixed 0 dB")
        );
    }

    #[test]
    fn rejects_a_curve_captured_for_another_output() {
        let controls = vec![
            choice("Output Select", "Speakers"),
            switch("Enable OutFX", false),
            level("Master", 99, 99, true, false),
            level("Front", 90, 99, true, true),
            level("PCM", 255, 255, false, true),
        ];

        assert!(
            curve()
                .validate_linux_path(&controls)
                .unwrap_err()
                .to_string()
                .contains("captured for Headphone")
        );
    }

    fn choice(name: &str, selected: &str) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: Some(selected.to_owned()),
            choices: Vec::new(),
            playback_switch: None,
            capture_switch: None,
            playback_level: None,
            capture_level: None,
            playback_channels: Vec::new(),
            capture_channels: Vec::new(),
        }
    }

    fn switch(name: &str, enabled: bool) -> ControlSnapshot {
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

    fn level(
        name: &str,
        value: i64,
        max: i64,
        unmuted: bool,
        with_channels: bool,
    ) -> ControlSnapshot {
        ControlSnapshot {
            name: name.to_owned(),
            selected: None,
            choices: Vec::new(),
            playback_switch: unmuted.then_some(true),
            capture_switch: None,
            playback_level: Some(crate::Level {
                value,
                min: 0,
                max,
                db: None,
            }),
            capture_level: None,
            playback_channels: if with_channels {
                ["Front Left", "Front Right"]
                    .map(|name| crate::ChannelLevel {
                        name: name.to_owned(),
                        value,
                    })
                    .to_vec()
            } else {
                Vec::new()
            },
            capture_channels: Vec::new(),
        }
    }
}
