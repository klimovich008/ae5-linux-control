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
On the target AE-5, guarded 44.1 and 96 kHz physical What U Hear captures
matched direct ALSA and PipeWire by 0.00 dB in level and response when the PCM
mixer was at 0 dB; alternative-rate switching remains an explicit opt-in.

Typed write commands validate choices and ranges and verify the value by
reading it back. `Output Select` and `Input Source` use the matching
WirePlumber port from the packaged AE-5 profile so the desktop route and ALSA
enum cannot silently disagree; the other controls write directly through
ALSA:

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
if a write fails. Profiles validate their projected final bass-routing state
before the first write, then disable conflicting effects before route changes
and enable target effects afterward. CA0132 factory EQ presets contain
fractional values that the whole-dB band controls cannot represent reliably,
so newly captured factory-preset profiles omit those stale bands. Legacy
profiles ignore them during apply, preserving the exact preset curve:

```sh
cargo run -- profile-library
cargo run -- profile-save "My headphones" headphones.json
cargo run -- profile-show headphones.json
cargo run -- profile-check headphones.json
cargo run -- profile-apply headphones.json
```

AE-5 Control also exposes an evidence-based Linux-driver processing baseline.
It is not labeled as Creative's factory reset because Sound Blaster Command's
exact reset semantics are undocumented. Previewing and checking are read-only:

```sh
cargo run -- linux-defaults-show
cargo run -- linux-defaults-check
```

An apply requires both an explicit confirmation flag and a new backup path.
The previous valid mixer state is saved before the first write, and the normal
profile transaction verifies or rolls back the reset:

```sh
cargo run -- linux-defaults-apply before-reset.json --confirm
```

Routing, speaker layout, mixer volumes and mutes, and PipeWire settings are
preserved. The exact values, source provenance, exclusions, and validation
status are in
[docs/LINUX_DRIVER_DEFAULTS.md](docs/LINUX_DRIVER_DEFAULTS.md).

`profile-export` takes a library filename shown by `profile-library`, writes a
standalone copy anywhere, and refuses to overwrite an existing file:

```sh
cargo run -- profile-export headphones.json ~/Documents/headphones.json
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
installation by selecting the Windows user directory:

```sh
cargo run -- sbcommand-import-user "Windows speakers" \
  "/run/media/$USER/Windows/Users/<WindowsUser>" speaker windows-speakers.json
cargo run -- profile-check windows-speakers.json
```

The importer discovers the newest numeric Sound Blaster Command version and
requires one unambiguous AE-5 product directory. When the selected user folder
is on a complete mounted Windows volume, it also matches the active
`CtxHda.sys` to its DriverStore package and reads that package's INF version.
Both versions lead the migration report. If an installation has multiple
candidates, `sbcommand-import-active` remains available with explicit
`USER_CONFIG` and `AE5_PRODUCT_DIR` paths. The desktop performs the same
discovery after **Import active Windows setup** asks for the mounted Windows
user folder.

This flow follows the selected profile and EQ IDs, preserves the output route,
and maps standard Windows speaker masks from stereo through 5.1. It reads only
plain XML string settings. Binary-serialized application state is never
deserialized. Creative headphone tuning selections are reported as
unsupported until the Linux driver exposes a safe equivalent. A Windows Bass
request on an LFE speaker layout is also retained as unsupported because
CA0132 cannot enable X-Bass there; the converted profile explicitly turns
X-Bass off before changing to that route.

The importer maps SBX switches and levels, crossover frequency, Smart Volume
mode, and all ten EQ bands. It selects the driver's Flat preset before custom
bands so a prior factory curve cannot leak into the migrated settings. Before
saving, it separates exact mappings, values rounded to ALSA steps, and
unsupported non-null source settings. The CLI prints the complete report and
the desktop preview lists every unsupported field. Unsupported settings such
as a non-zero EQ preamp are skipped while the representable controls are
retained; invalid products, files, ranges, units, band counts, and frequencies
are still rejected.

## Native desktop application

The GTK 4 application groups device diagnostics, system audio, profiles,
playback, effects, equalizer, and recording into dedicated pages. The
**Device** page shows the exact detected hardware, live capability counts, and
driver values outside their advertised ranges. It can save the same
privacy-conscious diagnostics report as `ae5-collect-report` without invoking a
shell or requiring root. The **System audio** page can make the AE-5 the default
PipeWire playback or recording device and opt into native-rate switching
without changing its ALSA mixer controls.

Stereo ALSA controls receive separate accessible channel sliders; selectors,
switches, and bounded sliders write through the verified ALSA backend. Each
control row also exposes its ALSA name and current state to assistive
technology, including the reason when a guarded action is unavailable. High
headphone gain requires an explicit opt-in. The GUI enables bass redirection
only for Speakers with an LFE channel and disables X-Bass on those speaker
layouts; each unavailable switch explains which setting must change. The
equalizer disables custom band sliders while a factory preset is selected and
explains that Flat must be selected first. The shared backend applies the same
guards to CLI and profile writes, so those constraints cannot be bypassed
outside the GUI. It listens for native ALSA mixer events, so changes made by
another mixer application or command-line process are reflected without a
polling loop while the selected page remains open:

```sh
sudo dnf install gtk4-devel
cargo run --features gui --bin ae5-control
```

The release GUI has reproducible startup, hardware-refresh, idle CPU, and
resident-memory budgets. Run the read-only measurement with:

```sh
cargo build --locked --release --all-features
bash scripts/measure-gui-performance.sh
```

All five reference-system runs meet the sub-second startup, 100 ms refresh,
1% idle CPU, and 100 MiB RSS targets. The exact method, hardware, before/after
evidence, and results are recorded in
[docs/GUI_PERFORMANCE.md](docs/GUI_PERFORMANCE.md).

Nobara/Fedora RPM build and install instructions are in
[packaging/README.md](packaging/README.md). The package installs the GTK app,
CLI, desktop entry, AppStream metadata, and icon without a privileged helper.
A clean Fedora 44 build/install/verify/remove transaction is now enforced in
pull-request CI, and a read-only run of an exact RPM payload on the physical
AE-5 passed. The evidence and remaining authenticated-host install gate are in
[docs/PACKAGING_VALIDATION.md](docs/PACKAGING_VALIDATION.md).

The **Profiles** page can:

- list reusable profiles from the per-user library with a guarded preview and
  apply action;
- export a standalone copy without changing or overwriting the saved profile;
- rename profiles in place and move unwanted profiles to the recoverable
  desktop Trash;
- save the current hardware state as a native JSON profile;
- validate and preview a native profile before applying it transactionally;
- preview and restore source-derived Linux driver processing defaults after
  automatically saving a native recovery profile;
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
The evidence-tracked [feature parity matrix](feature-parity.tsv) classifies
each Command feature as verified, intentionally substituted, deferred, or
unsupported; deferred rows name the acceptance evidence still required.

The reported first-use headphone failure is now reproduced. PipeWire's generic
headphone route muted the CA0132 `Front` DAC even though the AE-5 headphones
share it. The RPM supplies a card-scoped ACP headphone path that keeps Front
enabled and exact Microphone, Front Microphone, and Line In routes; all three
input ports selected the matching ALSA enum in a physical-card matrix. The
fixed headphone route survived a WirePlumber restart and one instrumented cold
boot with the intended output selection, codec pin, and WirePlumber port and
without an intervening output toggle. The boot probe now also records the
root-cause `Front` switch for future samples. A guarded Fifine microphone test
measured its 997 Hz output 18.84 dB above a Front-muted negative control, with
exact route and volume restoration. Repeated cold-boot/suspend acceptance
remains. Evidence and transition matrices are documented in
[docs/DRIVER_ROUTING_INVESTIGATION.md](docs/DRIVER_ROUTING_INVESTIGATION.md).
The ineffective AE-5 What U Hear volume/mute controls, guarded measurements,
profile compatibility, and build-tested kernel candidate are documented in
[docs/RECORDING_MIXER_INVESTIGATION.md](docs/RECORDING_MIXER_INVESTIGATION.md).
Until that candidate is running, the app leaves those misleading controls
visible but read-only; new profiles omit them and legacy profiles ignore them.
The exact Nobara/upstream driver source, public research references, firmware
licence boundary, and pinned revisions are recorded in
[docs/SOURCE_INVENTORY.md](docs/SOURCE_INVENTORY.md).

The target host can safely isolate the AE-5 for Linux-guest kernel A/B tests.
Before any passthrough setup, run the non-mutating gate:

```sh
bash scripts/check-vfio-host.sh
```

The audited topology, package boundary, recovery rules, and per-kernel matrix
are in [docs/VFIO_TEST_PLAN.md](docs/VFIO_TEST_PLAN.md). A guest cannot replace
the final physical cold-boot and suspend tests.

The complete patch stack now also builds, boots, and passes a guarded physical
cycle on maintained Linux 6.18.40 LTS. That cycle covered the first-use/manual
route fix, safe packaged control write, package install/removal, and exact host
audio restoration. Reproduction commands and evidence are in
[docs/LTS_KERNEL_VALIDATION.md](docs/LTS_KERNEL_VALIDATION.md).

An additional upstream-based candidate now exposes the AE-5's five onboard
RGB LEDs through Linux's standard multicolor LED class without `/dev/mem` or
userspace MMIO. It passed strict source/build checks, a card-less boot, and a
managed physical cycle covering solid RGB frames, independent per-LED values,
brightness off/on, unchanged audio controls, and exact host recovery. Visual
color confirmation and a least-privilege GUI path remain before the feature is
complete; the external strip is not yet supported. The patch and evidence are
in [kernel/README.md](kernel/README.md) and
[docs/LTS_KERNEL_VALIDATION.md](docs/LTS_KERNEL_VALIDATION.md).

The named-headphone-tuning gap, why the packaged `ctspeq.bin` must not be
loaded on the AE-5, and the bounded driver experiment sequence are documented
in
[docs/HEADPHONE_TUNING_INVESTIGATION.md](docs/HEADPHONE_TUNING_INVESTIGATION.md).
The read-only address-query experiment was also built and run on the physical
AE-5. Request `60` received no reply both immediately after firmware download
and after the full AE-5 DSP setup, so neither run returned an address. Normal
playback, the known mixer state, and exact host recovery remained intact. This
negative result does not justify guessing other protocol fields or uploading
the Chromebook SpeakerEQ image.

The hardware audit also found independent upstream CA0132 Wedge Angle and
factory-EQ cache bugs, plus an unbounded DSP fast-load parser. The repository
carries minimal Wedge Angle and EQ cache fixes, a separately reviewable
parser-hardening candidate with KUnit coverage, and the physically tested
read-only probe.
Evidence, proposed commit messages, and validation steps are in
[kernel/README.md](kernel/README.md). The four functional patches now build
and boot together in both session and system Fedora KVM guests. Five managed
physical-card cycles also passed: the DSP firmware loaded, Wedge initialized
to `30` and read back at both boundaries, the ineffective What U Hear control
disappeared while its PCM remained, every factory EQ cache vector and
notification matched, and low-gain headphone playback measured 19.59 dB above
a Front-muted control. The patched What U Hear PCM captured the same fixture,
three warm guest reboots restored the exact state, and 50 alternating output
selections produced the expected speaker/headphone codec pins. The guest and
host recovered their exact mixer hashes after shutdown. The diagnostic read
probe remains separate from the functional patch stack; its two later guarded
boots returned no address and also recovered exactly. Because the host still
runs the unpatched Nobara kernel, AE-5 Control continues to display its invalid
Wedge value as a driver warning and excludes it from newly captured profiles.

Objective Windows/Linux level, frequency-response, and noise comparison is
documented in
[docs/AUDIO_PARITY_MEASUREMENT.md](docs/AUDIO_PARITY_MEASUREMENT.md). The
SoX-based harness generates one hash-verified reference set and compares
unaltered 48 kHz/24-bit stereo captures without changing mixer controls.
Physical digital-loopback measurements for isolated output effects and all ten
EQ bands, plus every factory EQ preset, are in
[docs/DSP_EFFECT_MEASUREMENT.md](docs/DSP_EFFECT_MEASUREMENT.md).
