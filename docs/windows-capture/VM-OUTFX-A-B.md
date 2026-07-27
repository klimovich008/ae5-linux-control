# Windows VM OutFX and equalizer A/B

Use this runbook only for the prepared `ae5-windows-compare-system` guest with
the physical AE-5 passed through. Its first purpose is to establish whether
Windows `What U Hear` contains the Creative render APO output. Only after that
gate passes may the same captures be compared with Linux's internal tap.

This is a digital-processing comparison. It does not measure the DAC,
headphone amplifier, analog noise, or final analog frequency response.

## Required state before starting

- The Linux stable-kernel bare-metal runtime and playback gates have passed.
- Every AE-5 analog output is physically unplugged.
- Both Linux system guests and the Windows guest are powered off.
- The Windows guest has a fresh powered-off recovery point.
- The exact `fixtures-48000` bundle passes `VERIFY-SHA256.ps1`.
- A user can log in interactively. Guest Agent reporting zero logged-in users
  is a hard stop because Command's render properties belong to that session.
- Sound Blaster Command 3.5.10.0 and the imported settings are already
  installed. Do not copy settings again during the measurement cycle.

The Windows render endpoint and player session must stay at or below 20%.
Start at 5% and muted. Command has previously unmuted the endpoint during an
output transition, so reapply and independently verify the cap plus mute after
every route, profile, format, device, or Command-master change.

## Capture states

Use the Headphones route, Low gain, stereo, 48 kHz, one unchanged DAC filter,
Windows enhancements off, Spatial Sound off, Scout Mode off, and Direct Mode
off for every state.

The three states intentionally separate Equalizer from OutFX. Static analysis
shows that Command's OutFX/Acoustic Engine master groups Surround, Dialog+,
Smart Volume, Crystalizer, and Bass management; Equalizer is not one of those
five children.

| State | Acoustic Engine / OutFX | Equalizer | Other five child effects |
|---|---|---|---|
| `neutral` | Off | Off | Retained but inactive |
| `eq-only` | Off | Imported curve on | Retained but inactive |
| `full-profile` | On | Imported state | Exact imported state |

Do not call `eq-only` “OutFX on.” Do not compare `full-profile` with Linux as
feature parity until the corresponding Linux substitutes exist and have their
own acceptance evidence.

## Prove the capture boundary first

1. Log in interactively and launch Sound Blaster Command.
2. Confirm that Command recognizes the AE-5 and shows the imported headphone
   profile. Save a screenshot of the profile, Playback, Equalizer, and Sound
   Effects pages in the private Windows reference directory.
3. In Audacity, select the Creative `What U Hear` endpoint as the recording
   device. Use stereo, 48 kHz, no monitoring, and no recording effects.
4. Verify the endpoint and player are still muted and at or below 20%.
5. Select `neutral`, reapply the cap and mute, then unmute only for the
   intended playback. Record `parity-tones.wav` twice:

   ```text
   windows-wuh-neutral-a.wav
   windows-wuh-neutral-b.wav
   ```

6. Select `full-profile` with at least one of the five OutFX children visibly
   enabled at its imported nonzero value. Reapply the cap and mute. Record
   twice:

   ```text
   windows-wuh-full-a.wav
   windows-wuh-full-b.wav
   ```

7. Return to `neutral`, reapply the cap and mute, and take one counterbalanced
   repeat:

   ```text
   windows-wuh-neutral-c.wav
   ```

Export every recording as untrimmed 48 kHz PCM WAV without normalization,
resampling, fades, denoising, compression, or effects.

Analyze the files on Linux:

```sh
bash scripts/audio-parity.sh compare-tones \
  windows-wuh-neutral-a.wav windows-wuh-neutral-b.wav
bash scripts/audio-parity.sh compare-tones \
  windows-wuh-neutral-a.wav windows-wuh-full-a.wav
bash scripts/audio-parity.sh compare-tones \
  windows-wuh-neutral-c.wav windows-wuh-full-b.wav
```

The two neutral captures must first be repeatable. Both counterbalanced
neutral/full comparisons must then show the same profile-shaped change above
that repeat spread. If they do not, Windows `What U Hear` is not a validated
post-APO instrument and no internal Windows/Linux OutFX conclusion may use it.
Fall back to the fixed headphone/FiFine acoustic screen or a safely attenuated
electrical capture.

## Same-settings equalizer matrix

Run this only if the capture-boundary gate passes.

1. Record two `neutral` tone captures and one digital-silence capture.
2. Select `eq-only` with the exact imported ten-band curve. Reapply and verify
   endpoint plus player limits, then record two tone captures and one silence
   capture.
3. Select `full-profile` and record two tone captures for Windows-only
   characterization. Keep this separate from the equalizer parity result.
4. Complete every field in `VM-OUTFX-NOTES.txt`, including the exact visible
   values and screenshot filenames.

Use:

```text
windows-wuh-neutral-{a,b}.wav
windows-wuh-neutral-silence.wav
windows-wuh-eq-only-{a,b}.wav
windows-wuh-eq-only-silence.wav
windows-wuh-full-{a,b}.wav
```

On Linux, use the same fixture, rate, physical target, route, DAC filter, and
profile curve. Capture the physical sink with software EQ disabled, then with
only the guarded PipeWire EQ enabled:

```text
linux-wuh-neutral-{a,b}.wav
linux-wuh-eq-only-{a,b}.wav
```

Compare repeatability first, then operating systems:

```sh
bash scripts/audio-parity.sh compare-tones \
  windows-wuh-neutral-a.wav windows-wuh-neutral-b.wav
bash scripts/audio-parity.sh compare-tones \
  linux-wuh-neutral-a.wav linux-wuh-neutral-b.wav
bash scripts/audio-parity.sh compare-tones \
  windows-wuh-neutral-a.wav linux-wuh-neutral-a.wav
bash scripts/audio-parity.sh compare-tones \
  windows-wuh-eq-only-a.wav linux-wuh-eq-only-a.wav
```

Report both absolute level delta and response delta normalized to 1 kHz.
Equalizer acceptance concerns the relative response curve; a level mismatch
must still be reported and investigated separately.

## Shutdown and recovery

1. Mute the Windows render endpoint and stop playback.
2. Close Audacity and Sound Blaster Command.
3. Shut Windows down cleanly through Guest Agent or the Start menu.
4. Remove the temporary managed host device from the powered-off domain.
5. Verify the AE-5 returned to host `snd_hda_intel`.
6. Restore only the Linux audio services that were active before the cycle.
7. Reapply the exact AE-5 sink at 5% and muted, Master and Front off, and Low
   gain.
8. Require both playback PCMs closed and compare complete host mixer and route
   snapshots with their pre-cycle files.

Any login failure, auto-unmute that cannot be independently corrected,
capture-boundary failure, clipped capture, state mismatch, driver warning, or
incomplete host recovery invalidates the comparison.
