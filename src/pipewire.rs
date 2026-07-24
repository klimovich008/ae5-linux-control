use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const NATIVE_RATES_CONFIG: &str = "\
# Managed by AE-5 Control.
context.properties = {
    default.clock.allowed-rates = [ 44100 48000 96000 ]
}
";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipeWireNode {
    pub id: u32,
    pub node_name: String,
    pub description: String,
    pub is_default: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeRatesConfig {
    pub path: PathBuf,
    pub enabled: bool,
}

pub fn ae5_output(card_index: i32) -> io::Result<Option<PipeWireNode>> {
    ae5_node(card_index, "sinks")
}

pub fn ae5_input(card_index: i32) -> io::Result<Option<PipeWireNode>> {
    ae5_node(card_index, "sources")
}

pub fn set_ae5_default_output(card_index: i32) -> io::Result<PipeWireNode> {
    set_ae5_default_node(card_index, "sinks", "playback output")
}

pub fn set_ae5_default_input(card_index: i32) -> io::Result<PipeWireNode> {
    set_ae5_default_node(card_index, "sources", "recording input")
}

pub fn native_rates_config() -> io::Result<NativeRatesConfig> {
    native_rates_config_at(&native_rates_path()?)
}

pub fn set_native_rates_enabled(enabled: bool) -> io::Result<NativeRatesConfig> {
    let path = native_rates_path()?;
    let current = native_rates_config_at(&path)?;
    if current.enabled == enabled {
        return Ok(current);
    }

    if enabled {
        fs::create_dir_all(path.parent().expect("rate config has a parent"))?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        file.write_all(NATIVE_RATES_CONFIG.as_bytes())?;
        file.sync_all()?;
    } else {
        fs::remove_file(&path)?;
    }
    native_rates_config_at(&path)
}

fn native_rates_path() -> io::Result<PathBuf> {
    if let Some(path) = env::var_os("XDG_CONFIG_HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return Ok(path.join("pipewire/pipewire.conf.d/91-ae5-control-rates.conf"));
    }
    env::var_os("HOME")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(".config/pipewire/pipewire.conf.d/91-ae5-control-rates.conf"))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "neither XDG_CONFIG_HOME nor HOME is available",
            )
        })
}

fn native_rates_config_at(path: &Path) -> io::Result<NativeRatesConfig> {
    match fs::read_to_string(path) {
        Ok(contents) if contents == NATIVE_RATES_CONFIG => Ok(NativeRatesConfig {
            path: path.to_owned(),
            enabled: true,
        }),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "{} exists but is not managed by AE-5 Control",
                path.display()
            ),
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(NativeRatesConfig {
            path: path.to_owned(),
            enabled: false,
        }),
        Err(error) => Err(error),
    }
}

fn ae5_node(card_index: i32, nodes: &str) -> io::Result<Option<PipeWireNode>> {
    let mut fallback = None;
    for listing in parse_node_list(&run_wpctl(&["list", "audio", nodes])?) {
        let details = run_wpctl(&["inspect", &listing.id.to_string()])?;
        if property(&details, "alsa.card").and_then(|value| value.parse().ok()) != Some(card_index)
        {
            continue;
        }
        let Some(node_name) = property(&details, "node.name") else {
            continue;
        };
        let node = PipeWireNode {
            id: listing.id,
            description: property(&details, "node.description")
                .unwrap_or_else(|| node_name.clone()),
            node_name,
            is_default: listing.is_default,
        };
        if property(&details, "alsa.device").as_deref() == Some("0") {
            return Ok(Some(node));
        }
        fallback.get_or_insert(node);
    }
    Ok(fallback)
}

fn set_ae5_default_node(
    card_index: i32,
    nodes: &str,
    description: &str,
) -> io::Result<PipeWireNode> {
    let node = ae5_node(card_index, nodes)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("PipeWire has no {description} for ALSA card {card_index}"),
        )
    })?;
    if node.is_default {
        return Ok(node);
    }
    run_wpctl(&["set-default", &node.id.to_string()])?;
    ae5_node(card_index, nodes)?
        .filter(|node| node.is_default)
        .ok_or_else(|| io::Error::other(format!("PipeWire did not retain the AE-5 {description}")))
}

fn run_wpctl(arguments: &[&str]) -> io::Result<String> {
    let output = Command::new("wpctl")
        .args(arguments)
        .output()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "wpctl is unavailable; install WirePlumber",
                )
            } else {
                error
            }
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(io::Error::other(if detail.is_empty() {
            format!("wpctl {} failed", arguments.join(" "))
        } else {
            detail
        }));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NodeListing {
    id: u32,
    is_default: bool,
}

fn parse_node_list(output: &str) -> Vec<NodeListing> {
    output
        .lines()
        .filter_map(|line| {
            let id = line.split_whitespace().next()?.parse().ok()?;
            Some(NodeListing {
                id,
                is_default: line.split_whitespace().last() == Some("*"),
            })
        })
        .collect()
}

fn property(output: &str, name: &str) -> Option<String> {
    let prefix = format!("{name} = ");
    output.lines().find_map(|line| {
        line.trim()
            .strip_prefix("* ")
            .unwrap_or(line.trim())
            .strip_prefix(&prefix)
            .map(|value| value.trim_matches('"').to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parses_wpctl_node_identity_and_default_marker() {
        let listing = "\
57\talsa_output.pci-hdmi\taudio/sink\t \n\
58\talsa_output.pci-ae5\taudio/sink\t*\n";
        assert_eq!(
            parse_node_list(listing),
            vec![
                NodeListing {
                    id: 57,
                    is_default: false,
                },
                NodeListing {
                    id: 58,
                    is_default: true,
                },
            ]
        );

        let details = r#"
id 58, type PipeWire:Interface:Node
    alsa.card = "1"
    alsa.device = "0"
  * node.description = "Creative Sound BlasterX AE-5"
  * node.name = "alsa_output.pci-ae5.analog-stereo"
"#;
        assert_eq!(property(details, "alsa.card").as_deref(), Some("1"));
        assert_eq!(
            property(details, "node.name").as_deref(),
            Some("alsa_output.pci-ae5.analog-stereo")
        );
    }

    #[test]
    fn native_rate_config_is_idempotent_and_refuses_foreign_content() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = env::temp_dir().join(format!(
            "ae5-control-rate-test-{}-{unique}",
            std::process::id()
        ));
        let path = directory.join("91-ae5-control-rates.conf");

        assert!(!native_rates_config_at(&path).unwrap().enabled);
        fs::create_dir_all(&directory).unwrap();
        fs::write(&path, NATIVE_RATES_CONFIG).unwrap();
        assert!(native_rates_config_at(&path).unwrap().enabled);
        fs::write(&path, "user configuration\n").unwrap();
        assert_eq!(
            native_rates_config_at(&path).unwrap_err().kind(),
            io::ErrorKind::AlreadyExists
        );

        fs::remove_dir_all(directory).unwrap();
    }
}
