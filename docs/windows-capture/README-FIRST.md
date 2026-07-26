# AE-5 Windows capture handoff

This bundle collects the Windows reference needed to compare the AE-5
headphone path with Linux. It does not install a driver or modify Sound Blaster
Command.

## Safety gate

Do not press Play until every item below is true:

- Headphones are unworn and fixed beside the FiFine microphone.
- Sound Blaster Command reports **Low** headphone gain.
- Windows master volume is **20% or lower**.
- VLC/player session volume is **20% or lower**.
- Every Creative playback-volume control that is exposed is **20% or lower**.
- Speakers are disconnected or confirmed not to be the active output.
- The file being played is from `fixtures-48000`.

Sound Blaster Command 3.5.10.0 was observed unmuting the Windows render
endpoint when switching from Speakers to Headphones. After every output,
profile, or device transition, reapply the 20% cap and mute, then verify both
outside Command before continuing. Unmute only after the complete safety gate
passes and immediately before the intended capture.

The supplied tones peak at `-18 dBFS`. Never use the six-channel
`parity-channel-id-6ch.wav` file for this headphone/microphone procedure.
Stop immediately if the route, gain, or volume is uncertain.

## 1. Verify the bundle

Open PowerShell in this directory and run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\VERIFY-SHA256.ps1
```

Continue only when it prints `All 6 handoff files verified`.

## 2. Start portable Audacity

Use Explorer's **Extract All** on
`tools\audacity-win-3.7.7-64bit.zip`. Run `Audacity.exe` from the extracted
folder. No installation is required. The archive is from the
[official Audacity 3.7.7 release](https://github.com/audacity/audacity/releases/tag/Audacity-3.7.7);
the verifier checks its published SHA-256.

In Audacity:

1. Select the FiFine microphone as the recording device.
2. Select one recording channel if the microphone exposes mono; otherwise use
   stereo. Use the same channel count later on Linux.
3. Set the project/sample rate to 48000 Hz.
4. Disable software input monitoring to avoid feedback.
5. Do not enable recording effects, normalization, or automatic level control.

Fix the microphone, headphones, and ear cups in position. Mark their positions
so they do not move between captures. Keep the room and microphone input gain
unchanged.

## 3. Neutral captures

In Windows Sound settings, disable audio enhancements and Spatial Sound. In
Sound Blaster Command:

- select the AE-5 headphone output;
- select Low gain;
- disable SBX processing, Equalizer, Scout Mode, Direct Mode, and any named
  headphone tuning;
- record the output mode, DAC filter, driver version, and Command version in
  `captures\WINDOWS-NOTES.txt`.

For each capture, begin recording at least 0.5 seconds before VLC playback and
stop at least 0.5 seconds afterward.

1. Play `fixtures-48000\parity-tones.wav` once and export
   `captures\windows-neutral-tones.wav`.
2. Play `fixtures-48000\parity-silence.wav` once and export
   `captures\windows-neutral-silence.wav`.

Export each file as 24-bit PCM WAV at 48000 Hz. Do not trim, normalize,
resample, fade, denoise, compress, or otherwise process the recording.

## 4. Named-tuning capture

Without moving anything or changing any volume, gain, filter, microphone, or
format setting:

1. Enable only the exact named headphone tuning that should be ported.
2. Record its exact visible name in `captures\WINDOWS-NOTES.txt`.
3. Play `fixtures-48000\parity-tones.wav` once.
4. Export `captures\windows-tuning-tones.wav` as 24-bit PCM WAV.

Disable the tuning after the capture. Leave the three original WAV files and
the completed notes in `captures`; Linux will analyze those originals without
editing them.

## Limits

This microphone method is a preliminary acoustic A/B screen. It can show a
repeatable response change from the named tuning, but microphone response,
headphone placement, and room noise prevent a final analog-parity claim. Final
validation still requires safely attenuated line-level capture through the
same independent interface on Windows and Linux.
