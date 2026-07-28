# AE-5 Linux Control

Linux control software and upstream driver fixes for the Creative Sound
BlasterX AE-5, developed from public source and reproducible hardware evidence.

**New maintainer or reviewer:** start with
[`HANDOVER.md`](HANDOVER.md). It identifies the authoritative development
branch, current host state, non-negotiable audio-safety rules, known blockers,
and the shortest validation path. Then use [`ROADMAP.md`](ROADMAP.md) for the
single active milestone, completion criteria, and bounded test-rerun policy.
This README retains cumulative milestone history and should not be read as a
single current-state report.

## Current sound-safety status

The usable Linux path now fails closed around the confirmed corruption
triggers. Hardware OutFX, its child output effects, hardware EQ, and Direct
Mode are blocked before an ALSA write. The production kernel queue also
initializes OutFX off and rejects an enable request with `-EOPNOTSUPP`.
Windows Command's OutFX is a software APO master switch; it is not equivalent
to Linux's unsafe CA0132 hardware control.

Waveform-qualified tests found a second trigger: clearing and later
reassigning the AE-5 HDA playback converter could change a clean stream into
approximately 26.4% THD even with OutFX off. The current kernel queue fixes
that lifetime error and prevents AE-5 runtime autosuspend from invalidating
the retained assignment. It completed 50/50 clean reopens after a fresh
host-driver-to-VFIO boot, plus clean warm, idle, 48/96 kHz, and 2/6-channel
matrices. A rejected hardware-OutFX enable attempt left eight further captures
clean. The exact `7.1.4-ae5-stable` RPM then passed a fresh packaged-kernel
boot, first-open capture, 12 warm reopens, a 20-second idle transition, and
eight captures after an exact rejected OutFX write. The WirePlumber no-suspend
rule remains defense in depth.

The committed fail-closed host harness independently repeated that physical
card matrix in both the stable-kernel passthrough guest and a true motherboard
power-removal boot. The bare-metal run accepted 22 internal captures at
0.002720620–0.002724749% THD with one exact 3.130447865% peak: first open, 12
immediate reopens, one reopen after 20 seconds idle, and eight after OutFX
enable was rejected with `EOPNOTSUPP`. It requires explicit confirmation that
every AE-5 analog output is unplugged, enforces Master and Front off plus Low
gain, uses a −30 dBFS fixture, and treats 1% THD as corruption. Cleanup
restored 5% muted, both hardware switches off, Low gain, OutFX off, and both
PCMs closed.

Before that cold boot, the user heard the same fault after warm-booting into
Windows; only complete power removal cleared it. This is user-reported rather
than instrumented evidence, but it places that incident below a
Linux/PipeWire-only boundary and is consistent with persistent AE-5 DSP or PCI
power state. Source tracing then found that driver removal resets the CA0132
DSP but the generic HDA warm-shutdown path does not. A ninth, not-yet-installed
queue candidate adds an AE-5-only codec shutdown reset so Linux does not hand
the next operating system a running DSP. Its evidence and remaining
warm-handoff gate are in
[docs/WARM_REBOOT_DSP_RESET.md](docs/WARM_REBOOT_DSP_RESET.md).

This is an internally captured digital result. The AE-5 analog outputs were
unplugged and the user's headphones were connected to the motherboard
line-out. A safe AE-5 analog capture and runtime Windows same-settings capture
remain pending. The compact evidence is in
[docs/PCM_REOPEN_EVIDENCE.md](docs/PCM_REOPEN_EVIDENCE.md).

Earlier Direct Mode patch stacks produced host-configured side-by-side RPMs
and passed cardless boot gates. Those artifacts are now historical and must
not be installed: they contain Direct Mode and predate the OutFX guard. See
[docs/HOST_KERNEL_BUILD.md](docs/HOST_KERNEL_BUILD.md) only for build-history
evidence.

The current nine-patch queue applies cleanly to ALSA `for-next`, excludes
Direct Mode, and includes the OutFX guard, stable-playback fix, and the
source-, object-, and package-validated warm-shutdown reset candidate. Its
separate `7.1.4-ae5-shutdown` RPM passed non-installing package verification
and is installed side by side for a guarded one-shot boot, but has not been
booted or physically accepted. The verified `7.1.4-ae5-stable` RPM contains
the first eight patches and remains the running kernel; the stock Nobara
kernel remains the saved default. Use the ninth-patch package only for its
documented guarded runtime gate.
The previously installed `7.1.4-ae5-guarded` artifact predates the
stable-playback patch and is historical. Reproducible hashes, validation
evidence, and the physical acceptance sequence are in
[docs/KERNEL_MAINTENANCE.md](docs/KERNEL_MAINTENANCE.md).

## Current milestone: guarded persistent playback, profiles, and onboard lighting

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

The complete Sound Blaster Command compatibility ledger is also embedded in
the binary. It can be read without a card present and filtered to the features
that are unavailable or still waiting for physical acceptance:

```sh
cargo run -- features
cargo run -- features unsupported
cargo run -- features deferred
```

Self-test the physical playback acceptance instrument without touching the
card:

```sh
bash scripts/check-ae5-playback-stability.sh --self-test
bash scripts/check-software-eq-performance.sh --self-test
```

The output names the Linux mechanism, current evidence, and remaining gate for
every tracked feature. It is generated from `feature-parity.tsv`, so the CLI,
GUI, and project evidence cannot silently disagree.

PipeWire may prefer other playback and recording devices even when the AE-5 is
detected. Inspect the mapped nodes or explicitly make either one the desktop
default through WirePlumber:

```sh
cargo run -- output-status
cargo run -- route-status
cargo run -- route-repair
cargo run -- set-default-output
cargo run -- input-status
cargo run -- set-default-input
```

`route-status` is read-only and exits nonzero when the ALSA `Output Select` or
`Input Source` choice disagrees with PipeWire's active hardware routes, or
when normal-mode Headphone output has a muted or unreadable `Front` playback
switch. `route-repair` remains explicit and may repair the input or unmute the
current headphone DAC. Direct output and speaker-layout changes are enabled
only on the exact clean `7.1.4-ae5-stable` kernel; other kernels remain
fail-closed. Profile application still retains output-route metadata without
applying it to hardware. The GTK Device page uses the same guards.
Nothing repairs or unmutes a route automatically at login.
The default-device actions invoke `wpctl` directly without a shell and verify
the new default. They do not change the card's ALSA mixer controls. CLI status
and the GTK System audio page also show the PipeWire node volume separately;
with the installed soft-mixer profile, it is the only user-facing playback
volume. The card-specific Headphone path pins Master, Front, and PCM to their
0 dB values so their attenuation cannot stack underneath PipeWire. The GUI
labels that fixed Master stage as `0 dB`, not `100%`.

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

Phase A of the software-effects path can generate a ten-band PipeWire
equalizer from any native or imported profile:

```sh
ae5ctl eq-chain-status
ae5ctl eq-chain-enable ~/.config/ae5-control/profiles/example.json
ae5ctl eq-chain-response 48000
ae5ctl eq-chain-activate
ae5ctl eq-chain-disable
```

Enabling saves only a managed AE-5 Control state file. Activation suspends the
exact current AE-5 sink, hot-loads the graph into that sink's PipeWire
`audioconvert` stage, verifies a runtime signature marker, and resumes it. It
does not create a virtual sink, change the desktop default, stack a second
cubic volume control, or require a PipeWire restart. Activation requires
OutFX to remain readable and off and the physical target to remain current.

The ten imported bands receive a deterministic automatic preamp calculated
from the combined response at 44.1, 48, and 96 kHz plus 0.25 dB margin. The
GTK Equalizer page applies a chosen profile in one action and exposes the
saved/runtime state and preamp. The real muted and unplugged AE-5 accepted the
full imported headphone graph at −10.80 dB preamp while both playback PCMs
remained closed; unload restored the prior PipeWire device/routing and ALSA
state. On the true power-removal stable-kernel boot, two neutral and two
equalized 48 kHz What U Hear captures repeated within 0.00 dB at every band.
The measured equalized-minus-neutral response matched the requested graph
within 0.34 dB, including automatic preamp, across 31 Hz–16 kHz. A guarded
physical-card benchmark kept the same sink and 2048-frame quantum, adding no
PipeWire buffer and 0.3990 percentage points of process CPU; filter work added
178.564 µs, or 0.4185% of each 42.7 ms quantum. The 7200-second nonzero
qualification recorded 7197 zero-error samples, 200.430 µs mean and 267.900 µs
maximum sink work, and exact state recovery. Broader-rate/preset coverage and
a verified post-EQ Windows measurement remain pending. Windows `What U Hear`
was proven post-Acoustic-Engine but did not contain Command's displayed
graphic-EQ curve; see
[docs/SOFTWARE_EFFECTS_PLAN.md](docs/SOFTWARE_EFFECTS_PLAN.md) and the
[Windows result](docs/windows-capture/VM-OUTFX-RESULTS.md).
The response command emits the exact ten-band prediction used by
`scripts/audio-parity.sh compare-eq` for the physical acceptance capture.
`scripts/check-software-eq-performance.sh` provides the corresponding
fail-closed benchmark and soak gate.

With the onboard-LED kernel candidate and the packaged device rule installed,
the same unprivileged desktop user can inspect, set, and persist the five
onboard RGB colors:

```sh
ae5ctl lighting-status
ae5ctl lighting-set 255 0 0
ae5ctl lighting-set-led 3 0 0 255
ae5ctl lighting-restore
```

Colors are range-checked, read back through Linux's multicolor LED class, and
saved in `~/.config/ae5-control/lighting.json`. A hidden desktop autostart
entry restores that file after login. The package grants writes only to
`brightness` and `multi_intensity` on the exact five
`1102:0012/1102:0051` AE-5 LED devices; it installs no daemon or privileged
helper. The commands report that kernel support is unavailable on an
unpatched kernel.

Typed write commands validate choices and ranges and verify the value by
reading it back. `Output Select` and `Input Source` use the matching
WirePlumber port from the packaged AE-5 profile, while `Surround Channel
Config` selects the exact stereo, 2.1, 4.0, 4.1, or 5.1 PipeWire profile.
The shared transaction verifies both layers and rolls back on failure. The
packaged card rule also enables PipeWire software volume and persistent
playback for this exact AE-5 so desktop clients cannot reload unsafe hardware
gains or close and reopen the codec stream. Safe controls write directly
through ALSA; unsafe hardware output-processing controls are retained only as
profile metadata and rejected:

```sh
cargo run -- get "Output Select"
cargo run -- set-choice "Output Select" Headphone
cargo run -- set-playback-switch "FX: Surround" off # rejected: software path only
cargo run -- set-playback-channel-level Front "Front Right" 82
```

Output-route changes are enabled only on the exact clean, physically
qualified `7.1.4-ae5-stable` kernel; unpatched kernels still reject them. The
setter resolves the current PipeWire route by exact name rather than a
position-dependent index, mutes before changing it, and restores the previous
software volume and mute state afterward. A live Speakers → Headphones cycle
retained 5% muted, OutFX off, and both playback PCMs closed.

High headphone gain is rejected unless `--allow-high-gain` is supplied. The
hardware smoke test changes the disabled, route-safe Bass Redirection
Crossover value, verifies it, and restores the original value:

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
cargo run -- profile-rename headphones.json "Late night"
cargo run -- profile-show headphones.json
cargo run -- profile-check headphones.json
cargo run -- profile-apply headphones.json
```

The GTK Sound Effects page also embeds all 33 selectable factory profiles from
the validated AE-5 Sound Blaster Command 3.5.10.0 installation. Built-ins are
immutable and always available: applying one previews the exact native
controls, chooses its speaker or headphone section from the live route, and
maps Windows Bass to CA0132 Bass Redirection for an active `.1` speaker layout
without changing the selected output. Only converted names, identifiers, and
representable values are included; no Creative artwork, binaries, raw files,
or user settings are distributed. Provenance is recorded in
[data/README.md](data/README.md).

AE-5 Control also exposes an evidence-based Linux-driver processing baseline.
It is not labeled as Creative's factory reset because Sound Blaster Command's
exact reset semantics are undocumented. Previewing and checking are read-only:

```sh
cargo run -- linux-defaults-show
cargo run -- linux-defaults-check
```

An apply requires both an explicit confirmation flag and a new backup path.
Before creating that file or writing a control, the reset verifies that every
targeted current value can be represented by a profile and restored on the
live driver. The previous valid mixer state is then saved, and the normal
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

`profile-rename` changes only the saved display name, not the filename or live
hardware. It accepts a library filename shown by `profile-library`, trims
surrounding whitespace, writes atomically, and rejects symlinks or paths
outside the library.

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
user folder. In the validated Command 3.5.10.0 build, the application's export
actions copy these stored profile and EQ JSON files byte-for-byte; the
interoperability evidence and input hashes are recorded in
[docs/WINDOWS_MIGRATION_VALIDATION.md](docs/WINDOWS_MIGRATION_VALIDATION.md).

This flow follows the selected profile and EQ IDs, preserves the output route,
and maps standard Windows speaker masks from stereo through 5.1. It reads only
plain XML string settings. Binary-serialized application state is never
deserialized. The Desktop speaker category is recorded as an exact no-op
because Command uses it only to choose the separately represented `Bass.XOver`
value. Named Creative headphone tuning remains unsupported until Linux exposes
a safe equivalent; the importer resolves its display model from Command's
bounded text metadata so the warning identifies what was selected. Command's
model records contain only display name and order; the actual correction
response remains in the proprietary Windows driver/APO. The Playback page
states that boundary instead of presenting a non-functional model selector.
Personal graphic-EQ presets remain fully importable through the native profile
library. Command's
shared Bass feature uses X-Bass for headphones and speaker layouts without a
subwoofer, but switches to Bass Management for 2.1, 4.1, and 5.1. The active
setup importer mirrors that behavior with CA0132's `Bass Redirection` and
`Bass Redirection Crossover` controls while explicitly turning X-Bass off.
Live Command 3.5.10.0 validation with the physical AE-5 established that
profile section type `0` means headphones and type `1` means speakers; focused
tests pin that mapping.

The importer maps SBX switches and levels, crossover frequency, Smart Volume
mode, and all ten EQ bands. It selects the driver's Flat preset before custom
bands so a prior factory curve cannot leak into the migrated settings. Before
saving, it separates exact mappings, values rounded to ALSA steps, and
unsupported non-null source settings. The CLI prints the complete report and
the desktop preview lists every unsupported field. Unsupported settings such
as a non-zero EQ preamp are skipped while the representable controls are
retained; invalid products, files, ranges, units, band counts, and frequencies
are still rejected. Explicitly disabled Scout settings and a false subwoofer
gain flag are reported as exact no-ops; configured or enabled values remain
unsupported. Zero-valued `SpeakerMethod`, `Surround.Mode`,
`DialogPlus.Mode`, and `SVM.PlusMode` defaults are also exact no-ops: the
first selects a Windows routing API, while Creative's own profile path applies
the latter three only to Katana. Unexpected nonzero values remain unsupported
for review.

The CA0132 driver exposes a Smart Volume level plus Normal, Loud, and Night
modes. Only Normal uses the adjustable level; Loud and Night select fixed DSP
values. The desktop app therefore keeps their mode and enable controls active
but disables the ineffective level slider until Normal is selected. Saved and
imported profiles still retain that value for a later return to Normal. A
separate kernel candidate now restores the cached Smart Volume level and mode
after a real DSP-losing resume. Its complete host RPM has passed non-installing
verification, a no-audio QEMU smoke boot, and a cardless full-root boot with
automatic fallback, but still needs the guarded physical suspend test
documented in [`kernel/README.md`](kernel/README.md).

## Native desktop application

The GTK 4 application groups device diagnostics, Command compatibility, system
audio, onboard lighting, profiles, playback, effects, equalizer, and recording
into dedicated pages. The **Compatibility** page summarizes verified,
Linux-native, pending, and unavailable features, then exposes the evidence and
remaining gate for every pending or unavailable item without touching the
hardware. The
**Device** page shows the exact detected hardware, live capability counts, and
driver values outside their advertised ranges. It can save the same
privacy-conscious diagnostics report as `ae5-collect-report` without invoking a
shell or requiring root. During pre-release validation, the application writes
a structured operation trace to the bounded user journal by default. It records
startup, route/profile requests, checked mixer writes, software-EQ lifecycle,
refreshes, and errors without audio or profile contents; set `AE5_TRACE=0` to
opt out. The diagnostics report includes at most the latest 500 current-boot
trace lines. The **System audio** page can make the AE-5 the default
PipeWire playback or recording device and opt into native-rate switching
without changing its ALSA mixer controls.
The **Equalizer** page separates the saved CA0132 hardware EQ state from the
PipeWire software path. It labels a hardware EQ child setting as `ARMED` when
OutFX is off instead of claiming that it is processing audio.

The **Lighting** page uses native GTK color dialogs for a unified color or five
individual LED colors. It shares the CLI's verified, transactional backend and
reverts the displayed color when a hardware or persistence write fails. The
unchanged release GUI has exercised that native dialog in a real KDE/Wayland
desktop against five private LED-class fixtures: unified and individual
changes, cancellation, JSON persistence, restore, and cold GUI readback all
passed without changing the host's `/sys` mount or audio state.

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
polling loop while the selected page remains open. The installed native
Wayland build passed this on the physical card: with Surround disabled, an
external CLI changed only its latent level from 0 to 1 and the GUI exposed
the synchronized state through AT-SPI. Ten external 1-to-0 cycles retained
exactly one watcher thread, restored the complete mixer hash, and opened no
PCM:

```sh
sudo dnf install gtk4-devel
cargo run --features gui --bin ae5-control
```

The Device page also reports the running kernel release and taint state, then
distinguishes the normal stock-compatible path from the project kernel's
Direct Mode and five-device onboard-lighting interfaces. This is a read-only
readiness summary, not a substitute for physical kernel acceptance.

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
CLI, desktop entry, AppStream metadata, icon, scoped onboard-LED device rule,
and login-time color restore without a privileged helper.
A clean Fedora 44 build/install/verify/remove transaction is now enforced in
pull-request CI, and a read-only run of an exact RPM payload on the physical
AE-5 passed.

When system package installation is unavailable, build and install the same
application for the current desktop user:

```sh
bash scripts/install-user.sh
# Later, if wanted:
ae5-control-user-install --uninstall
```

This rootless path installs the binaries, application-menu metadata,
login-time lighting restore, and card-scoped WirePlumber/ACP configuration
under the normal XDG user directories. It refuses conflicting files and stages
a complete verified payload before replacing an earlier user installation.
Rerun the same command to upgrade; profiles and lighting settings are
preserved by upgrades and removal. It cannot install a kernel patch or udev
permissions, so onboard-lighting writes still require the system package's
exact rule. The isolated lifecycle check runs in CI, and the reference host has
launched the installed application from its desktop entry with an unchanged
mixer and route state. Full evidence and the remaining authenticated-RPM gate
are in
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

Collect the exact application binary fingerprints and installation scope,
actual card identity, kernel taint and failed-unit state, driver and module
metadata, ALSA controls, onboard LED-class state, codec data, and relevant
kernel log with the installed package:

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
unsupported; deferred rows name the acceptance evidence still required. The
installed `ae5ctl features` command and GUI Compatibility page embed this same
validated matrix.

The reported first-use headphone failure is now reproduced. PipeWire's generic
headphone route muted the CA0132 `Front` DAC even though the AE-5 headphones
share it. The RPM supplies a card-scoped ACP headphone path that keeps Front
enabled and exact Microphone, Front Microphone, and Line In routes; all three
input ports selected the matching ALSA enum in a physical-card matrix. The
fixed headphone route survived a WirePlumber restart and one instrumented cold
boot with the intended output selection, codec pin, and WirePlumber port and
without an intervening output toggle. The boot probe now waits for the
root-cause `Front` switch and every other required route control in one
complete snapshot, then reports progress with
`bash scripts/collect-routing-state.sh --summary 10`. Guarded Fifine
microphone tests measured the fixed route 18.84 dB above a Front-muted
negative control, then measured an independent installed-CLI
Speakers→Headphone cycle 10.88 dB above both its quiet and muted controls.
Both restored the exact persistent mixer, route, and volume state. Repeated
connected-headphone cold-boot and suspend/resume acceptance remains. A silent,
user-driven suspend probe now
rejects any playback stage above 20%, non-Low headphone gain, open PCM, wrong
route, unreadable evidence, changed mixer state, changed boot/kernel, or new
audio warning; it never suspends or plays audio itself. Run its paired
`--before-suspend campaign-01` and `--after-resume campaign-01` captures, then
check progress with `--suspend-summary 20`. A standalone read-only snapshot is
available through `--preflight ID`; it validates the same safety conditions
without appending campaign state. For connected-headphone route campaigns,
run
`bash scripts/check-ae5-kernel-runtime.sh 7.1.4-ae5-stable` before changing
any control. It additionally requires the exact untainted kernel, signed
matching CA0132 module, AE-5 PCI identity and driver, Direct Mode absent,
OutFX off, and all five onboard LED interfaces. The Rust CLI and GTK
diagnostics also read
PipeWire's live Route parameters: deliberately recreated
Headphone-versus-Line-Out and Microphone-versus-Line-In splits failed
`route-status`, while the normal setters repaired both and restored the exact
mixer hash. Route health also rejects normal-mode Headphone output when the
shared Front DAC is muted or unreadable, so the original silent state cannot
be reported as healthy merely because the route names agree. Reapplying the
already-selected Headphone value deliberately preserves a muted Front switch,
so the CLI `route-repair` command and the conditional GTK action provide the
separate, explicit recovery path. Both repaired a guarded real-card negative
test, returned the raw mixer to its exact starting hash, kept PipeWire at the
20% ceiling, and opened no PCM. Route writes still require the analog PCM to
close first; the bounded wait allows five seconds for WirePlumber startup to
settle and otherwise fails without touching the mixer. Historical boot records
that predate Front collection are reported as unavailable instead of being
mislabelled as muted. A later silent real-card matrix synchronized
2.0, 2.1, 4.0, 4.1, and 5.1 with
`analog-stereo`, `analog-surround-21`,
`analog-surround-40`, `analog-surround-41`, and `analog-surround-51`,
preserving the duplex input profile and every hardware gain. It also exposed
and fixed two unsafe ACP interactions: hardware volume ownership reloaded
saved route gains, and the old headphone path interpreted `volume=zero` as
0 dB. The package now requires `api.alsa.soft-mixer=true` and uses
`volume=ignore`; the backend refuses managed routing until that policy is
active. Every matrix stage kept PipeWire at 0%, ALSA at or below 20%, Low
gain, and all PCM devices closed; no sound was played. Evidence and transition
matrices are documented in
[docs/DRIVER_ROUTING_INVESTIGATION.md](docs/DRIVER_ROUTING_INVESTIGATION.md).
The ineffective AE-5 What U Hear volume/mute controls, guarded measurements,
profile compatibility, and build-tested kernel candidate are documented in
[docs/RECORDING_MIXER_INVESTIGATION.md](docs/RECORDING_MIXER_INVESTIGATION.md).
Until that candidate is running, the app leaves those misleading controls
visible but read-only; new profiles omit them and legacy profiles ignore them.
The exact Nobara/upstream driver source, public research references, firmware
licence boundary, and pinned revisions are recorded in
[docs/SOURCE_INVENTORY.md](docs/SOURCE_INVENTORY.md).

The earlier AE-5 Direct Mode candidate is now a historical diagnostic patch,
not part of `kernel/series`. Later waveform-qualified testing proved that
normal playback close/reopen can corrupt the CA0132 route, and
Direct-to-normal transitions cross that unsafe boundary. The Rust CLI and GTK
app reject Direct Mode even if an older patched kernel exposes the control.
Past behavior research remains in
[docs/DIRECT_MODE_INVESTIGATION.md](docs/DIRECT_MODE_INVESTIGATION.md), with a
correction banner and the gate required before reconsidering it.

The target host can safely isolate the AE-5 for Linux-guest kernel A/B tests.
Before any passthrough setup, run the non-mutating gate:

```sh
bash scripts/check-vfio-host.sh
```

The audited topology, package boundary, recovery rules, and per-kernel matrix
are in [docs/VFIO_TEST_PLAN.md](docs/VFIO_TEST_PLAN.md). A guest cannot replace
the connected-output and physical suspend/resume tests.

A separate Windows 11 comparison VM is installed with the exact Creative
Command `3.5.10.0` build and a file-by-file-verified copy of the saved AE-5
settings. Managed system passthrough now assigns the physical card
successfully; Creative's signed `6.0.105.65` driver reports healthy devices,
Command recognizes the complete AE-5 UI, and both route-specific imported
settings views match the corrected Linux conversion. Read-only transfer
attributes had to be cleared on destination user files before Command could
update `user.config`.

All physical outputs stayed unplugged and no audio was played during that
validation. A Speakers-to-Headphones transition caused Command to unmute the
Windows render endpoint, despite its prior muted state. The Windows capture
procedure therefore reapplies and independently verifies the 20% cap and mute
after every output, profile, format, or device transition. A guarded 997 Hz
Windows/Linux screen subsequently exposed and fixed the silent normal
PipeWire transport: the AE-5 requires raw read/write playback, its working
6016-by-four buffer geometry, and ignored ALSA dB metadata. A later
format-controlled retest showed that S32 preserves more software-volume
precision, but a later real track change produced a loud buzz until the PCM
suspended. The desktop sink therefore stays on the previously stable S16
transport while S32 transition safety is investigated. Linux playback is
audible through the normal desktop sink. A preliminary capture initially
appeared 16.50 dB below Windows, but the levels were not matched: ALSA's
virtual Master attenuation was stacked with low Front and PCM values. Kernel
source review and a repeatable Master 19/20 A/B explain the result, so it is
not evidence of a 16.50 dB driver-performance gap. Full matched electrical
response/noise parity is still pending. A later guarded cycle recovered the
current private test credential from its local setup record without exposing
it, logged in, and removed the temporary automatic-login values immediately.
At 5% endpoint and application volume, counterbalanced 48 kHz `What U Hear`
captures were perfectly repeatable in neutral and changed strongly with the
imported Acoustic Engine profile. This proves that the endpoint is post-OutFX
for that profile. Two forced graphic-EQ captures did not contain Command's
displayed curve and missed the Linux model by up to 13.08 dB, so the endpoint
is explicitly rejected for EQ parity. The guest then shut down cleanly, the
AE-5 returned to `snd_hda_intel`, and Linux recovered at 5% muted with both
playback PCMs closed. See the
[complete Windows result](docs/windows-capture/VM-OUTFX-RESULTS.md).
No source NTFS volume, Windows image, Creative binary, private setting, or
credential is stored in the repository.

A diagnostic transition harness covers complete close/reopen, abrupt
disconnect, different-rate and different-format replacement clients, a
client-owned in-place S16/S32 and 44.1/48/96 kHz probe, gapless overlap, and
the PipeWire suspend boundary while targeting only the exact AE-5 sink. It
continuously verifies the two hardware mutes and pairs its
client/PCM/PipeWire timeline with the kernel's existing HDA lifecycle and DMA
position tracepoints. The in-place helper's unlinked mode passed with zero
links and both AE-5 playback PCMs closed. Linked S16/48 and S32/96 virtual
sinks each completed all four updates on one stable node and the same two
links. This validates the client path, not S32 safety on the AE-5. Inspect the
live prerequisites without changing state:

```sh
bash scripts/track-transition-stress.sh --dry-run
```

Two exact-target five-trial S16 campaigns ran at 5%, but neither captured the
waveform. Their clean client exits and logs did not prove clean audio. Later
What U Hear captures showed that the forced suspend-boundary PCM closures were
the corruption trigger. Production now sets the exact AE-5 sink's suspend
timeout to zero; the harness remains for isolated reproduction and is not a
production acceptance gate. The current headphones are connected to the
motherboard line-out and nothing is connected to an AE-5 output. The
correction, root-only trace companion, and evidence schema are documented in
[docs/TRACK_TRANSITION_INVESTIGATION.md](docs/TRACK_TRANSITION_INVESTIGATION.md).

The complete patch stack now also builds, boots, and passes a guarded physical
cycle on maintained Linux 6.18.40 LTS. That cycle covered the first-use/manual
route fix, safe packaged control write, package install/removal, and exact host
audio restoration. Reproduction commands and evidence are in
[docs/LTS_KERNEL_VALIDATION.md](docs/LTS_KERNEL_VALIDATION.md).

The same maintained RGB kernel later passed an external headphone
level/mute/gain matrix. Two five-step Master changes tracked their advertised
gain within `0.66 dB`, Master mute reached the quiet baseline, a confirmed
Front mute suppressed the fixture by more than 34 dB, and Low, Medium, and
attenuated High gain produced distinct repeatable levels. The microphone path
did not have enough 18 kHz signal-to-noise ratio to compare DAC filters, so
that gate remains assigned to an attenuated electrical capture. Guest and host
state restored exactly after the cycle.

An additional upstream-based candidate now exposes the AE-5's five onboard
RGB LEDs through Linux's standard multicolor LED class without `/dev/mem` or
userspace MMIO. It passed strict source/build checks, a card-less boot, and a
managed physical cycle covering solid RGB frames, independent per-LED values,
brightness off/on, unchanged audio controls, and exact host recovery. A second
physical cycle installed the exact RPM, changed and persisted colors as an
unprivileged user, exercised hardware and file-write rollback, restored saved
colors, and returned the scoped sysfs permissions to their original mode on
uninstall. A separate real-desktop test exercised the native GTK chooser,
verified unified and per-LED file writes, proved Cancel made no change, and
reloaded the saved pattern after restart through an isolated LED-class
fixture. Visible confirmation on the physical card remains before the feature
is complete; the external strip is not yet supported. The patch and evidence
are in
[kernel/README.md](kernel/README.md) and
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
unaltered 48 kHz/24-bit stereo captures without changing mixer controls. The
same set includes a peak-checked six-channel identification WAV for speaker
layout tests; generated `speaker-test` signals are not used. Physical
digital-loopback measurements for isolated output effects and all ten EQ
bands, plus every factory EQ preset, are in
[docs/DSP_EFFECT_MEASUREMENT.md](docs/DSP_EFFECT_MEASUREMENT.md).
