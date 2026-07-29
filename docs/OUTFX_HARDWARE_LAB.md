# AE-5 hardware OutFX corruption lab

> **Diagnostic use only.** This path deliberately re-enables a CA0132 control
> that the production application and kernel reject. It is not the Windows
> Sound Blaster Command OutFX path: Windows Command controls a software APO
> chain, while this lab writes the AE-5 hardware DSP controls.

Use this runbook only to identify the exact hardware-control transition that
causes the AE-5 DSP fault. The known failure is severe distortion or a
continuous roughly 61–65 Hz buzz that can survive software mute, an OutFX
toggle, a warm reboot, and an operating-system change. Complete motherboard
power removal has recovered every persistent case observed so far.

## Safety boundary

Before booting the lab kernel:

- physically unplug every cable from the AE-5 headphone and analog line-out
  jacks;
- keep headphones off your head during any later connected-output test;
- keep the PipeWire AE-5 sink at or below 20%;
- leave Direct Mode disabled;
- close the lab immediately after an unexpected write failure or noise;
- do not assume mute will stop a latched DSP oscillation.

The first run is internal-capture-only. Do not reconnect an AE-5 analog output
until an isolated sequence has completed without corruption and the user has
explicitly agreed to an audible follow-up.

## Why the lab cannot be enabled accidentally

All three gates must agree before the GUI will write hardware OutFX or its
child output effects:

1. the GUI was compiled with the `outfx-lab` Cargo feature;
2. the running release is exactly `7.1.4-ae5-outfx-lab` and its module
   parameter `ae5_unsafe_outfx_lab` is `Y`;
3. the process has
   `AE5_OUTFX_LAB=I_ACCEPT_AE5_DSP_CORRUPTION`.

The lab kernel boots with the parameter set to `N` and OutFX initialized off.
Direct Mode and output-route writes remain blocked. A normal build, the
production kernel, or any missing gate remains fail-closed.

## Build and install

Build the lab GUI without replacing the normal user installation:

```sh
cargo build --release --locked --features gui,outfx-lab
```

Apply `kernel/ca0132-ae5-outfx-lab.patch` only after the production OutFX
guard, then build a side-by-side kernel whose exact release is
`7.1.4-ae5-outfx-lab`. Verify the resulting RPM before installation:

```sh
scripts/check-host-kernel-rpm.sh \
  /path/to/kernel-7.1.4_ae5_outfx_lab-1.x86_64.rpm \
  7.1.4-ae5-outfx-lab
```

Follow the side-by-side installation gate in
[`KERNEL_MAINTENANCE.md`](KERNEL_MAINTENANCE.md). Do not replace the stock or
qualified stable kernel, and select the lab entry for one boot only.

## Preflight after the lab boot

Do not enable the module parameter until every check below passes:

```sh
test "$(uname -r)" = 7.1.4-ae5-outfx-lab
test "$(cat /proc/sys/kernel/tainted)" = 0
test "$(cat /sys/module/snd_hda_codec_ca0132/parameters/ae5_unsafe_outfx_lab)" = N
ae5ctl status
scripts/dsp-oscillation-monitor.sh --self-test
scripts/dsp-reinit.sh --self-test
```

Confirm from the live AE-5 mixer that `Enable OutFX` is off and both the
headphone and front line-out jack sensors are off. Set the AE-5 PipeWire sink
to 20% or lower and mute it before proceeding.

## Capture a session

Create one new evidence directory per boot. Start the passive What U Hear
monitor before allowing writes:

```sh
mkdir -p ~/.local/state/ae5-outfx-lab
AE5_MONITOR_INTERVAL=2 \
AE5_MONITOR_SAMPLE=1 \
AE5_MONITOR_LOG=~/.local/state/ae5-outfx-lab/monitor.csv \
  scripts/dsp-oscillation-monitor.sh
```

The monitor needs hardware Master on because What U Hear is after that switch.
It changes no playback control. Its RMS alarm is meaningful during digital
silence; ordinary music can also exceed the threshold, so pause playback and
wait for a fresh sample after every tested transition.

In a second terminal, enable the kernel gate for this boot:

```sh
printf Y | pkexec /usr/bin/tee \
  /sys/module/snd_hda_codec_ca0132/parameters/ae5_unsafe_outfx_lab
```

Verify that the parameter reads `Y`, then launch the lab GUI under Wayland
with its structured mixer trace:

```sh
AE5_OUTFX_LAB=I_ACCEPT_AE5_DSP_CORRUPTION \
AE5_TRACE=1 \
GDK_BACKEND=wayland \
  target/release/ae5-control \
  2>~/.local/state/ae5-outfx-lab/gui-trace.log
```

Test one transition at a time. Wait at least one silent monitor sample between
steps and record the user-visible action:

1. baseline with global OutFX off;
2. enable global OutFX while every child effect is off;
3. enable one child effect without changing its level;
4. change that effect by one small step;
5. disable the child effect;
6. disable global OutFX;
7. repeat for the next child only after the previous sequence remains clean.

Do not apply a full profile in this phase: it writes several controls and
would hide the first corrupting transition. Track changes are tested only
after the idle single-control matrix is clean. For a track-change test, play
at no more than 20%, switch or stop the track, pause playback, and judge the
next silent samples.

## Stop and recover

Stop immediately if the monitor prints `OSCILLATING`, the GUI reports a failed
write, or any unexpected noise is heard. Do not keep toggling controls in an
attempt to clear the fault. Preserve the monitor CSV and GUI trace, note the
last successful action, and close playback applications.

Try the scoped PCI rebind recovery with the AE-5 analog outputs still
physically disconnected:

```sh
AE5_REINIT_YES=1 scripts/dsp-reinit.sh
```

The helper hard-mutes, stops the user PipeWire session, verifies the exact
AE-5 PCI and subsystem IDs, unbinds and rebinds only that card, confirms a DSP
download when visible in the kernel log, and leaves the sink muted at 5%.
It requests desktop authorization before stopping PipeWire and falls back to
authenticated sudo when PolicyKit is unavailable.

If rebind fails or the fault remains, shut the machine down completely,
remove motherboard power long enough for the card to lose power, and boot the
qualified stable kernel. A warm reboot is not an accepted recovery.

## Return to the safe path

Before leaving the lab boot, close the GUI, set `Enable OutFX` off, set the
module parameter back to `N`, and verify the readbacks. Reboot the qualified
stable kernel for normal use. Never add the unsafe parameter to a module-load
configuration or make the lab kernel the saved default.
