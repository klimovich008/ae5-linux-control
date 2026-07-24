# AE-5 Linux Control

Linux control software and upstream driver fixes for the Creative Sound
BlasterX AE-5, developed from public source and reproducible hardware evidence.

## Current milestone: desktop profiles, live synchronization, and routing diagnosis

The first Rust slice detects the audited AE-5 revision by its PCI and subsystem
IDs, opens the matching ALSA mixer through `libasound`, and reads its live
controls without changing them.

On Fedora/Nobara, install the build dependency and run:

```sh
sudo dnf install alsa-lib-devel
cargo run -- status
cargo run -- controls
```

`status` prints the exact card identity and important control state. `controls`
prints all simple mixer controls and their current values.

PipeWire may prefer other playback and recording devices even when the AE-5 is
detected. Inspect the mapped nodes or explicitly make either one the desktop
default through WirePlumber:

```sh
cargo run -- output-status
cargo run -- set-default-output
cargo run -- input-status
cargo run -- set-default-input
```

The routing action invokes `wpctl` directly without a shell and verifies the
new default. It does not change the card's ALSA mixer controls.

The optional native-rate configuration lets PipeWire switch the global graph
between 44.1, 48, and 96 kHz after its next restart:

```sh
cargo run -- native-rates-status
cargo run -- native-rates-enable
cargo run -- native-rates-disable
```

It is never enabled automatically. The commands only manage AE-5 Control's
per-user PipeWire fragment and refuse to overwrite a different file at the
same path. Hardware evidence, limitations, and verification steps are in
[docs/PIPEWIRE_RATE_PARITY.md](docs/PIPEWIRE_RATE_PARITY.md).

Typed write commands validate choices and ranges, write through ALSA, and
verify the value by reading it back:

```sh
cargo run -- get "Output Select"
cargo run -- set-choice "Output Select" Headphone
cargo run -- set-playback-switch "FX: Surround" off
cargo run -- set-playback-level "FX: Surround" 50
cargo run -- set-playback-channel-level Front "Front Right" 82
```

High headphone gain is rejected unless `--allow-high-gain` is supplied. The
hardware smoke test changes a disabled effect level, verifies it, and restores
the original value:

```sh
cargo run -- smoke-test
```

Native profiles use semantic control and channel names rather than ALSA card
indexes or numeric control IDs. Stereo balances are captured and restored
without breaking profiles created before channel support. Saving refuses to
overwrite an existing file; checking performs all validation without changing
hardware; applying verifies every write and rolls back the targeted controls
if a write fails:

```sh
cargo run -- profile-library
cargo run -- profile-save "My headphones" headphones.json
cargo run -- profile-show headphones.json
cargo run -- profile-check headphones.json
cargo run -- profile-apply headphones.json
```

The desktop keeps reusable profiles in
`$XDG_CONFIG_HOME/ae5-control/profiles`, falling back to
`~/.config/ae5-control/profiles`. The library command lists every valid profile
and reports malformed JSON without hiding usable entries. Desktop save and
import dialogs start in this folder but can still target another local folder.

## Import Sound Blaster Command settings

Creative's AE-5 profile and EQ JSON files can be combined into a native,
validated profile without changing the hardware:

```sh
cargo run -- sbcommand-import "Windows headphones" \
  Profile.json Equalizer.json headphone windows-headphones.json
cargo run -- profile-check windows-headphones.json
cargo run -- profile-apply windows-headphones.json
```

The active selection can also be imported directly from a mounted Windows
installation. `USER_CONFIG` is Sound Blaster Command's versioned
`Creative.SBCommand.../<version>/user.config` file under the Windows user's
`AppData/Local/Creative_Technology_Ltd` directory. `AE5_PRODUCT_DIR` is that
user's `AppData/Local/Creative/<installation-id>/Product/AE5` directory:

```sh
cargo run -- sbcommand-import-active "Windows speakers" \
  "$USER_CONFIG" "$AE5_PRODUCT_DIR" speaker windows-speakers.json
cargo run -- profile-check windows-speakers.json
```

This form follows the selected profile and EQ IDs, preserves the output route,
and maps standard Windows speaker masks from stereo through 5.1. It reads only
plain XML string settings. Binary-serialized application state is never
deserialized. Creative headphone tuning selections are reported as unsupported
until the Linux driver exposes a safe equivalent.

The importer maps SBX switches and levels, crossover frequency, Smart Volume
mode, and all ten EQ bands. Before saving, it separates exact mappings,
values rounded to ALSA steps, and unsupported non-null source settings. The
CLI prints the complete report and the desktop preview lists every unsupported
field. Unsupported settings such as a non-zero EQ preamp are skipped while the
representable controls are retained; invalid products, files, ranges, units,
band counts, and frequencies are still rejected.

## Native desktop application

The GTK 4 application groups every live control into system audio, profiles,
playback, effects, equalizer, and recording pages. The **System audio** page can
make the AE-5 the default PipeWire playback or recording device and opt into
native-rate switching without changing its ALSA mixer controls. Stereo ALSA
controls receive separate accessible channel sliders; selectors, switches, and
bounded sliders write through the verified ALSA backend. High headphone gain
requires an explicit opt-in. It listens for native ALSA mixer events, so changes
made by another mixer application or command-line process are reflected without
a polling loop while the selected page remains open:

```sh
sudo dnf install gtk4-devel
cargo run --features gui --bin ae5-control
```

Nobara/Fedora RPM build and install instructions are in
[packaging/README.md](packaging/README.md). The package installs the GTK app,
CLI, desktop entry, AppStream metadata, and icon without a privileged helper.

The **Profiles** page can:

- list reusable profiles from the per-user library with a guarded preview and
  apply action;
- save the current hardware state as a native JSON profile;
- validate and preview a native profile before applying it transactionally;
- import the active setup from a mounted Windows `user.config` and AE-5 product
  folder, or choose Sound Blaster Command profile and EQ JSON files manually;
- review exact, approximate, and unsupported mappings for headphones or
  speakers, then save a native copy.

The Windows source files are only read. Importing does not change the hardware;
the converted profile must be applied separately. Existing destination files
are never overwritten, and a profile requesting high headphone gain requires
a dedicated confirmation.

## Hardware audit

Collect the actual card identity, driver state, ALSA controls, codec data, and
relevant kernel log with the installed package:

```sh
ae5-collect-report
```

From a source checkout, the equivalent command is:

```sh
bash scripts/collect-linux-report.sh
```

The command is read-only, does not use `sudo`, and creates a private
`ae5-report-YYYYMMDD-HHMMSS.txt` file in the current directory. Review it
before sharing. Run its built-in check with:

```sh
bash scripts/collect-linux-report.sh --self-test
```

The implementation and test plan is in [PORT_PLAN.md](PORT_PLAN.md).

For the reported first-use headphone routing problem, the current kernel
already contains both relevant upstream CA0132 fixes. The read-only cold-boot
probe and evidence matrix are documented in
[docs/DRIVER_ROUTING_INVESTIGATION.md](docs/DRIVER_ROUTING_INVESTIGATION.md).
The exact Nobara/upstream driver source, public research references, firmware
licence boundary, and pinned revisions are recorded in
[docs/SOURCE_INVENTORY.md](docs/SOURCE_INVENTORY.md).

Objective Windows/Linux level, frequency-response, and noise comparison is
documented in
[docs/AUDIO_PARITY_MEASUREMENT.md](docs/AUDIO_PARITY_MEASUREMENT.md). The
SoX-based harness generates one hash-verified reference set and compares
unaltered 48 kHz/24-bit stereo captures without changing mixer controls.
