# How the Windows stack actually processes AE-5 effects

Interoperability findings from examining the vendor stack installed on this
host's Windows partition. Recorded because they change what this project
should expect from the hardware, and because they explain a fault we have
been unable to reproduce on Windows.

Scope and method: only shipped configuration files were parsed, plus symbol
and string tables read from binaries. No vendor code was disassembled for
reimplementation, and **no vendor binary, firmware, or data file is copied
into this repository** — consistent with the boundary in HANDOVER.md. What
follows is interface and architecture fact, which is what interoperability
work is permitted to establish.

Versions examined: driver package `ctxhda.inf` 1.05.0064 (2022-03-23),
`CtxRFX64.dll` 2.2 MB (2022-12-20), `Creative.SBCommand.exe` (2023-10-06,
.NET Framework 4.6.1, x86).

## The finding

**The SBX effects do not run on the CA0132 hardware DSP under Windows. They
run on the CPU, as a Windows Audio Processing Object.**

The driver INF registers a software APO on every render endpoint:

```text
[RenderAPO.AddReg]
HKR,"FX\0",%PKEY_DisplayName%,,%CTRFX_FriendlyName%
HKR,"FX\0",%PKEY_SYSFX_PreMixClsid%,,%CTRFX_PREMIX_CLSID%
HKR,"FX\0",%PKEY_SYSFX_StreamEffectClsid%,,%CTRFX_PREMIX_CLSID%

CTRFX_FriendlyName = "Creative Render Audio Effects"
```

`CTRFX_PREMIX_CLSID` resolves to `CtxRFX64.dll`, installed to
`C:\Windows\System32\`. Its symbol table names the effect implementations
directly:

| Symbol observed | Corresponding SBX control |
|---|---|
| `CCrystalizerEfxMod`, `CTHXCrystalizerEfxMod` | Crystalizer |
| `CSVMEfxMod`, `CTHXSVMEfxMod` | Smart Volume |
| `CStereoSurround3EfxMod` | Surround |
| `CBassBoost`, `CBassBoostMixerLFE`, `CBassBoostMixerSmallSpeakers` | Bass / X-Bass |
| `CBassManagementEfxMod`, `CBassMgmtEfxMod` | Bass redirection |

with node identifiers `EFXNODE_BASSBOOST`, `EFXNODE_THX_CRYSTALIZER` and
`EFXNODE_THX_REALITY3D_BASSMGMT`. A matching `CtxMLX64.dll` covers capture.
`Creative.SBCommand.exe` is a .NET front end that talks to this layer through
`Interop.EfxNodeInfo.dll`; the SBX sliders are APO parameters, not DSP
register writes.

## Why this matters here

Linux takes the opposite approach. `snd_hda_codec_ca0132` implements the same
effects by programming the CA0132's on-board DSP through `dspio` commands, so
processing happens in the card's silicon.

Three consequences follow.

**It explains the idle oscillation being Linux-only.** The DSP effect chain we
drive is a path the vendor's own Windows stack does not exercise for these
effects. A Windows user with Crystalizer and Bass enabled is running CPU code;
the hardware DSP is not doing that work. We are, as far as this evidence goes,
the ones putting that silicon path under sustained use — which is consistent
with meeting an instability no Windows user reports.

**It reframes Windows/Linux parity.** `docs/AUDIO_PARITY_MEASUREMENT.md`
compares analog output between the two. Those are not two implementations of
one algorithm; they are different algorithms in different places. Matching
them exactly through the hardware DSP may not be achievable, and a residual
difference is not necessarily a Linux defect.

**It offers a way out of the fault.** If the vendor computes these effects in
software on Windows, a Linux build can do the same in a PipeWire filter chain
— the direct analogue of an APO — and leave the unstable hardware path alone.
That trades hardware offload for stability and would need its own DSP design
and measurement work, but it is an architecture the vendor themselves chose.

## What was not established

- Effect coefficients and parameter scaling. The `.hda` files beside the APO
  (`CtxRFX64.hda`, `CtxMLX64.hda`) hold configuration data that was not
  decoded, and would not be copied here if it were.
- Whether any effect still reaches hardware on Windows. The registration
  proves a software path exists and is installed on every render endpoint; it
  does not prove the DSP is idle.
- The DSP initialisation sequence `CtxHda.sys` performs, which is the natural
  next question for the oscillation and would require disassembly.

## Reproducing the observation

```sh
INF="/path/to/Windows/System32/DriverStore/FileRepository/ctxhda.inf_amd64_*/ctxhda.inf"
tr -d '\r' < $INF | awk '/^\[RenderAPO.AddReg\]/,/^$/'
strings -a /path/to/Windows/System32/CtxRFX64.dll | grep -E 'EfxMod|EFXNODE_'
```
