# AE-5 Linux Control

Linux control software and upstream driver fixes for the Creative Sound
BlasterX AE-5, developed from public source and reproducible hardware evidence.

## Current milestone: Rust hardware backend

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
