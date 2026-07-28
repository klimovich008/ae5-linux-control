use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const ONBOARD_LED_COUNT: usize = 5;
const FORMAT_VERSION: u32 = 1;
const TARGET: &str = "1102:0012/1102:0051";
const LED_ROOT: &str = "/sys/class/leds";
const MAX_CONFIG_BYTES: u64 = 4096;
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct RgbColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl RgbColor {
    pub const fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }
}

impl fmt::Display for RgbColor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "#{:02X}{:02X}{:02X}",
            self.red, self.green, self.blue
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct LightingConfig {
    pub format_version: u32,
    pub target: String,
    pub leds: [RgbColor; ONBOARD_LED_COUNT],
}

impl LightingConfig {
    fn new(leds: [RgbColor; ONBOARD_LED_COUNT]) -> Self {
        Self {
            format_version: FORMAT_VERSION,
            target: TARGET.to_owned(),
            leds,
        }
    }

    fn validate(&self) -> io::Result<()> {
        if self.format_version != FORMAT_VERSION {
            return Err(invalid_data(format!(
                "unsupported lighting format version {}",
                self.format_version
            )));
        }
        if self.target != TARGET {
            return Err(invalid_data(format!(
                "lighting targets '{}', expected '{TARGET}'",
                self.target
            )));
        }
        Ok(())
    }

    fn load(path: &Path) -> io::Result<Option<Self>> {
        let file = match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        if file.metadata()?.len() > MAX_CONFIG_BYTES {
            return Err(invalid_data("lighting configuration is too large"));
        }
        let mut contents = Vec::new();
        file.take(MAX_CONFIG_BYTES + 1).read_to_end(&mut contents)?;
        if contents.len() as u64 > MAX_CONFIG_BYTES {
            return Err(invalid_data("lighting configuration is too large"));
        }
        let config = serde_json::from_slice::<Self>(&contents).map_err(invalid_data)?;
        config.validate()?;
        Ok(Some(config))
    }

    fn save(&self, path: &Path) -> io::Result<()> {
        self.validate()?;
        let contents = serde_json::to_vec_pretty(self).map_err(invalid_data)?;
        let parent = path
            .parent()
            .ok_or_else(|| invalid_data("lighting configuration has no parent directory"))?;
        fs::create_dir_all(parent)?;
        let file_name = path
            .file_name()
            .ok_or_else(|| invalid_data("lighting configuration has no file name"))?
            .to_string_lossy();

        let (temporary, mut file) = loop {
            let temporary = parent.join(format!(
                ".{file_name}.{}.{}.tmp",
                std::process::id(),
                NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed)
            ));
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(file) => break (temporary, file),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        };

        let result = (|| {
            file.write_all(&contents)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            drop(file);
            fs::rename(&temporary, path)?;
            fs::File::open(parent)?.sync_all()
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ae5Lighting {
    leds: [PathBuf; ONBOARD_LED_COUNT],
}

impl Ae5Lighting {
    pub fn discover() -> io::Result<Self> {
        Self::discover_at(Path::new(LED_ROOT))
    }

    fn discover_at(root: &Path) -> io::Result<Self> {
        let mut leds = Vec::new();
        for entry in fs::read_dir(root)? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                continue;
            };
            let Some(index) = led_index(name) else {
                continue;
            };
            let path = entry.path();
            if read_trimmed(&path.join("multi_index"))? != "red green blue" {
                return Err(invalid_data(format!(
                    "{} has an unexpected multicolor channel order",
                    path.display()
                )));
            }
            if read_number(&path.join("max_brightness"))? != 255 {
                return Err(invalid_data(format!(
                    "{} has an unexpected maximum brightness",
                    path.display()
                )));
            }
            if leds.iter().any(|(existing, _)| *existing == index) {
                return Err(invalid_data(format!(
                    "duplicate AE-5 onboard LED {}",
                    index + 1
                )));
            }
            leds.push((index, path));
        }
        leds.sort_by_key(|(index, _)| *index);
        if leds.len() != ONBOARD_LED_COUNT
            || leds
                .iter()
                .enumerate()
                .any(|(expected, (actual, _))| expected != *actual)
        {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "expected {ONBOARD_LED_COUNT} AE-5 onboard LEDs from the patched CA0132 driver"
                ),
            ));
        }
        let leds = leds
            .into_iter()
            .map(|(_, path)| path)
            .collect::<Vec<_>>()
            .try_into()
            .map_err(|_| invalid_data("invalid AE-5 onboard LED count"))?;
        Ok(Self { leds })
    }

    pub fn colors(&self) -> io::Result<[RgbColor; ONBOARD_LED_COUNT]> {
        let mut colors = [RgbColor::default(); ONBOARD_LED_COUNT];
        for (color, path) in colors.iter_mut().zip(&self.leds) {
            *color = read_color(path)?;
        }
        Ok(colors)
    }

    pub fn set_colors(&self, colors: [RgbColor; ONBOARD_LED_COUNT]) -> io::Result<()> {
        self.set_colors_with(colors, write_color)
    }

    fn set_colors_with(
        &self,
        colors: [RgbColor; ONBOARD_LED_COUNT],
        mut write: impl FnMut(&Path, RgbColor) -> io::Result<()>,
    ) -> io::Result<()> {
        let before = self.colors()?;
        for (path, color) in self.leds.iter().zip(colors) {
            if let Err(error) = write(path, color) {
                let mut rollback = None;
                for (path, color) in self.leds.iter().zip(before) {
                    match read_color(path) {
                        Ok(actual) if actual == color => {}
                        Ok(_) => {
                            if let Err(error) = write(path, color) {
                                rollback.get_or_insert(error);
                            }
                        }
                        Err(error) => {
                            rollback.get_or_insert(error);
                        }
                    }
                }
                return Err(match rollback {
                    Some(rollback) => io::Error::other(format!(
                        "lighting write failed: {error}; rollback failed: {rollback}"
                    )),
                    None => error,
                });
            }
        }
        Ok(())
    }
}

pub fn lighting_config_path() -> io::Result<PathBuf> {
    let profiles = crate::profile_library::profile_library_directory()?;
    Ok(profiles
        .parent()
        .expect("profile library always has an application directory")
        .join("lighting.json"))
}

pub fn saved_lighting() -> io::Result<Option<LightingConfig>> {
    LightingConfig::load(&lighting_config_path()?)
}

pub fn set_saved_lighting(colors: [RgbColor; ONBOARD_LED_COUNT]) -> io::Result<LightingConfig> {
    let lighting = Ae5Lighting::discover()?;
    let before = lighting.colors()?;
    lighting.set_colors(colors)?;
    let config = LightingConfig::new(colors);
    if let Err(error) = config.save(&lighting_config_path()?) {
        return Err(match lighting.set_colors(before) {
            Ok(()) => error,
            Err(rollback) => io::Error::other(format!(
                "saving lighting failed: {error}; hardware rollback failed: {rollback}"
            )),
        });
    }
    Ok(config)
}

pub fn set_saved_led(index: usize, color: RgbColor) -> io::Result<LightingConfig> {
    if !(1..=ONBOARD_LED_COUNT).contains(&index) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("LED index must be 1 through {ONBOARD_LED_COUNT}"),
        ));
    }
    let lighting = Ae5Lighting::discover()?;
    let before = lighting.colors()?;
    let mut colors = before;
    colors[index - 1] = color;
    lighting.set_colors(colors)?;
    let config = LightingConfig::new(colors);
    if let Err(error) = config.save(&lighting_config_path()?) {
        return Err(match lighting.set_colors(before) {
            Ok(()) => error,
            Err(rollback) => io::Error::other(format!(
                "saving lighting failed: {error}; hardware rollback failed: {rollback}"
            )),
        });
    }
    Ok(config)
}

pub fn restore_saved_lighting() -> io::Result<Option<LightingConfig>> {
    let Some(config) = saved_lighting()? else {
        return Ok(None);
    };
    Ae5Lighting::discover()?.set_colors(config.leds)?;
    Ok(Some(config))
}

fn led_index(name: &str) -> Option<usize> {
    let (prefix, suffix) = name.rsplit_once(":rgb:ae5-")?;
    let index = suffix.parse::<usize>().ok()?;
    (!prefix.is_empty() && suffix == index.to_string() && (1..=ONBOARD_LED_COUNT).contains(&index))
        .then(|| index - 1)
}

fn read_color(path: &Path) -> io::Result<RgbColor> {
    let intensity = read_trimmed(&path.join("multi_intensity"))?
        .split_whitespace()
        .map(|value| {
            value
                .parse::<u16>()
                .map_err(invalid_data)
                .and_then(|value| {
                    u8::try_from(value).map_err(|_| invalid_data("lighting intensity exceeds 255"))
                })
        })
        .collect::<io::Result<Vec<_>>>()?;
    let [red, green, blue]: [u8; 3] = intensity
        .try_into()
        .map_err(|_| invalid_data("lighting intensity must contain red, green, and blue"))?;
    let brightness = u8::try_from(read_number(&path.join("brightness"))?)
        .map_err(|_| invalid_data("lighting brightness exceeds 255"))?;
    let scale = |value: u8| ((u16::from(value) * u16::from(brightness) + 127) / 255) as u8;
    Ok(RgbColor::new(scale(red), scale(green), scale(blue)))
}

fn write_color(path: &Path, color: RgbColor) -> io::Result<()> {
    fs::write(
        path.join("multi_intensity"),
        format!("{} {} {}\n", color.red, color.green, color.blue),
    )?;
    fs::write(path.join("brightness"), b"255\n")?;
    let actual = read_color(path)?;
    if actual != color {
        return Err(invalid_data(format!(
            "{} read back as {actual}, expected {color}",
            path.display()
        )));
    }
    Ok(())
}

fn read_number(path: &Path) -> io::Result<u16> {
    read_trimmed(path)?.parse().map_err(invalid_data)
}

fn read_trimmed(path: &Path) -> io::Result<String> {
    fs::read_to_string(path).map(|value| value.trim().to_owned())
}

fn invalid_data(error: impl fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn discovers_writes_and_reads_the_five_leds() {
        let root = test_directory();
        for index in 1..=ONBOARD_LED_COUNT {
            let path = root.join(format!("hdaudioC0D1:rgb:ae5-{index}"));
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("multi_index"), "red green blue\n").unwrap();
            fs::write(path.join("multi_intensity"), "10 20 30\n").unwrap();
            fs::write(path.join("brightness"), "255\n").unwrap();
            fs::write(path.join("max_brightness"), "255\n").unwrap();
        }

        let lighting = Ae5Lighting::discover_at(&root).unwrap();
        assert_eq!(lighting.colors().unwrap(), [RgbColor::new(10, 20, 30); 5]);

        let colors = [
            RgbColor::new(255, 0, 0),
            RgbColor::new(0, 255, 0),
            RgbColor::new(0, 0, 255),
            RgbColor::new(255, 160, 0),
            RgbColor::new(180, 0, 255),
        ];
        lighting.set_colors(colors).unwrap();
        assert_eq!(lighting.colors().unwrap(), colors);

        let blocked = root.join("hdaudioC0D1:rgb:ae5-3");
        let mut blocked_once = false;
        let error = lighting
            .set_colors_with(
                [RgbColor::new(1, 2, 3); ONBOARD_LED_COUNT],
                |path, color| {
                    if path == blocked && !blocked_once {
                        blocked_once = true;
                        return Err(io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            "injected LED write failure",
                        ));
                    }
                    write_color(path, color)
                },
            )
            .unwrap_err();
        assert!(blocked_once);
        assert!(!error.to_string().contains("rollback failed"));
        assert_eq!(lighting.colors().unwrap(), colors);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn validates_names_channels_and_saved_configuration() {
        assert_eq!(led_index("hdaudioC0D1:rgb:ae5-1"), Some(0));
        assert_eq!(led_index("hdaudioC0D1:rgb:ae5-5"), Some(4));
        assert_eq!(led_index("hdaudioC0D1:rgb:ae5-0"), None);
        assert_eq!(led_index("hdaudioC0D1:rgb:ae5-01"), None);
        assert_eq!(led_index("other"), None);

        let root = test_directory();
        let path = root.join("lighting.json");
        let colors = [RgbColor::new(12, 34, 56); ONBOARD_LED_COUNT];
        let config = LightingConfig::new(colors);
        config.save(&path).unwrap();
        assert_eq!(LightingConfig::load(&path).unwrap(), Some(config));

        fs::write(&path, br#"{"format_version":2,"target":"x","leds":[]}"#).unwrap();
        assert!(LightingConfig::load(&path).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    fn test_directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "ae5-lighting-test-{}-{}",
            std::process::id(),
            NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        path
    }
}
