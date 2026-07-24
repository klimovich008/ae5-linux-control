use alsa::card::Card;
use std::fs;
use std::io;
use std::path::Path;

const CREATIVE_VENDOR_ID: u16 = 0x1102;
const AE5_DEVICE_ID: u16 = 0x0012;
const AE5_SUBSYSTEM_ID: u16 = 0x0051;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ae5Device {
    pub card_index: i32,
    pub alsa_name: String,
    pub alsa_long_name: String,
    pub codec_name: Option<String>,
    pub vendor_id: u16,
    pub device_id: u16,
    pub subsystem_vendor_id: u16,
    pub subsystem_device_id: u16,
}

impl Ae5Device {
    pub fn discover() -> io::Result<Option<Self>> {
        let cards = match fs::read_dir("/sys/class/sound") {
            Ok(cards) => cards,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };

        for entry in cards {
            let entry = entry?;
            let Some(card_index) = parse_card_index(&entry.file_name().to_string_lossy()) else {
                continue;
            };
            let pci = entry.path().join("device");
            let Some(vendor_id) = read_hex(&pci.join("vendor")) else {
                continue;
            };
            let Some(device_id) = read_hex(&pci.join("device")) else {
                continue;
            };
            let Some(subsystem_vendor_id) = read_hex(&pci.join("subsystem_vendor")) else {
                continue;
            };
            let Some(subsystem_device_id) = read_hex(&pci.join("subsystem_device")) else {
                continue;
            };

            if !is_supported_ae5(
                vendor_id,
                device_id,
                subsystem_vendor_id,
                subsystem_device_id,
            ) {
                continue;
            }

            let card = Card::new(card_index);
            return Ok(Some(Self {
                card_index,
                alsa_name: card.get_name().map_err(io::Error::other)?,
                alsa_long_name: card.get_longname().map_err(io::Error::other)?,
                codec_name: read_codec_name(card_index),
                vendor_id,
                device_id,
                subsystem_vendor_id,
                subsystem_device_id,
            }));
        }

        Ok(None)
    }

    pub fn pci_id(&self) -> String {
        format!("{:04x}:{:04x}", self.vendor_id, self.device_id)
    }

    pub fn subsystem_id(&self) -> String {
        format!(
            "{:04x}:{:04x}",
            self.subsystem_vendor_id, self.subsystem_device_id
        )
    }
}

fn read_hex(path: &Path) -> Option<u16> {
    let value = fs::read_to_string(path).ok()?;
    u16::from_str_radix(value.trim().trim_start_matches("0x"), 16).ok()
}

fn read_codec_name(card_index: i32) -> Option<String> {
    let codecs = fs::read_dir(format!("/proc/asound/card{card_index}")).ok()?;
    for entry in codecs.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("codec") {
            continue;
        }
        let contents = fs::read_to_string(entry.path()).ok()?;
        if let Some(name) = contents
            .lines()
            .find_map(|line| line.strip_prefix("Codec: "))
        {
            return Some(name.to_owned());
        }
    }
    None
}

fn parse_card_index(name: &str) -> Option<i32> {
    let index = name.strip_prefix("card")?;
    (!index.is_empty() && index.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| index.parse().ok())
        .flatten()
}

fn is_supported_ae5(
    vendor_id: u16,
    device_id: u16,
    subsystem_vendor_id: u16,
    subsystem_device_id: u16,
) -> bool {
    vendor_id == CREATIVE_VENDOR_ID
        && device_id == AE5_DEVICE_ID
        && subsystem_vendor_id == CREATIVE_VENDOR_ID
        && subsystem_device_id == AE5_SUBSYSTEM_ID
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_alsa_card_directory_names() {
        assert_eq!(parse_card_index("card0"), Some(0));
        assert_eq!(parse_card_index("card12"), Some(12));
        assert_eq!(parse_card_index("controlC1"), None);
        assert_eq!(parse_card_index("card"), None);
        assert_eq!(parse_card_index("card1x"), None);
    }

    #[test]
    fn matches_the_audited_ae5_revision_only() {
        assert!(is_supported_ae5(0x1102, 0x0012, 0x1102, 0x0051));
        assert!(!is_supported_ae5(0x1102, 0x0012, 0x1102, 0x0052));
        assert!(!is_supported_ae5(0x1234, 0x0012, 0x1102, 0x0051));
    }
}
