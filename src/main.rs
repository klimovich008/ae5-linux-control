use ae5_control::{Ae5Device, Ae5Mixer, snapshot_controls};
use std::error::Error;
use std::io;

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
        [command] if command == "smoke-test" => smoke_test(),
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
    if name == "AE-5: Headphone Gain"
        && choice.to_ascii_lowercase().starts_with("high")
        && !allow_high_gain
    {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "high headphone gain requires --allow-high-gain",
        )
        .into());
    }
    println!("{}", mixer()?.set_choice(name, choice)?);
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
         \x20 get NAME\n\
         \x20 set-choice NAME CHOICE [--allow-high-gain]\n\
         \x20 set-playback-switch NAME on|off\n\
         \x20 set-capture-switch NAME on|off\n\
         \x20 set-playback-level NAME VALUE\n\
         \x20 set-capture-level NAME VALUE\n\
         \x20 smoke-test  Safely change, verify, and restore a disabled effect level\n\
         \x20 help      Show this help"
    );
}
