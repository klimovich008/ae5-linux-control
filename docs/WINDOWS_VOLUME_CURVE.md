# Windows-equivalent AE-5 volume curve

Status on 2026-07-29: **the Linux mapping and guarded setter are implemented;
the exact physical Windows AE-5 capture is pending**.

The collector was also executed in the existing cardless Windows 11 guest.
Windows PowerShell parsed it, `Add-Type` compiled the Core Audio COM
definitions, and execution reached the expected
`GetDefaultAudioEndpoint(eRender, eMultimedia)` failure because that guest had
no playback endpoint. The VM had no PCI host device attached and was shut
down after the check. This validates script loading, not the physical AE-5
curve.

## Why equal percentages do not match

Sound Blaster Command forwards its displayed master volume divided by 100 to
Windows `IAudioEndpointVolume::SetMasterVolumeLevelScalar`. Windows converts
that scalar through its endpoint-specific audio-tapered curve. PipeWire's
normal user volume uses a cubic curve: a displayed fraction `p` produces
approximately `p³` sample amplitude.

AE5 Control therefore does not copy the displayed percentage. It:

1. records the installed Windows AE-5 endpoint's scalar-to-decibel result;
2. interpolates the captured points in decibels; and
3. solves PipeWire's cubic curve for the control value that produces the same
   attenuation.

For a captured Windows attenuation `dB`, the existing PipeWire sink receives:

```text
PipeWire percent = 100 × 10^(dB / 60)
```

No extra filter, sink, master gain, or virtual device is added.

## Safety boundary

[`measure-ae5-volume-curve.ps1`](../scripts/windows/measure-ae5-volume-curve.ps1)
uses the default Windows multimedia render endpoint and refuses to continue
unless Plug and Play identifies it as an AE-5. It:

- opens no playback or capture stream;
- writes no Creative feature, profile, gain, or OutFX property;
- records all 101 integer endpoint-volume positions while the endpoint is
  muted;
- records `GetVolumeRange`, `GetVolumeStepInfo`,
  `QueryHardwareSupport`, and the endpoint channel count; and
- restores and verifies the original scalar and mute state before saving
  JSON.

The output label is explicit because the Windows headphone and line-out paths
must not be assumed equivalent.

`QueryHardwareSupport` is also reported. If Windows says endpoint volume is
hardware-backed, matching its dB attenuation in PipeWire can match level but
does not prove equal analog noise or dynamic range; the later electrical
capture remains mandatory.

On Linux, `volume-curve-apply` fails before changing volume unless:

- the curve is a verified, restored AE-5 capture;
- the selected Linux output matches the captured output;
- hardware OutFX is readable and off;
- Master is on at 99/99, Front is on at 90/99, and PCM is at 255/255; and
- the exact AE-5 PipeWire profile has `api.alsa.soft-mixer=true` and
  `api.alsa.ignore-dB=true`.

The setter changes only the existing AE-5 sink volume. It reads the exact
software volume and mute first, verifies both after the write, and restores
the previous values if verification fails.

## Capture procedure

Do not use the passthrough VM from `7.1.4-ae5-stable` for this measurement.
The accepted shutdown-reset candidate is not the current running kernel, and
the newly reported OutFX failure can survive an OS handoff. Use a full power
off before entering Windows.

1. Keep AE-5 analog outputs disconnected for the unattended capture. Close
   all audio applications.
2. Boot the installed Windows system after the machine has lost motherboard
   power.
3. Select the AE-5 as the default Windows multimedia playback device.
4. Select the intended Creative output, starting with **Headphones**. Do not
   open or change OutFX during this run.
5. Open an ordinary, non-administrator PowerShell in
   `Documents\AE5-parity-capture` and run:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\measure-ae5-volume-curve.ps1 -Output Headphone
```

The result is written below `captures`. Preserve that JSON and perform a full
Windows shutdown and motherboard power removal before returning to Linux.

## Linux validation and use

After returning to Linux:

```sh
ae5ctl volume-curve-check /path/to/ae5-windows-volume-headphone-*.json
ae5ctl volume-curve-map /path/to/capture.json 5
ae5ctl volume-curve-map /path/to/capture.json 30
ae5ctl volume-curve-apply /path/to/capture.json 5
```

`volume-curve-apply` preserves the current mute state. The first physical
comparison remains capped at 20% on the Windows scale, uses Low headphone
gain, and starts muted with the headphones unworn. A successful mapping is
not yet a loudness-parity acceptance result; that still requires the existing
matched electrical or guarded acoustic A/B procedure.

## OutFX incident boundary

The user's latest observation is consistent with the existing failure model:
OutFX initially processed audio, a later setting transition left both Linux
and Windows in a bad audio state, and only removal of motherboard power
cleared it. That is not evidence that profile JSON or a normal software
volume value was corrupted. It is evidence that an unsafe DSP/card state can
survive driver and operating-system transitions.

Consequently, hardware `Enable OutFX` and its child ALSA writes remain
rejected. Software OutFX implementation resumes only after this volume curve
is captured and accepted.
