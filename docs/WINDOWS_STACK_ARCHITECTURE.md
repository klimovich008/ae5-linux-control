# How the Windows stack actually processes AE-5 effects

Interoperability findings from the vendor stack installed on this host's
Windows partition. They answer one specific question: what Sound Blaster
Command's Acoustic Engine/OutFX master actually controls.

Scope and method: the shipped INF was parsed, managed application IL was
traced, and the relevant native control DLL, render APO and kernel driver were
examined with static disassembly. This is a clean-room architecture record:
no vendor implementation, coefficient, firmware, binary, decompiler output
or data file is copied into this repository. Machine-local reports remain
private.

Versions examined: `ctxhda.inf` 6.0.105.0065 (2022-11-24),
`CtxRFX64.dll` (2022-12-20), and `Creative.SBCommand.exe` (2023-10-06,
.NET Framework 4.6.1, x86).

The mounted Windows installation was rechecked on 2026-07-27. All four live
binary hashes still match this table. The installed 0065 INF still registers
the same render APO, and the machine-local Ghidra reports still show the
master/child registration, endpoint-property write path, APO module chain and
passthrough branch described below. Those reports remain under
`~/.cache/ae5-control/ghidra-reports/` and are intentionally not committed.

This is a static architecture comparison, not a completed same-settings audio
A/B. The prepared Windows VM still requires interactive login, and the AE-5
analog outputs are currently unplugged. Runtime parity must be recorded
separately before any Windows/Linux sound-quality claim.

A 2026-07-27 runtime-readiness retry confirmed that the passed-through AE-5
device was healthy and the two Windows audio services were running, but Guest
Agent reported zero logged-in users and Sound Blaster Command was absent.
Because the render APO properties belong to a user render session, that
logged-out state was not used as an OutFX measurement. The cycle played no
audio and returned the card safely to Linux.

| Installed binary | SHA-256 |
|---|---|
| `CtxHda.sys` | `4be35390a2de694041cd20317ed5a148d4852e46f201945a346a8b2a2c79dccf` |
| `CtxRFX64.dll` | `07de141f54a6a128747cc76d69a5eb42963107ea89f05fdb09ce8bb1a3977770` |
| `CtxHdC64.dll` | `ac4ab46eebd8cba2577f47567eb6d83a4d0a2b9d7d1eeea829a2d4a37fd02761` |
| `Creative.SBCommand.exe` | `32c71d5ad40f5d3cc1bb35f756038e3de5c08e3291550f26ac9fa1cb1cabff58` |

## The finding

**The Windows render stack contains a real software implementation of the SBX
effects, and OutFX master is a group control over five effect properties. It
is not Direct Mode and is not a global CA0132 DSP-bypass command.**

The driver INF registers a software APO on every render endpoint:

```text
[RenderAPO.AddReg]
HKR,"FX\0",%PKEY_DisplayName%,,%CTRFX_FriendlyName%
HKR,"FX\0",%PKEY_SYSFX_PreMixClsid%,,%CTRFX_PREMIX_CLSID%
HKR,"FX\0",%PKEY_SYSFX_StreamEffectClsid%,,%CTRFX_PREMIX_CLSID%

CTRFX_FriendlyName = "Creative Render Audio Effects"
```

`CTRFX_PREMIX_CLSID` is
`{4F7DD42B-513E-42AE-B730-A64221F1F526}` and resolves to
`CtxRFX64.dll` in `C:\Windows\System32\`. Its native types identify the
effect implementations:

| Symbol observed | Corresponding SBX control |
|---|---|
| `CCrystalizerEfxMod`, `CTHXCrystalizerEfxMod` | Crystalizer |
| `CSVMEfxMod`, `CTHXSVMEfxMod` | Smart Volume |
| `CStereoSurround3EfxMod` | Surround |
| `CBassBoost`, `CBassBoostMixerLFE`, `CBassBoostMixerSmallSpeakers` | Bass / X-Bass |
| `CBassManagementEfxMod`, `CBassMgmtEfxMod` | Bass redirection |

with node identifiers `EFXNODE_BASSBOOST`, `EFXNODE_THX_CRYSTALIZER` and
`EFXNODE_THX_REALITY3D_BASSMGMT`. Static analysis of
`CAPOContainerEFX` confirms that its real-time method processes interleaved
floating-point audio through a module/buffer chain. It also contains an
explicit path that copies input samples to output without running that chain.
A matching `CtxMLX64.dll` covers capture.

## Exact OutFX master trace

Sound Blaster Command's `AcousticEnginePageViewModel.IsEnabled` setter calls
`SoundCoreSBXMasterFeature.SetEnabled(bool)`. The AE-5 maps that operation to
SoundCore feature `0x60000001`, parameter `2`, named both
`THXEfx Master OnOff` and `eParamEfxMasterControl_SBXMasterOnOff` in the
installed stack.

The native registration table in `CtxHdC64.dll` maps that master to five child
feature switches:

| Feature ID | Effect |
|---:|---|
| `0x10000001` | Surround |
| `0x10000002` | Dialog+ |
| `0x10000004` | Smart Volume |
| `0x10000008` | Crystalizer |
| `0x10000020` | Bass management |

The master handler disables all five child booleans when switched off and
restores their saved state when switched on. It also maintains the stack's
last-state and property-change notifications. The generic SoundCore write
path resolves the feature parameter to a 20-byte endpoint property key,
builds a `PROPVARIANT`, and calls the endpoint-property object's `SetValue`.
There is no CA0132 module/request write in this master handler.

This gives Windows OutFX precise semantics: it is a convenient software-effect
group switch. Turning it off does not unload the APO, change the physical
route, or enter Direct Mode. At the audio-processing layer, disabled modules
can take the APO's passthrough path.

`CtxHda.sys` was also searched for the master feature ID and associated
handlers. No scalar reference implementing `0x60000001` was found. The driver
does manage DSP firmware, playback routing and unmute state, so absence of
that master ID is not proof that no individual endpoint property is ever
mirrored into hardware.

## Normal playback transport comparison

The 0065 `CtxHda.sys` stream-control and helper paths were disassembled
separately from the OutFX property path. Its normal AE-5 setup programs the
same visible transport shape used by Linux:

- stream `0x05` routes source `0x43` to destination `0x00`;
- stream `0x18` routes source `0x09` to destination `0xd0`;
- the `0xd0` connection runs at 96 kHz with six stream channels; and
- the stream is enabled after the normal PLL/connection sequence.

This supports the Linux fix's scope: the reproducible reopen corruption came
from clearing and reassigning the HDA playback converter, not from an obvious
steady-state mismatch in those visible Windows/Linux routes. It does not
establish bit-identical hidden DSP state.

The OutFX conclusion is independent and stronger: Sound Blaster Command's
managed setter reaches the native endpoint-property write path, and
`CtxRFX64.dll` consumes those properties in its software processing chain.
The hardware driver disassembly contains normal routing and DSP setup but no
matching implementation of the `0x60000001` master feature. Thus a rejected
Linux CA0132 hardware-OutFX write cannot be called a Windows OutFX-on
comparison.

## Why this matters here

Linux takes the opposite approach. `snd_hda_codec_ca0132` implements the same
effects by programming the CA0132's on-board DSP through `dspio` commands, so
processing happens in the card's silicon.

Three consequences follow.

**It gives the Linux-only oscillation a concrete architectural suspect.** A
Windows user with Crystalizer or Bass enabled has a software APO performing
those operations. Linux explicitly enables CA0132 hardware effect modules.
That difference is consistent with the observed Linux instability, although
static analysis alone cannot prove that Windows leaves every hardware module
unchanged.

**It reframes Windows/Linux parity.** `docs/AUDIO_PARITY_MEASUREMENT.md`
compares analog output between the two. Those are not two implementations of
one algorithm; they are different algorithms in different places. Matching
them exactly through the hardware DSP may not be achievable, and a residual
difference is not necessarily a Linux defect.

**It offers a way to reduce exposure to the fault.** A Linux build can compute
effects in a PipeWire filter chain — the direct analogue of an APO — while
keeping the individual CA0132 effect modules disabled. That does not by itself
bypass the normal CA0132 router/mixer path; the project's Direct Mode is the
separate true-bypass option.

## What was not established

- Effect coefficients and parameter scaling. The `.hda` files beside the APO
  (`CtxRFX64.hda`, `CtxMLX64.hda`) hold configuration data that was not
  decoded, and would not be copied here if it were.
- Whether an individual Windows effect property is also mirrored into
  hardware. Software processing and master semantics are confirmed; complete
  hardware idleness is not.
- The full DSP initialisation sequence in `CtxHda.sys`.
- Proprietary coefficients or bit-identical algorithms. They are neither
  needed nor appropriate for the Linux implementation.

## Reproducing the observation

```sh
INF="/path/to/Windows/System32/DriverStore/FileRepository/ctxhda.inf_amd64_*/ctxhda.inf"
tr -d '\r' < "$INF" | awk '/^\[RenderAPO.AddReg\]/,/^$/'
strings -a /path/to/Windows/System32/CtxRFX64.dll |
  grep -E 'EfxMod|EFXNODE_'
```

The managed/native call trace requires a disassembler and is intentionally
summarized here rather than checked into Git as vendor-derived output.
