# GUI performance baseline

AE-5 Control's Version 1 budgets are:

- cold startup below 1,000 ms;
- a complete hardware-backed control refresh within 100 ms;
- no more than 1% CPU while idle;
- resident memory below 100 MiB.

Run the read-only release-build measurement on a graphical Linux session with
the AE-5 connected:

```sh
cargo build --locked --release --all-features
bash scripts/measure-gui-performance.sh
```

The application uses a non-unique performance-probe instance, so another
installed AE-5 Control process cannot receive the activation and invalidate the
sample. The first GTK idle after presenting the complete window measures
startup. The probe then reads the AE-5 and rebuilds the visible page; the next
GTK idle measures control refresh. The harness samples `/proc` for five seconds
to report process CPU time and peak idle `VmRSS`.

The probe only reads hardware and process state. It does not write ALSA
controls, change PipeWire routing, apply profiles, or create a native-rate
configuration.

## Reference result

Measured on Nobara 44, Linux `7.1.4-200.nobara.fc44.x86_64`, GTK 4.22.4, an AMD
Ryzen 7 5700X3D, Radeon RX 9070 XT, and the physical
`1102:0012/1102:0051` AE-5:

| Run | Startup ms | Refresh ms | Idle CPU % | Peak idle RSS KiB |
|---:|---:|---:|---:|---:|
| 1 | 324 | 62 | 0.40 | 83,952 |
| 2 | 311 | 63 | 0.60 | 83,620 |
| 3 | 297 | 61 | 0.40 | 83,476 |
| 4 | 301 | 62 | 0.40 | 83,736 |
| 5 | 301 | 62 | 0.20 | 83,420 |

All five runs pass every budget. The worst observed result was 324 ms startup,
63 ms refresh, 0.60% idle CPU, and 83,952 KiB (82.0 MiB) RSS. After manually
opening all seven lazy pages, a second five-second idle sample measured 0.00%
CPU and 89,384 KiB (87.3 MiB) RSS.

Before optimization, one equivalent sample reported 414 ms startup, 120 ms
refresh, and 139,916 KiB RSS. The application previously constructed all seven
pages before presenting the first one and retained the Vulkan driver's LLVM
renderer footprint. It now constructs pages when first selected and defaults
this mostly static interface to GTK's Cairo renderer. Setting `GSK_RENDERER`
explicitly still overrides that default for troubleshooting or accelerated
renderer comparison.

The release GUI was also opened under the native Nobara GTK theme and all seven
pages were selected without changing a control. GTK 4.22 otherwise reports a
negative minimum size for the theme's 6 px scrollbar slider with its negative
margins and transparent border. AE-5 Control gives scrollbar sliders an 8 px
minimum on their narrow axis, preserving the theme while keeping every
slider's measured minimum non-negative.

These numbers prove the software budgets on the recorded reference system.
They do not replace audio latency, DSP, cold-boot, or suspend/resume tests.
