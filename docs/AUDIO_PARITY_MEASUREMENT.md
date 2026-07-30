# Windows/Linux audio parity measurement

Subjective sound differences are not enough evidence for a kernel change. This
procedure measures the first three parity targets with the same source
material:

- output level within 0.5 dB at 1 kHz;
- relative frequency-response delta within 1 dB from 31 Hz to 16 kHz;
- noise-floor delta within 3 dB or the repeatability limit of the setup.

It does not yet measure distortion or prove the analog performance of the
AE-5. Those require a capture interface whose own limits are known.

## Required physical path

The preferred path is:

```text
AE-5 line/headphone output -> safe fixed attenuation -> separate line input
```

Use the same cable, capture interface, input gain, output connector, and volume
on Windows and Linux. Do not connect a powered speaker output to a microphone
input. Do not use the FiFine USB microphone on the current test system as a
line input.

The AE-5 `What U Hear` device can isolate the digital DSP path, and the
motherboard line input can provide an initial analog check. Neither replaces a
calibrated independent interface for final noise or distortion claims.

## Generate one shared reference set

SoX 14.4 or later is required for this development-only harness:

```sh
bash scripts/audio-parity.sh --self-test
bash scripts/audio-parity.sh generate /path/to/ae5-reference
```

The command creates 48 kHz, 24-bit stereo tone, sweep, level-step, and digital
silence files, a six-channel identification file, and `SHA256SUMS`. It refuses
to overwrite any existing fixture. Copy these exact generated files to
Windows and verify their hashes there; do not generate a second set.

The tone, sweep, level-step, and channel-identification fixtures never exceed
`-18 dBFS`, approximately 12.6% of full-scale sample amplitude. The stereo
measurement files begin with a 997 Hz synchronization marker at that ceiling.
The generator independently measures every completed file and refuses any peak
above `-14 dBFS`, which is just below 20% amplitude. The analyzer removes
leading capture latency from the marker before measuring.

Older reference sets may contain a `-6 dBFS` marker from before this safety
gate. Do not play those sets through headphones. Generate a new set in a new
directory with the current script; its `SHA256SUMS` file distinguishes it from
the retired fixtures.

For a Linux-only native-rate comparison, generate separate directories and
pass the same rate to the analyzer:

```sh
AE5_SAMPLE_RATE=44100 bash scripts/audio-parity.sh generate fixtures-44100
AE5_SAMPLE_RATE=96000 bash scripts/audio-parity.sh generate fixtures-96000

AE5_SAMPLE_RATE=96000 bash scripts/audio-parity.sh compare-tones \
  direct-96000.wav pipewire-96000.wav
```

Only 44.1, 48, and 96 kHz are accepted. The 48 kHz reference set remains the
required Windows/Linux comparison unless matching Windows captures are
deliberately collected at another rate.

## Six-channel identity

`parity-channel-id-6ch.wav` sequentially excites Front Left, Front Right,
Front Center, LFE, Rear Left, and Rear Right for one second each, with a
half-second silent gap after every channel. Full-range channels use 997 Hz;
LFE uses 80 Hz so a receiver's crossover does not discard the test. The
generator verifies six channels, nine-second duration, complete channel
isolation, both tone frequencies, peak, and hash.

Use this known file instead of `speaker-test`, whose generated signal cannot
pass the fixture peak scanner. Run `playback-preflight` against this exact WAV
immediately before opening its direct-ALSA or PipeWire stream.

## Capture matrix

Start with all output DSP processing disabled or flat, Low headphone gain, a
fixed capture gain, and every hardware and software playback-volume control at
or below 20%. Keep headphones unworn or physically clear during unattended
playback. Record at 48 kHz, 24-bit stereo for an electrical capture. A
single-channel microphone capture is accepted for preliminary acoustic
screening when the same microphone channel count is used on both operating
systems:

1. Windows playback through the Creative driver.
2. Linux direct ALSA `hw:` playback.
3. Linux normal PipeWire playback.
4. Digital silence through the same three paths.

Do not assume that the same displayed Windows and Linux percentage represents
the same attenuation. Binary tracing establishes that Sound Blaster Command
sends its percentage divided by 100 directly to Windows
`IAudioEndpointVolume::SetMasterVolumeLevelScalar`. Windows applies a
nonlinear audio-tapered endpoint curve, while the Linux PipeWire control uses
its own volume semantics. Match the measured endpoint decibel value for level
parity, then keep that value fixed for the response comparison. See
[`WINDOWS_STACK_ARCHITECTURE.md`](WINDOWS_STACK_ARCHITECTURE.md#exact-master-volume-trace).

Immediately before a playback command, run the non-mutating preflight for the
chosen path and exact fixture:

```sh
AE5CTL=target/release/ae5ctl \
  bash scripts/audio-parity.sh playback-preflight direct \
  /path/to/fixtures-48000/parity-tones.wav

AE5CTL=target/release/ae5ctl \
  bash scripts/audio-parity.sh playback-preflight pipewire \
  /path/to/fixtures-48000/parity-tones.wav
```

The direct check requires the fixture and every channel of the AE-5 Master,
Front, Surround, Center, LFE, and PCM playback stages to be at or below 20% of
its advertised raw range, and headphone gain to be Low. The PipeWire check
instead accepts the installed soft-mixer model's exact fixed stages—Master
99/99, Front 90/99, and PCM 255/255, all 0 dB—or the earlier at-or-below-20%
attenuated state. It also requires a healthy card-specific route, the AE-5 as
the default sink, and its user-facing software volume at or below 20%. It only
reads state and never lowers a value automatically; a missing or unparseable
safety control also fails closed. Any failure forbids playback until the
operator deliberately establishes and later restores a safe snapshot.

Do not change gain between captures. Record at least half a second before
starting playback and at least half a second after it ends. Preserve the
original WAV files and record the operating system, driver/kernel, selected
output, volume, gain, DSP state, DAC filter, playback API, capture device, and
cable in a text notebook beside them.

The exact playback and capture commands depend on the selected external
interface and are deliberately not automated yet. Opening the wrong ALSA
device or using an unsafe analog connection should remain an explicit human
decision.

## Prepared Windows dual-boot handoff

The current test machine has a ready-to-use bundle at
`%USERPROFILE%\Documents\AE5-parity-capture`. It contains:

- the exact hash-verified `fixtures-48000` reference set;
- the official portable Audacity 3.7.7 64-bit archive and its published
  SHA-256;
- [`README-FIRST.md`](windows-capture/README-FIRST.md), a Windows capture
  checklist;
- [`VERIFY-SHA256.ps1`](windows-capture/VERIFY-SHA256.ps1), which verifies all
  five fixtures and Audacity before use;
- a `captures` directory containing the settings-notes template.

Run the verifier from PowerShell before extraction:

```powershell
cd "$HOME\Documents\AE5-parity-capture"
powershell -NoProfile -ExecutionPolicy Bypass -File .\VERIFY-SHA256.ps1
```

The FiFine microphone and headphones placed in a fixed jig can provide a
useful neutral-versus-tuning acoustic A/B screen. This path is mono when that
is all the microphone exposes, and it is sensitive to placement and room
noise. It must not be reported as final analog parity. Do not play the
six-channel identity fixture through this headphone/microphone setup.

For the Windows screen, keep the headphones unworn, select Low gain, and keep
Windows master volume, the player session volume, and every Creative playback
volume at or below 20%. Use the same fixed positions and capture gain for:

1. `windows-neutral-tones.wav`, with all processing disabled;
2. `windows-neutral-silence.wav`, while playing the exact silence fixture;
3. `windows-tuning-tones.wav`, with only the named headphone tuning enabled.

Start recording at least half a second before playback and stop at least half
a second afterward. Export 48 kHz, 24-bit PCM WAV without normalization,
resampling, trimming, fades, denoising, or effects. Record every setting in
`captures\WINDOWS-NOTES.txt`.

The isolated Windows comparison VM has Command `3.5.10.0`, the imported AE-5
setting tree, and no emulated audio device. Its local-account credential was
recovered from the private setup record without exposing it, login succeeded,
and temporary automatic-login values were removed immediately. Managed
system-libvirt passthrough then completed the guarded `What U Hear` matrix.
The installed state, recovery hashes, and passthrough boundary are recorded
in [`VFIO_TEST_PLAN.md`](VFIO_TEST_PLAN.md).

For the VM's digital OutFX and equalizer method, follow
[`windows-capture/VM-OUTFX-A-B.md`](windows-capture/VM-OUTFX-A-B.md); the
completed evidence is in
[`windows-capture/VM-OUTFX-RESULTS.md`](windows-capture/VM-OUTFX-RESULTS.md).
The counterbalanced neutral/full captures prove that `What U Hear` contains
the imported Acoustic Engine result. The same endpoint did not contain the
displayed ten-band EQ curve, so it cannot support a graphic-EQ parity claim.

## Analyze and compare

Inspect one tone capture:

```sh
bash scripts/audio-parity.sh analyze-tones windows-tones.wav
```

Compare Windows and Linux:

```sh
bash scripts/audio-parity.sh compare-tones \
  windows-tones.wav linux-alsa-tones.wav
```

The comparison reports:

- the absolute Linux-minus-Windows level delta at every band;
- each band's response delta after normalizing both captures to 1 kHz;
- `pass` only when the 1 kHz level and maximum response delta meet the targets.

For Linux software-EQ acceptance, first save the profile and emit the exact
response of the generated PipeWire graph:

```sh
ae5ctl eq-chain-enable ~/.config/ae5-control/profiles/windows-headphones.json
ae5ctl eq-chain-response 48000 > expected-eq.tsv
bash scripts/audio-parity.sh compare-eq \
  expected-eq.tsv linux-wuh-neutral-a.wav linux-wuh-eq-only-a.wav
```

This comparison checks absolute equalized-minus-neutral level at every fixture
frequency. Current direct-filter-v2 predictions contain no automatic preamp,
and the comparison passes only when the maximum error is at most 1 dB.

Compare captures of `parity-silence.wav`:

```sh
bash scripts/audio-parity.sh compare-noise \
  windows-silence.wav linux-alsa-silence.wav
```

If the synchronization marker is recorded below -40 dBFS or the noise floor
triggers alignment too early, set a suitable SoX amplitude threshold:

```sh
AE5_SYNC_THRESHOLD=0.3% \
  bash scripts/audio-parity.sh analyze-tones quiet-capture.wav
```

Do not normalize, denoise, resample, or encode the captures before comparison.
The analyzer accepts mono or stereo captures and rejects a comparison when
their channel counts differ. Mono is only for the preliminary acoustic screen;
use stereo for the final electrical measurement.
If direct ALSA matches Windows but PipeWire does not, investigate PipeWire
format/rate policy before touching the kernel. If both Linux paths diverge in
the same way, compare DSP state and CA0132 initialization next.

## Guarded Windows/Linux acoustic screen

On 2026-07-26, the Windows comparison guest owned the physical AE-5 through
VFIO and loaded the imported headphone profile in Sound Blaster Command
3.5.10.0. The Windows endpoint and player session were both verified at 20%,
and Command reported Low headphone gain. The same -18 dBFS 997 Hz fixture,
headphone/microphone placement, and Fifine capture path were then reused after
returning the card to the Linux host.

The Windows capture measured -54.16 dBFS at 997 Hz. The repaired normal
PipeWire path measured -70.65 dBFS with the desktop sink at the separately
approved 40% ceiling, player stream at unity, raw Master and Front at 19/99,
PCM at 51/255, and Low gain. The Linux tone is present, but the 16.50 dB
difference is not a level-matched operating-system comparison.

`Master` is ALSA's virtual master over the `Front` follower. The kernel
computes the effective follower as `Front + Master - Master max`, clamped to
the follower range. At Master 19 and Front 19 that is `19 + 19 - 99`, which
clamps to 0/99. Master remains at that floor through 80/99 while Front stays
at 19/99. PCM 51/255 and PipeWire's software attenuation add two more
independent reductions.

A bounded four-capture A/B at the approved 40% PipeWire ceiling confirmed the
source-derived behavior. Two Master 19 captures measured -65.22 and
-65.19 dBFS; two Master 20 captures measured -65.17 and -65.19 dBFS. The
Master 20 minus Master 19 deltas were +0.05 and -0.01 dB, within the repeat
spread. This is expected virtual-master clamping, not a stuck 43% hardware
volume or evidence that Linux playback is intrinsically 16.50 dB quieter.

The final 20% safety-ceiling check repeated the same two-second, -18 dBFS,
997 Hz fixture with the headphones unworn beside the Fifine. PipeWire was
exactly 20% and initially muted, Master and Front were 19/99 and on, PCM was
51/255, and headphone gain was Low. The quiet capture measured -116.11 dBFS
in the 987-1007 Hz band; the playback capture measured -117.02 dBFS. Playback
completed, but there was no acoustic tone rise. A no-output follow-up muted
the hardware Master and recorded the same PipeWire stream through the card's
internal What U Hear tap. Its 987-1007 Hz band measured -97.36 dBFS, while an
otherwise identical idle capture was bit-exact zero. This proves that
PipeWire, ALSA transport, and the DSP received samples and isolates the
failure to final analog gain staging. The sink was immediately remuted, every
PCM closed, the complete mixer returned to SHA-256
`da1bb179b43584844826b8950653fd2fd9b6a78994c47a039ffd782db06497bc`,
and no relevant kernel warning appeared. This is a gain-staging failure in
the legacy attenuated state at the 20% operating point, not a route or
transport regression: the virtual Master keeps Front at its effective floor
while PCM and PipeWire add further attenuation.

After explicit approval to separate internal calibration from the 20% user
volume ceiling, Master was set to 99/99 (0 dB), Front to 90/99 (0 dB), and PCM
to 255/255 (0 dB). Headphone gain remained Low, the headphones remained
unworn, and PipeWire was ramped through 1, 2, 4, 8, 12, 16, and 20% with the
hardware Master muted between every exposure. A final matched-window FFT
screen at 20% measured mean 987-1007 Hz power `0.000652222222222` in the quiet
capture and `0.0177011666667` during the fixture, a +14.34 dB detection.

The installed card-specific ACP path now holds those three internal stages at
0 dB and keeps Master and Front on while PipeWire's software mixer owns volume
and mute. A negative persistence check deliberately restored the old
19/19/51 state and muted Master; restarting WirePlumber recovered
99/90/255, both hardware switches on, the matched Headphone/Microphone route,
and the still-muted 20% sink. A separate Speakers-to-Headphone activation
recovered the same state from the same injected attenuation. One later
acoustic repeat was rejected because its quiet microphone capture had a
substantially elevated 997 Hz baseline; it is not used as acceptance evidence.

This comparison found and fixed the Linux silent-transport path: use raw
`hw:%f` rather than HDA's `front:` softvol, ALSA read/write rather than mmap,
6016-frame periods with four periods, S32 for low-volume precision, and
`api.alsa.ignore-dB=true`. The original format conclusion was confounded by
testing S32 only with the broken default period geometry. Exact A/B evidence is in
[`DRIVER_ROUTING_INVESTIGATION.md`](DRIVER_ROUTING_INVESTIGATION.md).

The result is a transport and audibility pass, not a parity pass. It uses a
microphone rather than an attenuated electrical capture, compares only one
frequency, and deliberately uses different operating-system gain structures.
Full response, noise, and matched electrical-level measurements remain open.

## Target-card Linux digital baseline

On 2026-07-24, the generated tone fixture was played through both direct ALSA
and the normal PipeWire AE-5 sink while the physical card's `CA0132 What U
Hear` PCM recorded 48 kHz, 32-bit stereo.

With the saved effects profile active, PipeWire differed from direct ALSA by
`-0.01 dB` at 1 kHz and at most `0.20 dB` in relative response. With only
`Enable OutFX` disabled, every measured band was flat relative to 1 kHz and
the two Linux paths matched by `0.00 dB` in both reported metrics.

The active effects profile was itself measurable: enabling it changed the
1 kHz level by `7.70 dB` and relative response by as much as `9.05 dB`.
Separate 997 Hz-left/1503 Hz-right probes showed more than 42 dB separation
from the opposite band for both playback APIs. Every temporary control change
restored the exact complete mixer snapshot, and no audio-driver warning or
error appeared in the kernel journal.

This baseline isolates PipeWire from the current sound difference at 48 kHz:
the desktop path is equivalent to direct ALSA when measured through the card's
digital loopback. It does not establish Windows parity or analog output
performance. Exact target-system values and limits are recorded in
[`HARDWARE_BASELINE.md`](HARDWARE_BASELINE.md).

The same loopback path was subsequently used to isolate Surround, Crystalizer,
Dialog Plus, Smart Volume, X-Bass, Equalizer Flat, and all ten individual EQ
bands. Results, repeat deltas, and the two edge-band gain shortfalls are in
[`DSP_EFFECT_MEASUREMENT.md`](DSP_EFFECT_MEASUREMENT.md).

Separate neutral 44.1 and 96 kHz captures later matched direct ALSA and
PipeWire by `0.00 dB` in level and maximum response delta when the PCM mixer
was at 0 dB. Digital-silence captures were byte-identical. The guarded method,
mixer-gain diagnosis, and limitations are in
[`PIPEWIRE_RATE_PARITY.md`](PIPEWIRE_RATE_PARITY.md).
