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

Typed write commands validate choices and ranges, write through ALSA, and
verify the value by reading it back:

```sh
cargo run -- get "Output Select"
cargo run -- set-choice "Output Select" Headphone
cargo run -- set-playback-switch "FX: Surround" off
cargo run -- set-playback-level "FX: Surround" 50
```

High headphone gain is rejected unless `--allow-high-gain` is supplied. The
hardware smoke test changes a disabled effect level, verifies it, and restores
the original value:

```sh
cargo run -- smoke-test
```

Native profiles use semantic control names and values rather than ALSA card
indexes or numeric control IDs. Saving refuses to overwrite an existing file;
checking performs all validation without changing hardware; applying verifies
every write and rolls back the targeted controls if a write fails:

```sh
cargo run -- profile-save "My headphones" headphones.json
cargo run -- profile-show headphones.json
cargo run -- profile-check headphones.json
cargo run -- profile-apply headphones.json
```

## Import Sound Blaster Command settings

Creative's AE-5 profile and EQ JSON files can be combined into a native,
validated profile without changing the hardware:

```sh
cargo run -- sbcommand-import "Windows headphones" \
  Profile.json Equalizer.json headphone windows-headphones.json
cargo run -- profile-check windows-headphones.json
cargo run -- profile-apply windows-headphones.json
```

The importer maps SBX switches and levels, crossover frequency, Smart Volume
mode, and all ten EQ bands. Before saving, it separates exact mappings,
values rounded to ALSA steps, and unsupported non-null source settings. The
CLI prints the complete report and the desktop preview lists every unsupported
field. Unsupported settings such as a non-zero EQ preamp are skipped while the
representable controls are retained; invalid products, files, ranges, units,
band counts, and frequencies are still rejected.

## Native desktop application

The GTK 4 application groups every live control into profiles, playback,
effects, equalizer, and recording pages. Selectors, switches, and bounded
sliders write through the verified ALSA backend; high headphone gain requires
an explicit opt-in. It listens for native ALSA mixer events, so changes made by
another mixer application or command-line process are reflected without a
polling loop while the selected page remains open:

```sh
sudo dnf install gtk4-devel
cargo run --features gui --bin ae5-control
```

The **Profiles** page can:

- save the current hardware state as a native JSON profile;
- validate and preview a native profile before applying it transactionally;
- import real Sound Blaster Command profile and EQ JSON files for headphones
  or speakers, review exact, approximate, and unsupported mappings, and save a
  native copy.

The Windows source files are only read. Importing does not change the hardware;
the converted profile must be applied separately. Existing destination files
are never overwritten, and a profile requesting high headphone gain requires
a dedicated confirmation.

## Hardware audit

Before adding Rust or changing the kernel, collect the actual card identity,
driver state, ALSA controls, codec data, and relevant kernel log:

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
