# AE-5 Linux Control

Linux control software and upstream driver fixes for the Creative Sound
BlasterX AE-5, developed from public source and reproducible hardware evidence.

## Current milestone: hardware audit

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
