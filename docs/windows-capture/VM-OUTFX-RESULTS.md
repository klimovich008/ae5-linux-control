# Windows VM `What U Hear` result

This records the 2026-07-27 guarded run of
`ae5-windows-compare-system` with the physical AE-5 passed through. It answers
two narrower questions:

1. does Windows `What U Hear` contain Sound Blaster Command's Acoustic Engine
   (OutFX/SBX) result; and
2. can that same endpoint validate the visible ten-band graphic equalizer?

It is a digital-loopback result. It does not measure the DAC, headphone
amplifier, analog noise, or final analog response.

## Fixed state and safety boundary

- Sound Blaster Command: 3.5.10.0
- Creative driver: 6.0.105.65
- playback/recording application: Audacity 3.7.7, MME
- output: Speakers endpoint with Command's Headphones route
- recording endpoint: `What U Hear (Sound BlasterX AE-5)`, stereo
- Command format: 32-bit, 48 kHz
- exported captures: 48 kHz, stereo, signed 16-bit PCM
- headphone gain: Low, 16–31 ohm
- headphone tuning: `audio-technica ATH-M50`
- DAC filter: Fast Roll Off
- Direct Mode, Scout Mode, Spatial Sound: off
- render endpoint and Audacity session: 5%
- both AE-5 render endpoints: muted outside each intentional capture
- every AE-5 analog output: physically unplugged

Neutral used Acoustic Engine master off and Equalizer off. Full-profile used
the imported Acoustic Engine state: Surround 0, Crystalizer 50, Bass 53,
Smart Volume 15, and Dialog+ 0. The fixture and project rate were explicitly
set to 48 kHz before the accepted captures.

## Acoustic Engine / OutFX boundary: passed

Three counterbalanced neutral captures were identical at the analyzer's
two-decimal resolution: every 31 Hz–16 kHz band measured `-47.13 dBFS`, and
both neutral comparisons had `0.00 dB` maximum response delta.

Enabling the imported Acoustic Engine profile produced a large, repeatable
post-processing shape:

| Frequency | Full A relative to 1 kHz | Full B relative to 1 kHz |
|---:|---:|---:|
| 31 Hz | +10.59 dB | +14.32 dB |
| 62 Hz | +19.54 dB | +20.06 dB |
| 125 Hz | +13.36 dB | +13.65 dB |
| 250 Hz | +2.05 dB | +2.04 dB |
| 500 Hz | +0.22 dB | +0.19 dB |
| 1 kHz | 0.00 dB | 0.00 dB |
| 2 kHz | +0.61 dB | +0.62 dB |
| 4 kHz | +1.38 dB | +1.37 dB |
| 8 kHz | +1.77 dB | +1.77 dB |
| 16 kHz | +2.91 dB | +2.92 dB |

The two full-profile captures agreed within `0.03 dB` from 250 Hz through
16 kHz. Their low-frequency difference reached `3.73 dB` at 31 Hz, consistent
with stateful Bass or Smart Volume behavior; it is retained as an
investigation result rather than averaged away.

The neutral → full → neutral counterbalance proves that Windows `What U Hear`
is downstream of the Acoustic Engine processing used in this profile.
Together with the binary/property analysis recorded elsewhere in the
repository, this supports the current model: Command writes software APO
properties consumed by `CtxRFX64.dll`; it does not enable Linux's unsafe raw
`Enable OutFX` hardware switch.

## Graphic equalizer boundary: not established

The imported curve displayed in Command was:

| 31 | 62 | 125 | 250 | 500 | 1k | 2k | 4k | 8k | 16k |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| +9 | +6 | +8 | +4 | +1 | -2 | -2 | 0 | +6 | +6 dB |

Turning Equalizer on initially produced a file byte-identical to the neutral
capture. A forced `Rock` → `SHP Last` preset transition while muted produced
two repeatable diagnostic captures, but their measured response still did not
contain the displayed curve. The two diagnostics repeated within `0.86 dB`;
comparison with the Linux model of the imported curve, including its
automatic preamp, missed by as much as `13.08 dB`.

Therefore Windows `What U Hear` must not be used to claim graphic-EQ parity.
The evidence is consistent with either a pre-graphic-EQ tap or Command not
committing the displayed curve to the active endpoint. A later comparison
needs an independently verified post-EQ endpoint, a safely attenuated
electrical capture, or a fixed acoustic jig.

This negative result does not weaken Linux's independent software-EQ
acceptance: the physical AE-5 loopback measured the requested Linux graph
within `0.34 dB`.

## Accepted private evidence

The captures are intentionally not committed. On the reference machine they
are under:

```text
~/.cache/ae5-control/windows-compare-host-20260727-XGUXdE/
```

The accepted WAV hashes are:

| Capture | SHA-256 |
|---|---|
| `windows-wuh-neutral-a.wav` | `569a34240ec01357dfb335a232f378a55e02b0315cd3f2e8884963936cf69150` |
| `windows-wuh-neutral-b.wav` | `302bd8681b73ce516160c2b1cc4a44f6bd46148be0b650e90450edc3ecf4e841` |
| `windows-wuh-neutral-c.wav` | `f90c985d2bee63177de00844f3ce0639fcdd7fc2a36dacaf8d11e56efa6bc228` |
| `windows-wuh-full-a.wav` | `df2513ec502e6636829d75a07ca7c0573c389b46c86bffbf694dbe36229acad3` |
| `windows-wuh-full-b.wav` | `9bc06cf98f281426cf459ae0fd990dc81808ba1b436c3ef4999da359bbeba294` |
| `windows-wuh-eq-only-a-committed.wav` | `524e8b69648bd42d1b3ba9966b1a4a50ea1141c3249e49902fe2f0f24056012e` |
| `windows-wuh-eq-only-b-committed.wav` | `c9609ccb11a9912e563d15d3c7ff7b1caa895207190e585cba631dab5f30ff96` |

The same directory contains per-capture analyses, the repeatability and EQ
boundary tables, settings screenshots with a SHA-256 manifest, and host
pre/post recovery snapshots. Final cleanup left both Windows render endpoints
at 5% and muted, removed temporary automatic-login values, shut the guest down
cleanly, rebound the AE-5 to host `snd_hda_intel`, restored the Linux sink to
5% muted, kept OutFX off, and left both playback PCMs closed.
