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
GTK idle measures control refresh. The harness allows the measured refresh's
final rendered frame one second to settle, then samples `/proc` for five full
seconds to report process CPU time and peak idle `VmRSS`. Continuous or
periodic background work during those five seconds still fails the idle budget.

The probe only reads hardware and process state. It does not write ALSA
controls, change PipeWire routing, apply profiles, or create a native-rate
configuration.

## Reference result

Measured on Nobara 44, Linux `7.1.4-200.nobara.fc44.x86_64`, GTK 4.22.4, an AMD
Ryzen 7 5700X3D, Radeon RX 9070 XT, and the physical
`1102:0012/1102:0051` AE-5:

| Run | Startup ms | Refresh ms | Idle CPU % | Peak idle RSS KiB |
|---:|---:|---:|---:|---:|
| 1 | 351 | 69 | 0.00 | 81,264 |
| 2 | 330 | 68 | 0.00 | 81,168 |
| 3 | 331 | 67 | 0.00 | 81,244 |
| 4 | 329 | 68 | 0.00 | 81,344 |
| 5 | 330 | 68 | 0.00 | 81,252 |

All five runs pass every budget. The worst observed result was 351 ms startup,
69 ms refresh, 0.00% idle CPU, and 81,344 KiB (79.4 MiB) RSS.

Before the current route-query optimization, five runs took 115–117 ms to
refresh because desktop route health first located the AE-5 through three
`wpctl` queries and then read the device with `pw-dump`. PipeWire's dump already
identifies the device by ALSA card, so the refresh now performs one read-only
query and retains the same exact-card and ambiguity checks.

An earlier UI optimization reduced one equivalent sample from 414 ms startup,
120 ms refresh, and 139,916 KiB RSS. The application previously constructed
every page before presenting the first one and retained the Vulkan driver's
LLVM renderer footprint. It now constructs its nine pages when first selected
and defaults this mostly static interface to GTK's Cairo renderer. Setting
`GSK_RENDERER` explicitly still overrides that default for troubleshooting or
accelerated renderer comparison.

The release GUI was also opened under the native Nobara GTK theme and all nine
pages were selected without changing a control. GTK 4.22 otherwise reports a
negative minimum size for the theme's 6 px scrollbar slider with its negative
margins and transparent border. AE-5 Control gives scrollbar sliders an 8 px
minimum on their narrow axis, preserving the theme while keeping every
slider's measured minimum non-negative.

After the current five-run read-only pass, the physical AE-5 remained on the
matched Headphone/Microphone duplex route. PipeWire remained at `0.20`, Master
and Front at 19%, PCM at 20%, Surround/Center/LFE muted at 0%, and headphone
gain Low. Every playback PCM was closed, and the complete raw mixer hash
remained
`5f72b79126e713debcc4f975e86cc9ac1bfe1ed39cd4760e4f5f44a5766656bf`.
No playback was attempted.

These numbers prove the software budgets on the recorded reference system.
They do not replace audio latency, DSP, cold-boot, or suspend/resume tests.

The installed transport-fix build was rechecked explicitly with
`GDK_BACKEND=wayland` on 2026-07-26. It measured 235 ms startup, 68 ms refresh,
0.00% sampled idle CPU, and 68,440 KiB peak idle RSS. A live active-window
capture showed the refined hardware console with the exact AE-5 online and all
48 controls detected. The only GTK diagnostic was the distribution's
unrecognized global `gtk-modules` setting; the application itself emitted no
error.

The later kernel-readiness Device card was measured in the same native Wayland
session from the release binary. Its read-only kernel release, taint, Direct
Mode, and onboard-lighting checks completed with the normal hardware refresh:
209 ms startup, 68 ms refresh, 0.00% sampled idle CPU, and 70,844 KiB peak idle
RSS. Every Version 1 budget passed.
