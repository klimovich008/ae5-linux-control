use std::io;
use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipeWireNode {
    pub id: u32,
    pub node_name: String,
    pub description: String,
    pub is_default: bool,
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
}
