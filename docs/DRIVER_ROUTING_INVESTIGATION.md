# CA0132 cold-boot routing investigation

This investigation concerns the report that AE-5 headphone output sometimes
starts working only after selecting Line Out in the desktop and then selecting
Headphone in ALSA. The symptom is reproduced, and the failing layer is the
generic PipeWire ALSA Card Profile (ACP) headphone mixer path rather than the
CA0132 output-selection code.

## Current finding

The target system runs Nobara kernel `7.1.4-200.nobara.fc44.x86_64`. Linux 7.1
contains both recent upstream CA0132 routing fixes:

- [`778031e1658d`](https://github.com/torvalds/linux/commit/778031e1658d206a52bf9491c91ae5d4f4a2509d)
  initializes HP/Speaker auto-detect from the headphone pin's presence-detect
  capability.
- [`6fd9f6e870ea`](https://github.com/torvalds/linux/commit/6fd9f6e870ea285f05102e8e00e6a7f4495a9a02)
  makes a manual output selection disable auto-detect and apply the selected
  route even when the enum value was already selected.

The current upstream implementation is in
[`sound/hda/codecs/ca0132.c`](https://github.com/torvalds/linux/blob/master/sound/hda/codecs/ca0132.c).
The matching Nobara source RPM was verified, and its CA0132 source is
byte-identical to Linux stable `v7.1.4`; none of the packaged downstream
patches modifies CA0132. Exact hashes, revisions, licences, and the proprietary
analysis boundary are recorded in
[`SOURCE_INVENTORY.md`](SOURCE_INVENTORY.md).

On the failing boot:

- ALSA `Output Select` was `Headphone`.
- `HP/Speaker Auto Detect` was off.
- The read-only kernel `Headphone Jack` control was on.
- PipeWire selected its generic
  `analog-output-headphones;output-headphones` route.
- WirePlumber's ACP device had automatic profile and port selection disabled,
  so it restores an explicit route.
- `alsa-state.service` entered the active state roughly 1.4 seconds before the
  first PipeWire/WirePlumber startup recorded in the boot journal.
- On the first observed Linux boot there was no saved
  `/var/lib/alsa/asound.state`; one now exists after development and contains
  the manually selected headphone state.

The route was logically consistent but physically silent. The user recovered
audio by selecting PipeWire Line Out and then ALSA Headphone. A guarded
no-stream replay proved why: the generic ACP headphone path writes
`Front Playback Switch=off`, but the AE-5 headphone path shares that Front DAC.
Line Out writes `Front=on`, and the final ALSA Headphone selection routes that
enabled DAC to the headphone jack.

### First instrumented reboot

Boot `5c9efcee-2a1a-4cf3-ac07-bf5154ab6ef7` on 2026-07-24 provided the first
instrumented cold-boot sample:

- The kernel downloaded and started the CA0132 DSP without a relevant warning
  or error.
- The pre-PipeWire service found ALSA card 0, but its three `amixer` reads
  failed with `Invalid card number '0'`. The sysfs card was visible before the
  ALSA control device was ready for that user process, so this sample does not
  establish the pre-PipeWire logical route.
- After PipeWire started, ALSA reported `Headphone`, auto-detect off, and the
  headphone jack on. PipeWire selected the matching headphone port.
- Codec pins matched the driver's intended AE-5 headphone route: line-out
  `0x0b`, surround `0x0f`, and front headphone/center-LFE `0x10` were off,
  while rear headphone `0x11` was enabled for output.

The user subsequently confirmed that this internally consistent state was
silent on the same boot. The before and after snapshots share boot ID
`5c9efcee-2a1a-4cf3-ac07-bf5154ab6ef7`; codec pins and ALSA output selection
were unchanged after recovery. The post-recovery PipeWire route instead
identified Line Out, and `Front` was on.

This reboot also exposed two probe defects: it queried ALSA too early and
omitted the AE-5 rear-headphone pin `0x11`.

## Confirmed ACP root cause and fix

With no playback stream open, the exact transition matrix was:

| Step | PipeWire route | Output Select | Front switch | Master |
|---|---|---|---|---|
| Known working state | Line Out / Speaker | Headphone | on | 87 |
| Select generic Headphones | Headphones / Headphones | Headphone | **off** | 76 |
| Select Line Out | Line Out / Speaker | Speakers | on | 87 |
| Select ALSA Headphone | Line Out / Speaker | Headphone | on | 87 |

The upstream generic
`analog-output-headphones.conf` deliberately defines `[Element Front]` with
`switch=off`, because it assumes Front belongs only to speakers or line out.
That assumption is false for CA0132 desktop cards.

The package now supplies:

- `sound-blaster-ae5-output-headphones.conf`, which includes the generic path
  but changes only Front to `switch=mute` and `volume=zero`;
- `sound-blaster-ae5.conf`, which replaces the generic headphone path in the
  analog stereo mappings;
- a WirePlumber rule selecting that profile set only for PCI Creative
  `1102:0012` cards.

The live profile was parsed by PipeWire's `spa-acp-tool`, exposed the fixed
headphone route, and passed Speakers → Headphones switching with
`Output Select=Headphone` and `Front=on`. It also survived a WirePlumber
restart and one instrumented cold boot with the intended output selection,
codec pin, and WirePlumber port. The acoustic microphone check below also
passed. Repeated cold-boot/suspend testing remains.

### AE-5 Control route ownership

Direct ALSA writes can recreate a split state even with the fixed profile:
`Output Select` or `Input Source` changes immediately, while WirePlumber still
remembers the old desktop port. AE-5 Control now sends those five choices
through the card-scoped WirePlumber routes instead. The shared setter is used
by the CLI, GTK application, native profiles, rollback, and imported profiles.
It refuses the managed action unless the live node confirms
`sound-blaster-ae5.conf`; every unrelated enum remains a direct ALSA control.

A no-stream physical-card matrix deliberately changed the ALSA enum behind
WirePlumber, then used the rebuilt CLI:

| Requested choice | PipeWire port | ALSA result |
|---|---|---|
| Speakers | `analog-output-lineout;output-speaker` | `Speakers`, Front on |
| Headphone | `sound-blaster-ae5-output-headphones;output-headphones` | `Headphone`, Front on |
| Microphone | `sound-blaster-ae5-input-microphone` | `Microphone` |
| Front Microphone | `sound-blaster-ae5-input-front-microphone` | `Front Microphone` |
| Line In | `sound-blaster-ae5-input-line-in` | `Line In` |

A separate two-control native profile changed Headphone/Microphone to
Speakers/Line In and restored it. Both matrices returned the complete ALSA
mixer to SHA-256
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`
and WirePlumber route state to
`76dd10cc599da7cc4d310c32c028fe6e008d980eeb7d992ef3c6f23ba09babd6`.
No audio stream was open.

The application now also reads the active PipeWire device `Route`, profile,
and `device.profile-set` through `pw-dump` JSON. This is exposed without
writes in both the GTK **Desktop route health** card and:

```sh
ae5ctl route-status
```

On the physical host, a controlled negative test selected PipeWire's
`analog-output-lineout;output-speaker` route and then changed only raw ALSA
back to Headphone. `route-status` printed both sides of the split and exited
1. Reapplying Headphone through the normal Rust setter restored
`sound-blaster-ae5-output-headphones;output-headphones`; the command then
passed, no stream opened, defaults were unchanged, and the complete mixer
returned to SHA-256
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`.

The matching input negative test selected PipeWire's
`sound-blaster-ae5-input-line-in` route and then changed only raw ALSA back to
Microphone. `route-status` kept the healthy Headphone result, reported
`ALSA selects Microphone, but PipeWire uses
sound-blaster-ae5-input-line-in`, and exited 1. Reapplying Microphone through
the shared setter restored `sound-blaster-ae5-input-microphone`. The complete
mixer, both desktop defaults, both PipeWire routes, and the zero-stream state
matched their exact starting values afterward.

The check is deliberately limited to the validated analog-stereo profiles;
other profiles report that limitation rather than guessing route semantics.

WirePlumber documents `monitor.alsa.rules` as the supported mechanism for
updating ALSA-device properties, while ACP is responsible for profiles, ports,
and mixer settings:
[ALSA configuration](https://pipewire.pages.freedesktop.org/wireplumber/daemon/configuration/alsa.html).

### First post-fix instrumented cold boot

Boot `0b82f21b-a86a-47c3-973a-c8911ac07915` provided the first state-level
cold-boot check after installing the card-specific ACP path. Before PipeWire
started, the probe waited for the ALSA control interface and recorded
`Output Select=Headphone`, auto-detect off, the headphone jack present,
and only the AE-5 rear-headphone pin `0x11` enabled for output. Eight seconds
later, the same ALSA and codec route remained intact and WirePlumber selected
`sound-blaster-ae5-output-headphones;output-headphones`.

The CA0132 DSP initialized successfully, and the boot journal contained no
relevant CA0132, HDA, DSP, or timeout warning. No output toggle occurred
between the two probes. This proves output-selection, codec-pin, and
WirePlumber-port restoration for one cold boot. That probe version did not
record the root-cause `Front` switch, so the collector now includes it for
future boots. This result does not replace the remaining repeated-boot or
no-toggle acoustic checks.

### Acoustic headphone validation

On 2026-07-24, with the headphones placed beside the default Fifine USB
microphone and not worn, a guarded physical test compared:

1. a quiet room baseline;
2. the fixed AE-5 headphone route with `Front=90/on`;
3. the same route and fixture with `Front=off`.

The PipeWire sink was reduced from 43% to 15% before the final fixture. The
card's existing High headphone-gain setting was not changed. The source was a
two-second, 48 kHz, 24-bit stereo 997 Hz sine at -18 dBFS with 50 ms fades,
SHA-256
`5381b96a81c8526b0cc1138e3a1ed9cac1f06657bb110644a80b9f9f16701de4`.
Each Fifine capture was four seconds at 48 kHz/24-bit stereo. SoX measured the
left channel over the same 1.5-second fixture window; spectral power is the
mean of the 18 `stat -freq` bins centered at 996.09375 Hz.

| Condition | Overall RMS dBFS | 987-1007 Hz RMS dBFS | Mean 996 Hz power |
|---|---:|---:|---:|
| Baseline | -59.29 | -112.92 | 0.000677111 |
| Fixed headphone route, Front on | -57.98 | -99.68 | 0.353436333 |
| Negative control, Front off | -59.83 | -105.48 | 0.004616500 |

The fixed route's 996 Hz component was 27.18 dB above baseline and 18.84 dB
above the Front-muted negative control. A separate digital-silence check
confirmed that `Front=off` remained off before, during, and after opening a
PipeWire stream, so the negative control was not silently undone by route
activation.

The three final capture hashes were:

- baseline:
  `f0be09c5b721468641dd3aad4a2ce73994f29db9ee98a680782d5ac0d3dcf292`;
- fixed route:
  `aee3cdfdeaadf83394be6964b146c0f88629a12a66d9de98c61e07323ce0a675`;
- Front-muted control:
  `f0fc82a36dee38d54ecdc2652e8974454e054021161056901ed527f4995d9b97`.

The ambient recordings and generated fixtures were deleted after deriving
these values. The test restored the sink to 43%, `Output Select=Headphone`,
`Front=90/on`, the fixed headphone port, and the original High gain. The
Fifine remained the 100% default source, no stream remained open, and no
kernel or package was changed.

On 2026-07-25, a second physical check exercised the installed rootless
`ae5ctl` path rather than setting ALSA and PipeWire separately. With no stream
open, it selected Speakers and then Headphone. The corresponding desktop
ports changed to `analog-output-lineout;output-speaker` and then
`sound-blaster-ae5-output-headphones;output-headphones`; `Output Select`
read back exactly and `Front` remained on. The complete mixer was byte-equal
before and after that route cycle.

The same guarded 15% sink-volume setup then compared a quiet baseline, the
selected headphone route, and a `Front=off` negative control. The generated
997 Hz fixture had SHA-256
`943fe7eaf841b23afb9eadadc8b6cc19b47cc555ea9522721f192066a2cec38d`.

| Condition | 987-1007 Hz RMS dBFS |
|---|---:|
| Baseline | -113.98 |
| Installed-CLI headphone route | -103.10 |
| Negative control, Front off | -113.98 |

The installed-CLI route was 10.88 dB above both controls. The first immediate
post-stream snapshot retained ALSA's volatile `PCM Playback Channel Map` as
`FL,FR`; after the closed stream settled, it returned to the idle value and
the complete mixer again matched SHA-256
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`.
The AE-5/Fifine defaults, fixed port, and 43% sink volume were unchanged, no
stream or relevant kernel warning remained, and all ambient WAV data was
deleted. This independently proves audibility after an application route
cycle; it is not a cold-boot, DAC-filter, or Windows-parity measurement.

## Safe cold-boot probe

`collect-routing-state.sh` discovers the exact audited card by
`1102:0012/1102:0051`; it does not assume card 1. It reads the route controls,
jack state, all four AE-5 output pins, service timing, and the matching
PipeWire card/sink. It waits up to five seconds for one complete ALSA snapshot:
card metadata, `Output Select`, `Front`, auto-detect, and jack state must all
be readable in the same attempt. Merely opening the card is not sufficient.
The probe never writes a mixer control.

Install two temporary user services:

```sh
bash scripts/install-routing-probe.sh
```

The first snapshot is ordered before the PipeWire process. It deliberately
skips `pactl`, because connecting to PipeWire's passive socket would itself
start the process. A timer takes the second snapshot eight seconds after the
user manager starts. Both append to the private file:

```text
~/.local/state/ae5-control/routing-boot.log
```

Summarize the paired early/late snapshots and require ten consecutive valid
headphone route-state pairs with:

```sh
bash scripts/collect-routing-state.sh --summary 10
```

The command validates `Output Select`, both `Front` channels, auto-detect,
jack presence, all four output-pin states, PipeWire service timing, the
card-specific headphone port, and the active duplex profile. It exits
nonzero until the trailing run reaches the requested count; an incomplete or
failed pair resets that run. Its parser and failure behavior run in CI through
`--self-test`. The summary proves the persisted state and startup transition;
the user must still confirm audible output before opening a mixer or toggling
the route on each counted boot.

The first historical pair predates collection of the root-cause `Front`
switch. The second requested it, but exposed a subtler readiness race: card
metadata became readable before `Front`, and the failed query was discarded.
The strict summary therefore correctly reports `0/10`. The corrected collector
requires the entire route-control set before recording `alsa_control_ready=yes`
and is installed for the next boot. The earlier post-fix pair still proves its
recorded output-selection, codec-pin, and desktop-port state, but is not
silently promoted to the stronger acceptance gate.

## Safe suspend/resume probe

The same collector supports a user-driven, silent suspend campaign. It does
not call `systemctl suspend`, write a mixer control, or play audio. The host
exposes `deep` sleep, but the normal user cannot program its RTC wake alarm,
so an unattended auto-waking cycle is not safe on this machine. Start each
cycle with a unique campaign ID:

```sh
bash scripts/collect-routing-state.sh --before-suspend campaign-01
```

This command captures the complete route, both mixer fingerprints, every PCM
substream state, PipeWire route and volume, and a fingerprint of relevant
kernel warnings. It rejects the snapshot without appending it unless:

- Master, Front, Surround, Center, LFE, PCM, and the AE-5 PipeWire sink are
  each at or below 20%;
- headphone gain is Low;
- every AE-5 PCM is closed;
- ALSA, codec pins, and PipeWire all select the card-specific headphone path;
- the required fingerprints are readable.

Do not suspend if that command exits nonzero. On success it explicitly reports
that it did not suspend the system; use the desktop's normal suspend action
and wake the machine manually. After WirePlumber has settled, capture the
matching record:

```sh
bash scripts/collect-routing-state.sh --after-resume campaign-01
```

The post-resume command retains a failed snapshot for diagnosis and exits
nonzero when the restored state is unsafe or invalid. Repeat with
`campaign-02` through `campaign-20`, then summarize the private log:

```sh
bash scripts/collect-routing-state.sh --suspend-summary 20
```

A pair passes only when its records are ordered, use the same boot ID and
kernel, satisfy the complete safe headphone-route checks, have exact matching
raw and simple mixer hashes, leave every PCM closed, and add no relevant
kernel warning. A missing, duplicate, or failed pair resets the trailing
consecutive count. Synthetic valid, changed-state, and unsafe-volume cases run
under `--self-test`.

This gate proves silent state restoration. Audible output remains a separate
physical check and must use the non-mutating playback preflight and an
at-or-below-20% fixture and playback state.

Before the next reboot campaign, save the normal profile, deliberately
establish the documented at-or-below-20%/Low-gain test state, and leave that
state persisted across the reboot. After boot, run the non-mutating PipeWire
`playback-preflight` from
[`AUDIO_PARITY_MEASUREMENT.md`](AUDIO_PARITY_MEASUREMENT.md) against the exact
safe fixture. Only after it passes, test sound before opening the GUI or
manually toggling an output. If sound is broken, capture one more snapshot:

```sh
bash scripts/collect-routing-state.sh before-toggle --append
```

Then perform exactly one output toggle, confirm whether audio starts, and run:

```sh
bash scripts/collect-routing-state.sh after-toggle --append
```

Remove the temporary boot probes after the experiment:

```sh
bash scripts/install-routing-probe.sh --uninstall
```

Uninstalling leaves the private log intact.

## How the evidence will be interpreted

| Observation | Most likely next target |
|---|---|
| The current cold boot works | Treat the two upstream 2026 fixes as the resolution; test repeated boots before closing the bug |
| Pre-PipeWire and post-PipeWire ALSA values differ | WirePlumber/ACP policy or state restoration |
| ALSA says `Speakers` before PipeWire, then `Headphone` after it | Expected explicit userspace routing; investigate only if the physical route remains wrong |
| ALSA and pins are correct, but generic Headphones sets Front off | Confirmed here: fix the card-specific ACP path |
| ALSA, pins, and Front are all correct, but a same-value toggle starts audio | CA0132 DSP route initialization/replay; instrument `ca0132_alt_select_out()` |
| Jack presence is wrong before PipeWire | HDA unsolicited/presence detection or board pin configuration |
| Only the PipeWire port is wrong | WirePlumber profile/port persistence, not the kernel driver |

No CA0132 patch is justified for this reproduced failure. Docker and WSL cannot
validate PCI/DSP initialization. A Linux host with VFIO PCI passthrough can
give a Linux or Windows guest direct ownership of the AE-5; this target is
especially suitable because `0000:29:00.0` is the only device in IOMMU group
28. The host must release the card while the guest owns it, and physical
cold-boot checks remain authoritative. The audited setup, safety boundaries,
kernel A/B matrix, and recovery gates are in
[`VFIO_TEST_PLAN.md`](VFIO_TEST_PLAN.md).
