use ae5_control::{
    Ae5Device, Ae5Mixer, PipeWireNode, Profile, SbCommandImportReport, SbCommandTarget, ae5_input,
    ae5_output, import_sbcommand_profile_with_report, set_ae5_default_input,
    set_ae5_default_output, snapshot_controls,
};
use std::error::Error;
use std::io;
use std::path::Path;

const IMPORTANT_CONTROLS: &[&str] = &[
    "Output Select",
    "HP/Speaker Auto Detect",
    "AE-5: Headphone Gain",
    "AE-5: Sound Filter",
    "Surround Channel Config",
    "Input Source",
    "Enable OutFX",
];

fn main() {
    if let Err(error) = run() {
        eprintln!("ae5ctl: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => print_status(),
        [command] if command == "status" => print_status(),
        [command] if command == "controls" => print_controls(),
        [command] if command == "output-status" => print_output_status(),
        [command] if command == "set-default-output" => set_default_output(),
        [command] if command == "input-status" => print_input_status(),
        [command] if command == "set-default-input" => set_default_input(),
        [command, name] if command == "get" => print_control(name),
        [command, name, choice] if command == "set-choice" => set_choice(name, choice, false),
        [command, name, choice, flag] if command == "set-choice" && flag == "--allow-high-gain" => {
            set_choice(name, choice, true)
        }
        [command, name, value] if command == "set-playback-switch" => {
            set_playback_switch(name, value)
        }
        [command, name, value] if command == "set-capture-switch" => {
            set_capture_switch(name, value)
        }
        [command, name, value] if command == "set-playback-level" => {
            set_playback_level(name, value)
        }
        [command, name, value] if command == "set-capture-level" => set_capture_level(name, value),
        [command, name, channel, value] if command == "set-playback-channel-level" => {
            set_playback_channel_level(name, channel, value)
        }
        [command, name, channel, value] if command == "set-capture-channel-level" => {
            set_capture_channel_level(name, channel, value)
        }
        [command] if command == "smoke-test" => smoke_test(),
        [command, name, path] if command == "profile-save" => save_profile(name, path),
        [command, path] if command == "profile-show" => show_profile(path),
        [command, path] if command == "profile-check" => check_profile(path, false),
        [command, path, flag] if command == "profile-check" && flag == "--allow-high-gain" => {
            check_profile(path, true)
        }
        [command, path] if command == "profile-apply" => apply_profile(path, false),
        [command, path, flag] if command == "profile-apply" && flag == "--allow-high-gain" => {
            apply_profile(path, true)
        }
        [command, name, profile, eq, target, output] if command == "sbcommand-import" => {
            import_sbcommand(name, profile, eq, target, output)
        }
        [command] if matches!(command.as_str(), "-h" | "--help" | "help") => {
            print_help();
            Ok(())
        }
        [command] if matches!(command.as_str(), "-V" | "--version") => {
            println!("ae5ctl {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid arguments; run 'ae5ctl --help'",
        )
        .into()),
    }
}

fn print_status() -> Result<(), Box<dyn Error>> {
    let device = require_device()?;
    let controls = snapshot_controls(device.card_index)?;

    println!("Sound BlasterX AE-5 detected");
    println!("  ALSA card: {} ({})", device.card_index, device.alsa_name);
    println!("  ALSA device: {}", device.alsa_long_name);
    println!("  PCI ID: {}", device.pci_id());
    println!("  Subsystem ID: {}", device.subsystem_id());
    if let Some(codec_name) = &device.codec_name {
        println!("  Codec: {codec_name}");
    }
    println!("  Simple controls: {}", controls.len());
    match ae5_output(device.card_index) {
        Ok(Some(output)) => print_pipewire_node("output", &output),
        Ok(None) => println!("  PipeWire output: unavailable"),
        Err(error) => println!("  PipeWire output: unavailable ({error})"),
    }
    match ae5_input(device.card_index) {
        Ok(Some(input)) => print_pipewire_node("input", &input),
        Ok(None) => println!("  PipeWire input: unavailable"),
        Err(error) => println!("  PipeWire input: unavailable ({error})"),
    }
    println!();
    println!("Core control state");

    for name in IMPORTANT_CONTROLS {
        match controls.iter().find(|control| control.name == *name) {
            Some(control) => println!("  {control}"),
            None => println!("  {name}: unavailable"),
        }
    }
    Ok(())
}

fn print_output_status() -> Result<(), Box<dyn Error>> {
    let device = require_device()?;
    let output = ae5_output(device.card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "PipeWire has no playback output for ALSA card {}",
                device.card_index
            ),
        )
    })?;
    print_pipewire_node("output", &output);
    Ok(())
}

fn set_default_output() -> Result<(), Box<dyn Error>> {
    let device = require_device()?;
    let output = set_ae5_default_output(device.card_index)?;
    println!(
        "AE-5 is now the PipeWire default output: {} ({})",
        output.description, output.node_name
    );
    Ok(())
}

fn print_input_status() -> Result<(), Box<dyn Error>> {
    let device = require_device()?;
    let input = ae5_input(device.card_index)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "PipeWire has no recording input for ALSA card {}",
                device.card_index
            ),
        )
    })?;
    print_pipewire_node("input", &input);
    Ok(())
}

fn set_default_input() -> Result<(), Box<dyn Error>> {
    let device = require_device()?;
    let input = set_ae5_default_input(device.card_index)?;
    println!(
        "AE-5 is now the PipeWire default input: {} ({})",
        input.description, input.node_name
    );
    Ok(())
}

fn print_pipewire_node(kind: &str, node: &PipeWireNode) {
    println!(
        "  PipeWire {kind}: {} ({}){}",
        node.description,
        node.node_name,
        if node.is_default {
            " [default]"
        } else {
            " [not default]"
        }
    );
}

fn print_controls() -> Result<(), Box<dyn Error>> {
    let device = require_device()?;
    for control in snapshot_controls(device.card_index)? {
        println!("{control}");
        if !control.choices.is_empty() {
            println!("  choices: {}", control.choices.join(", "));
        }
    }
    Ok(())
}

fn print_control(name: &str) -> Result<(), Box<dyn Error>> {
    println!("{}", mixer()?.snapshot(name)?);
    Ok(())
}

fn set_choice(name: &str, choice: &str, allow_high_gain: bool) -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        mixer()?.set_choice_checked(name, choice, allow_high_gain)?
    );
    Ok(())
}

fn set_playback_switch(name: &str, value: &str) -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        mixer()?.set_playback_switch(name, parse_switch(value)?)?
    );
    Ok(())
}

fn set_capture_switch(name: &str, value: &str) -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        mixer()?.set_capture_switch(name, parse_switch(value)?)?
    );
    Ok(())
}

fn set_playback_level(name: &str, value: &str) -> Result<(), Box<dyn Error>> {
    println!("{}", mixer()?.set_playback_level(name, value.parse()?)?);
    Ok(())
}

fn set_capture_level(name: &str, value: &str) -> Result<(), Box<dyn Error>> {
    println!("{}", mixer()?.set_capture_level(name, value.parse()?)?);
    Ok(())
}

fn set_playback_channel_level(
    name: &str,
    channel: &str,
    value: &str,
) -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        mixer()?.set_playback_channel_level(name, channel, value.parse()?)?
    );
    Ok(())
}

fn set_capture_channel_level(name: &str, channel: &str, value: &str) -> Result<(), Box<dyn Error>> {
    println!(
        "{}",
        mixer()?.set_capture_channel_level(name, channel, value.parse()?)?
    );
    Ok(())
}

fn smoke_test() -> Result<(), Box<dyn Error>> {
    let mixer = mixer()?;
    let candidates = [
        ("FX: Surround", "FX: Surround"),
        ("Bass Redirection", "Bass Redirection Crossover"),
    ];

    for (switch_name, level_name) in candidates {
        if mixer.snapshot(switch_name)?.playback_switch != Some(false) {
            continue;
        }
        let original = mixer
            .snapshot(level_name)?
            .playback_level
            .ok_or_else(|| missing_level(level_name))?;
        let changed = if original.value < original.max {
            original.value + 1
        } else if original.value > original.min {
            original.value - 1
        } else {
            continue;
        };

        let change_result = mixer.set_playback_level(level_name, changed);
        let restore_result = mixer.set_playback_level(level_name, original.value);
        change_result?;
        restore_result?;
        println!(
            "passed: '{level_name}' changed {} -> {changed} -> {} while '{switch_name}' was off",
            original.value, original.value
        );
        return Ok(());
    }

    Err(io::Error::other("no disabled effect was available for a safe smoke test").into())
}

fn save_profile(name: &str, path: &str) -> Result<(), Box<dyn Error>> {
    let device = require_device()?;
    let profile = Profile::capture(name, snapshot_controls(device.card_index)?)?;
    profile.save_new(Path::new(path))?;
    println!(
        "saved '{}' with {} controls to {path}",
        profile.name,
        profile.controls.len()
    );
    Ok(())
}

fn show_profile(path: &str) -> Result<(), Box<dyn Error>> {
    let profile = Profile::load(Path::new(path))?;
    println!("Profile: {}", profile.name);
    println!("Target: {}", profile.target);
    println!("Controls: {}", profile.controls.len());
    Ok(())
}

fn check_profile(path: &str, allow_high_gain: bool) -> Result<(), Box<dyn Error>> {
    let profile = Profile::load(Path::new(path))?;
    profile.check(&mixer()?, allow_high_gain)?;
    println!(
        "valid: '{}' contains {} applicable controls",
        profile.name,
        profile.controls.len()
    );
    Ok(())
}

fn apply_profile(path: &str, allow_high_gain: bool) -> Result<(), Box<dyn Error>> {
    let profile = Profile::load(Path::new(path))?;
    let report = profile.apply(&mixer()?, allow_high_gain)?;
    println!(
        "applied '{}' ({} controls verified)",
        profile.name, report.controls_applied
    );
    Ok(())
}

fn import_sbcommand(
    name: &str,
    profile_path: &str,
    eq_path: &str,
    target: &str,
    output: &str,
) -> Result<(), Box<dyn Error>> {
    let target = target.parse::<SbCommandTarget>()?;
    let import = import_sbcommand_profile_with_report(
        name,
        Path::new(profile_path),
        Path::new(eq_path),
        target,
    )?;
    print_import_report(&import.report);
    import.profile.save_new(Path::new(output))?;
    println!(
        "converted Sound Blaster Command {target} settings to '{}' ({} controls) at {output}",
        import.profile.name,
        import.profile.controls.len()
    );
    Ok(())
}

fn print_import_report(report: &SbCommandImportReport) {
    println!("Migration report");
    print_report_section("Exact", &report.exact);
    print_report_section("Approximate", &report.approximate);
    print_report_section("Unsupported (skipped)", &report.unsupported);
}

fn print_report_section(title: &str, items: &[String]) {
    println!("  {title}: {}", items.len());
    for item in items {
        println!("    - {item}");
    }
}

fn mixer() -> Result<Ae5Mixer, Box<dyn Error>> {
    Ok(Ae5Mixer::open(require_device()?.card_index)?)
}

fn parse_switch(value: &str) -> Result<bool, io::Error> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "switch value must be 'on' or 'off'",
        )),
    }
}

fn missing_level(name: &str) -> io::Error {
    io::Error::other(format!("'{name}' has no playback level"))
}

fn require_device() -> Result<Ae5Device, Box<dyn Error>> {
    Ae5Device::discover()?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "supported AE-5 (1102:0012, subsystem 1102:0051) was not found",
        )
        .into()
    })
}

fn print_help() {
    println!(
        "Usage: ae5ctl [COMMAND]\n\
         \n\
         Commands:\n\
         \x20 status    Show the detected AE-5 and important live controls (default)\n\
         \x20 controls  Show every live ALSA simple control\n\
         \x20 output-status       Show the AE-5 PipeWire playback target\n\
         \x20 set-default-output  Make the AE-5 the PipeWire default playback target\n\
         \x20 input-status        Show the AE-5 PipeWire recording target\n\
         \x20 set-default-input   Make the AE-5 the PipeWire default recording target\n\
         \x20 get NAME\n\
         \x20 set-choice NAME CHOICE [--allow-high-gain]\n\
         \x20 set-playback-switch NAME on|off\n\
         \x20 set-capture-switch NAME on|off\n\
         \x20 set-playback-level NAME VALUE\n\
         \x20 set-capture-level NAME VALUE\n\
         \x20 set-playback-channel-level NAME CHANNEL VALUE\n\
         \x20 set-capture-channel-level NAME CHANNEL VALUE\n\
         \x20 smoke-test  Safely change, verify, and restore a disabled effect level\n\
         \x20 profile-save NAME FILE\n\
         \x20 profile-show FILE\n\
         \x20 profile-check FILE [--allow-high-gain]\n\
         \x20 profile-apply FILE [--allow-high-gain]\n\
         \x20 sbcommand-import NAME PROFILE_JSON EQ_JSON speaker|headphone OUTPUT\n\
         \x20 help      Show this help"
    );
}
