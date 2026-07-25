use ae5_control::{
    Ae5Device, Ae5Lighting, Ae5Mixer, DIRECT_MODE_CONTROL, FeatureSupport,
    LINUX_DRIVER_DEFAULTS_PRESERVED, ONBOARD_LED_COUNT, PipeWireNode, PipeWireRouteState, Profile,
    RgbColor, SbCommandImportReport, SbCommandTarget, ae5_input, ae5_output, ae5_route_state,
    apply_linux_driver_defaults, discover_sbcommand_installation, export_library_profile,
    feature_parity, headphone_playback_issue, import_active_sbcommand_profile_with_report,
    import_discovered_sbcommand_profile_with_report, import_sbcommand_profile_with_report,
    lighting_config_path, linux_driver_defaults, native_rates_config, profile_library,
    profile_library_directory, rename_library_profile, restore_saved_lighting,
    set_ae5_default_input, set_ae5_default_output, set_native_rates_enabled, set_saved_led,
    set_saved_lighting, snapshot_controls, validate_linux_driver_defaults,
};
use std::error::Error;
use std::fmt::Write as _;
use std::io;
use std::path::Path;

const IMPORTANT_CONTROLS: &[&str] = &[
    "Output Select",
    "Front",
    "HP/Speaker Auto Detect",
    "AE-5: Headphone Gain",
    "AE-5: Sound Filter",
    DIRECT_MODE_CONTROL,
    "Surround Channel Config",
    "Input Source",
    "Enable OutFX",
    "Wedge Angle",
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
        [command] if command == "features" => print_features(None),
        [command, status] if command == "features" => print_features(Some(status)),
        [command] if command == "profile-library" => print_profile_library(),
        [command] if command == "output-status" => print_output_status(),
        [command] if command == "route-status" => print_route_status(),
        [command] if command == "route-repair" => repair_route(),
        [command] if command == "set-default-output" => set_default_output(),
        [command] if command == "input-status" => print_input_status(),
        [command] if command == "set-default-input" => set_default_input(),
        [command] if command == "native-rates-status" => print_native_rates_status(),
        [command] if command == "native-rates-enable" => set_native_rates(true),
        [command] if command == "native-rates-disable" => set_native_rates(false),
        [command] if command == "lighting-status" => print_lighting_status(),
        [command] if command == "lighting-restore" => restore_lighting(),
        [command, red, green, blue] if command == "lighting-set" => set_lighting(red, green, blue),
        [command, index, red, green, blue] if command == "lighting-set-led" => {
            set_lighting_led(index, red, green, blue)
        }
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
        [command, source, output] if command == "profile-export" => export_profile(source, output),
        [command, source, new_name] if command == "profile-rename" => {
            rename_profile(source, new_name)
        }
        [command, path] if command == "profile-show" => show_profile(path),
        [command, path] if command == "profile-check" => check_profile(path, false),
        [command, path, flag] if command == "profile-check" && flag == "--allow-high-gain" => {
            check_profile(path, true)
        }
        [command, path] if command == "profile-apply" => apply_profile(path, false),
        [command, path, flag] if command == "profile-apply" && flag == "--allow-high-gain" => {
            apply_profile(path, true)
        }
        [command] if command == "linux-defaults-show" => show_linux_driver_defaults(),
        [command] if command == "linux-defaults-check" => check_linux_driver_defaults(),
        [command, backup, flag] if command == "linux-defaults-apply" && flag == "--confirm" => {
            apply_linux_defaults(backup)
        }
        [command, name, profile, eq, target, output] if command == "sbcommand-import" => {
            import_sbcommand(name, profile, eq, target, output)
        }
        [command, name, windows_user, target, output] if command == "sbcommand-import-user" => {
            import_sbcommand_user(name, windows_user, target, output)
        }
        [command, name, user_config, product_dir, target, output]
            if command == "sbcommand-import-active" =>
        {
            import_active_sbcommand(name, user_config, product_dir, target, output)
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
    match ae5_route_state(device.card_index) {
        Ok(state) => print_route_state(
            &state,
            selected_choice(&controls, "Output Select").unwrap_or("unavailable"),
            selected_choice(&controls, "Surround Channel Config").unwrap_or("unavailable"),
            selected_choice(&controls, "Input Source").unwrap_or("unavailable"),
            headphone_playback_issue(&controls),
        ),
        Err(error) => println!("  Desktop routes: unavailable ({error})"),
    }
    match Ae5Lighting::discover().and_then(|lighting| lighting.colors()) {
        Ok(colors) => println!(
            "  Onboard lighting: {}",
            colors
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Err(error) => println!("  Onboard lighting: unavailable ({error})"),
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

fn print_features(filter: Option<&str>) -> Result<(), Box<dyn Error>> {
    let filter = filter
        .map(|value| {
            FeatureSupport::parse(value).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "feature status must be verified, substituted, deferred, or unsupported",
                )
            })
        })
        .transpose()?;
    print!("{}", feature_status_report(filter));
    Ok(())
}

fn feature_status_report(filter: Option<FeatureSupport>) -> String {
    let features = feature_parity().collect::<Vec<_>>();
    let mut report = format!(
        "Sound Blaster Command feature compatibility ({} tracked)\n",
        features.len()
    );
    for support in FeatureSupport::ALL {
        let count = features
            .iter()
            .filter(|feature| feature.support == support)
            .count();
        let _ = writeln!(report, "  {support}: {count}");
    }

    for support in FeatureSupport::ALL {
        if filter.is_some_and(|selected| selected != support) {
            continue;
        }
        let selected = features
            .iter()
            .filter(|feature| feature.support == support)
            .collect::<Vec<_>>();
        let _ = writeln!(
            report,
            "\n{} ({}) — {}",
            support,
            selected.len(),
            support.description()
        );
        for feature in selected {
            let _ = writeln!(report, "  {} · {}", feature.area, feature.feature);
            let _ = writeln!(report, "    Linux: {}", feature.linux_mechanism);
            let _ = writeln!(report, "    Evidence: {}", feature.current_evidence);
            let _ = writeln!(report, "    Remaining: {}", feature.remaining_gate);
        }
    }
    report
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

fn print_route_status() -> Result<(), Box<dyn Error>> {
    let device = require_device()?;
    let controls = snapshot_controls(device.card_index)?;
    let output_choice = selected_choice(&controls, "Output Select").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "the AE-5 has no readable Output Select choice",
        )
    })?;
    let input_choice = selected_choice(&controls, "Input Source").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "the AE-5 has no readable Input Source choice",
        )
    })?;
    let speaker_layout =
        selected_choice(&controls, "Surround Channel Config").ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "the AE-5 has no readable Surround Channel Config choice",
            )
        })?;
    let state = ae5_route_state(device.card_index)?;
    let playback_issue = headphone_playback_issue(&controls);
    print_route_state(
        &state,
        output_choice,
        speaker_layout,
        input_choice,
        playback_issue,
    );
    if let Some(issue) = state
        .output_issue(output_choice, speaker_layout)
        .or_else(|| playback_issue.map(str::to_owned))
        .or_else(|| state.input_issue(input_choice))
    {
        return Err(io::Error::other(issue).into());
    }
    Ok(())
}

fn repair_route() -> Result<(), Box<dyn Error>> {
    let changes = mixer()?.repair_routes()?;
    if changes.is_empty() {
        println!("AE-5 desktop routes are already healthy; no changes made");
    } else {
        println!("AE-5 desktop route repaired");
        for change in changes {
            println!("  {change}");
        }
    }
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

fn print_native_rates_status() -> Result<(), Box<dyn Error>> {
    let config = native_rates_config()?;
    println!(
        "PipeWire native-rate switching: {}\n  {}",
        if config.enabled {
            "enabled in PipeWire configuration"
        } else {
            "disabled"
        },
        config.path.display()
    );
    Ok(())
}

fn set_native_rates(enabled: bool) -> Result<(), Box<dyn Error>> {
    let config = set_native_rates_enabled(enabled)?;
    println!(
        "PipeWire native-rate switching {}.\nRestart PipeWire or log in again to apply: {}",
        if config.enabled {
            "enabled for 44.1, 48, and 96 kHz"
        } else {
            "disabled"
        },
        config.path.display()
    );
    Ok(())
}

fn print_lighting_status() -> Result<(), Box<dyn Error>> {
    let colors = Ae5Lighting::discover()?.colors()?;
    println!("AE-5 onboard lighting");
    for (index, color) in colors.iter().enumerate() {
        println!("  LED {}: {color}", index + 1);
    }
    println!(
        "  Saved configuration: {}",
        lighting_config_path()?.display()
    );
    Ok(())
}

fn set_lighting(red: &str, green: &str, blue: &str) -> Result<(), Box<dyn Error>> {
    let color = parse_color(red, green, blue)?;
    let config = set_saved_lighting([color; ONBOARD_LED_COUNT])?;
    println!(
        "set and saved all {ONBOARD_LED_COUNT} onboard LEDs to {}",
        config.leds[0]
    );
    Ok(())
}

fn set_lighting_led(index: &str, red: &str, green: &str, blue: &str) -> Result<(), Box<dyn Error>> {
    let index = index.parse::<usize>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("LED index must be 1 through {ONBOARD_LED_COUNT}"),
        )
    })?;
    let color = parse_color(red, green, blue)?;
    set_saved_led(index, color)?;
    println!("set and saved onboard LED {index} to {color}");
    Ok(())
}

fn restore_lighting() -> Result<(), Box<dyn Error>> {
    match restore_saved_lighting()? {
        Some(config) => println!(
            "restored {} saved onboard LED colors from {}",
            config.leds.len(),
            lighting_config_path()?.display()
        ),
        None => println!(
            "no saved onboard lighting configuration at {}",
            lighting_config_path()?.display()
        ),
    }
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

fn print_route_state(
    state: &PipeWireRouteState,
    output_choice: &str,
    speaker_layout: &str,
    input_choice: &str,
    playback_issue: Option<&str>,
) {
    println!(
        "  Desktop output route: {}",
        state.output_route.as_deref().unwrap_or("unavailable")
    );
    println!(
        "  Desktop input route: {}",
        state.input_route.as_deref().unwrap_or("unavailable")
    );
    println!(
        "  PipeWire profile: {} ({})",
        state.active_profile.as_deref().unwrap_or("unavailable"),
        state
            .profile_set
            .as_deref()
            .unwrap_or("unknown profile set")
    );
    match state
        .output_issue(output_choice, speaker_layout)
        .or_else(|| playback_issue.map(str::to_owned))
    {
        None => println!("  Output route health: matched ALSA {output_choice}"),
        Some(issue) => println!("  Output route health: warning ({issue})"),
    }
    match state.input_issue(input_choice) {
        None => println!("  Input route health: matched ALSA {input_choice}"),
        Some(issue) => println!("  Input route health: warning ({issue})"),
    }
}

fn selected_choice<'a>(
    controls: &'a [ae5_control::ControlSnapshot],
    name: &str,
) -> Option<&'a str> {
    controls
        .iter()
        .find(|control| control.name == name)
        .and_then(|control| control.selected.as_deref())
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

fn print_profile_library() -> Result<(), Box<dyn Error>> {
    let library = profile_library()?;
    println!("Profile library: {}", library.directory.display());
    for entry in &library.profiles {
        println!(
            "  {} — {} controls ({})",
            entry.profile.name,
            entry.profile.controls.len(),
            entry
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("profile.json")
        );
    }
    if library.profiles.is_empty() {
        println!("  no saved profiles");
    }
    for warning in &library.skipped {
        println!("  skipped: {warning}");
    }
    Ok(())
}

fn export_profile(source: &str, output: &str) -> Result<(), Box<dyn Error>> {
    let source = profile_library_directory()?.join(source);
    let stored = export_library_profile(&source, Path::new(output))?;
    println!(
        "exported '{}' to {output} without changing the library copy",
        stored.profile.name
    );
    Ok(())
}

fn rename_profile(source: &str, new_name: &str) -> Result<(), Box<dyn Error>> {
    let source = profile_library_directory()?.join(source);
    let stored = rename_library_profile(&source, new_name)?;
    let file_name = stored
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("profile.json");
    println!(
        "renamed saved profile to '{}' ({file_name})",
        stored.profile.name
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

fn show_linux_driver_defaults() -> Result<(), Box<dyn Error>> {
    let profile = linux_driver_defaults()?;
    println!("Profile: {}", profile.name);
    println!("Target: {}", profile.target);
    println!("Controls reset: {}", profile.controls.len());
    println!("Conditional: X-Bass is disabled for a preserved LFE speaker layout.");
    println!("Preserved:");
    for item in LINUX_DRIVER_DEFAULTS_PRESERVED {
        println!("  - {item}");
    }
    println!();
    println!("{}", serde_json::to_string_pretty(&profile.controls)?);
    Ok(())
}

fn check_linux_driver_defaults() -> Result<(), Box<dyn Error>> {
    let mixer = mixer()?;
    let profile = validate_linux_driver_defaults(&mixer)?;
    println!(
        "compatible: {} Linux-driver default controls and their restorable backup are available; \
         no hardware values were changed",
        profile.controls.len()
    );
    Ok(())
}

fn apply_linux_defaults(backup: &str) -> Result<(), Box<dyn Error>> {
    let report = apply_linux_driver_defaults(&mixer()?, Path::new(backup))?;
    println!(
        "reset {} Linux-driver default controls after saving the previous valid state to {backup}",
        report.controls_applied
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

fn import_active_sbcommand(
    name: &str,
    user_config: &str,
    product_dir: &str,
    target: &str,
    output: &str,
) -> Result<(), Box<dyn Error>> {
    import_active_sbcommand_paths(
        name,
        Path::new(user_config),
        Path::new(product_dir),
        target,
        output,
    )
}

fn import_active_sbcommand_paths(
    name: &str,
    user_config: &Path,
    product_dir: &Path,
    target: &str,
    output: &str,
) -> Result<(), Box<dyn Error>> {
    let target = target.parse::<SbCommandTarget>()?;
    let import =
        import_active_sbcommand_profile_with_report(name, user_config, product_dir, target)?;
    print_import_report(&import.report);
    import.profile.save_new(Path::new(output))?;
    println!(
        "converted active Sound Blaster Command {target} settings to '{}' ({} controls) at {output}",
        import.profile.name,
        import.profile.controls.len()
    );
    Ok(())
}

fn import_sbcommand_user(
    name: &str,
    windows_user: &str,
    target: &str,
    output: &str,
) -> Result<(), Box<dyn Error>> {
    let installation = discover_sbcommand_installation(Path::new(windows_user))?;
    let target = target.parse::<SbCommandTarget>()?;
    let import = import_discovered_sbcommand_profile_with_report(name, &installation, target)?;
    print_import_report(&import.report);
    import.profile.save_new(Path::new(output))?;
    println!(
        "converted active Sound Blaster Command {target} settings to '{}' ({} controls) at {output}",
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

fn parse_color(red: &str, green: &str, blue: &str) -> Result<RgbColor, io::Error> {
    let parse = |name: &str, value: &str| {
        value.parse::<u8>().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{name} must be an integer from 0 through 255"),
            )
        })
    };
    Ok(RgbColor::new(
        parse("red", red)?,
        parse("green", green)?,
        parse("blue", blue)?,
    ))
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
         \x20 features [verified|substituted|deferred|unsupported]\n\
         \x20           Show evidence-tracked Sound Blaster Command compatibility\n\
         \x20 output-status       Show the AE-5 PipeWire playback target\n\
         \x20 route-status        Verify ALSA and PipeWire hardware routes agree\n\
         \x20 route-repair        Explicitly repair the current ALSA/PipeWire routes\n\
         \x20 set-default-output  Make the AE-5 the PipeWire default playback target\n\
         \x20 input-status        Show the AE-5 PipeWire recording target\n\
         \x20 set-default-input   Make the AE-5 the PipeWire default recording target\n\
         \x20 native-rates-status   Show the per-user PipeWire rate configuration\n\
         \x20 native-rates-enable   Allow native 44.1, 48, and 96 kHz after restart\n\
         \x20 native-rates-disable  Remove the managed native-rate configuration\n\
         \x20 lighting-status       Show all five onboard LED colors\n\
         \x20 lighting-set RED GREEN BLUE\n\
         \x20 lighting-set-led INDEX RED GREEN BLUE\n\
         \x20 lighting-restore      Restore the saved onboard LED colors\n\
         \x20 get NAME\n\
         \x20 set-choice NAME CHOICE [--allow-high-gain]\n\
         \x20 set-playback-switch NAME on|off\n\
         \x20 set-capture-switch NAME on|off\n\
         \x20 set-playback-level NAME VALUE\n\
         \x20 set-capture-level NAME VALUE\n\
         \x20 set-playback-channel-level NAME CHANNEL VALUE\n\
         \x20 set-capture-channel-level NAME CHANNEL VALUE\n\
         \x20 smoke-test  Safely change, verify, and restore a disabled effect level\n\
         \x20 profile-library  List native profiles in the per-user library\n\
         \x20 profile-save NAME FILE\n\
         \x20 profile-export LIBRARY_FILE OUTPUT\n\
         \x20 profile-rename LIBRARY_FILE NEW_NAME\n\
         \x20 profile-show FILE\n\
         \x20 profile-check FILE [--allow-high-gain]\n\
         \x20 profile-apply FILE [--allow-high-gain]\n\
         \x20 linux-defaults-show\n\
         \x20 linux-defaults-check  Validate the reset without changing hardware\n\
         \x20 linux-defaults-apply BACKUP_FILE --confirm\n\
         \x20 sbcommand-import NAME PROFILE_JSON EQ_JSON speaker|headphone OUTPUT\n\
         \x20 sbcommand-import-user NAME WINDOWS_USER_DIR speaker|headphone OUTPUT\n\
         \x20 sbcommand-import-active NAME USER_CONFIG AE5_PRODUCT_DIR speaker|headphone OUTPUT\n\
         \x20 help      Show this help"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_report_filters_details_but_keeps_summary_counts() {
        let report = feature_status_report(Some(FeatureSupport::Unsupported));
        let features = feature_parity().collect::<Vec<_>>();
        assert!(report.contains(&format!(
            "Sound Blaster Command feature compatibility ({} tracked)",
            features.len()
        )));
        for support in FeatureSupport::ALL {
            let count = features
                .iter()
                .filter(|feature| feature.support == support)
                .count();
            assert!(report.contains(&format!("{support}: {count}")));
        }
        for feature in features {
            assert_eq!(
                report.contains(&format!("{} · {}", feature.area, feature.feature)),
                feature.support == FeatureSupport::Unsupported
            );
        }
    }
}
