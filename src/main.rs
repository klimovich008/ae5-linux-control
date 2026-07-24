use ae5_control::{Ae5Device, snapshot_controls};
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
    match std::env::args().nth(1).as_deref() {
        None | Some("status") => print_status(),
        Some("controls") => print_controls(),
        Some("-h" | "--help" | "help") => {
            print_help();
            Ok(())
        }
        Some("-V" | "--version") => {
            println!("ae5ctl {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(command) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown command '{command}'; run 'ae5ctl --help'"),
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
        "Usage: ae5ctl [status|controls]\n\
         \n\
         Commands:\n\
         \x20 status    Show the detected AE-5 and important live controls (default)\n\
         \x20 controls  Show every live ALSA simple control\n\
         \x20 help      Show this help"
    );
}
