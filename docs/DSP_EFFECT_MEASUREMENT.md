# CA0132 output-effect measurement

Collected on 2026-07-24 from the target Sound BlasterX AE-5 without changing
the loaded kernel driver.

## Method

The project parity-tone fixture, SHA-256
`558745db6e46597641cad3b0a0da50e76f8895944b7453f7726ce8abf560ebdc`,
was played through direct ALSA at 48 kHz, 32-bit stereo and recorded through
the physical `CA0132 What U Hear` PCM at the same format.

The card remained on Headphone, low gain, and 2.0 channels. `Master` was set
to 0 so the physical output was effectively silent while the digital loopback
remained measurable. `Enable OutFX` stayed on. The neutral scenario disabled
Surround, Crystalizer, Dialog Plus, Smart Volume, X-Bass, and Equalizer. Each
effect scenario enabled only the named effect at the saved control value:

- Surround 67;
- Crystalizer 65;
- Dialog Plus 50;
- Smart Volume 74 in Normal mode;
- X-Bass 50 with crossover value 8;
- Equalizer Flat with all bands centered at 24.

Before each scenario, the same validated 47-control native profile was
applied. An exit guard applied it again after every capture. Its SHA-256 was
`c6f77352556c5a31f044f2e5993f7ca728946fb6ee7c499f472bc9286d11c333`.
The complete live mixer output had SHA-256
`02530d87f6ce78e00f213bfa25f53174e8bfea1778f94b83bc0c9d32278c89f6`
before and after every completed scenario.

## Isolated effects

The level delta is measured at 1 kHz. Maximum response delta is normalized to
1 kHz across the ten fixture frequencies. Repeat delta compares two separate
captures of the same isolated scenario.

| Scenario | 1 kHz level delta | Maximum response delta | Maximum repeat delta |
|---|---:|---:|---:|
| Neutral repeat | +0.00 dB | 0.00 dB | 0.00 dB |
| Surround | -1.10 dB | 3.71 dB | 0.00 dB |
| Crystalizer | +1.44 dB | 4.17 dB | 0.00 dB |
| Dialog Plus | -2.97 dB | 6.77 dB | 0.01 dB |
| Smart Volume | +7.41 dB | 4.82 dB | 0.42 dB |
| X-Bass | +0.01 dB | 7.89 dB | 0.00 dB |
| Equalizer Flat | +0.00 dB | 0.00 dB | 0.00 dB versus neutral |

Smart Volume is a stateful leveler, so its 0.42 dB repeat variation over the
ordered tone sequence is expected and remains inside the project's 0.5 dB
level target. The other static effects repeated within 0.01 dB.

Dialog Plus produced a repeatable low-band-heavy response with this pure-tone
fixture. That proves that the ALSA switch reaches active DSP processing; it
does not establish how the effect behaves on speech or whether its response
matches Sound Blaster Command.

## Ten-band equalizer

The importer maps a requested gain in dB to ALSA value `24 + gain`, so setting
one band from 24 to 36 requests +12 dB. Every other band remained centered,
and only Equalizer was enabled.

| Band | Fixture frequency | Requested gain | Measured gain | Error |
|---:|---:|---:|---:|---:|
| 0 | 31 Hz | +12.00 dB | +10.10 dB | -1.90 dB |
| 1 | 62 Hz | +12.00 dB | +11.61 dB | -0.39 dB |
| 2 | 125 Hz | +12.00 dB | +11.22 dB | -0.78 dB |
| 3 | 250 Hz | +12.00 dB | +11.27 dB | -0.73 dB |
| 4 | 500 Hz | +12.00 dB | +11.26 dB | -0.74 dB |
| 5 | 1 kHz | +12.00 dB | +11.26 dB | -0.74 dB |
| 6 | 2 kHz | +12.00 dB | +11.25 dB | -0.75 dB |
| 7 | 4 kHz | +12.00 dB | +11.14 dB | -0.86 dB |
| 8 | 8 kHz | +12.00 dB | +11.55 dB | -0.45 dB |
| 9 | 16 kHz | +12.00 dB | +10.20 dB | -1.80 dB |

Bands 1 through 8 meet the Version 1 requirement of gain within 1 dB. The
31 Hz and 16 kHz edge bands target the correct frequencies but fall short by
1.90 and 1.80 dB. A separate Band5 value of 12 requested -12 dB and measured
-11.26 dB, exactly symmetric with its +11.26 dB result.

Do not compensate the two edge bands in the importer yet. A Windows capture
must establish whether the same CA0132 DSP filters behave identically under
the Creative driver. If Windows reaches the full requested gain, characterize
the Linux filter centers and DSP parameters before changing the mapping.

## Safety and limits

All completed captures restored the exact original mixer state. No CA0132,
HDA, ALSA, or DSP warning or error appeared in the kernel journal. One initial
Surround attempt encountered a busy playback PCM before playing any fixture;
its partial capture was discarded recoverably, and the profile exit guard
restored the exact state before the successful repeat.

The raw captures and restore profile are retained privately outside the
repository. These measurements prove that the Linux controls produce
repeatable digital DSP changes on this AE-5. They do not prove Windows parity,
analog-output performance, subjective quality, speech semantics, or behavior
at other effect levels, routes, and sample rates.
