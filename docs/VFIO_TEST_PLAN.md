# AE-5 VFIO kernel A/B test plan

PCI passthrough is feasible on the current development host, but it is an
optional test environment rather than part of the application. It can compare
two Linux kernels against the same physical AE-5 without repeatedly changing
the host kernel. It cannot replace final cold-boot, suspend, and recovery tests
on the host.

The virtualization stack, session guests, and system guests were installed on
2026-07-24. Twelve guarded Linux managed-passthrough cycles completed with
exact host recovery. A separate Windows cycle has since attached the physical
AE-5 successfully and validated live Creative driver and Command discovery.

Run the read-only, fail-closed hardware check at any time:

```sh
bash scripts/check-vfio-host.sh
```

It discovers the exact supported subsystem rather than trusting a saved PCI
address, requires the card to be alone in its current IOMMU group, verifies the
audited bus-reset path and recovered `snd_hda_intel` state, and reports missing
VM tools. Once the VM packages are installed, make their absence fatal with
`--require-tools`. The checker never detaches, resets, or writes to the card.

## Read-only host audit

The audit on 2026-07-24 found:

- AMD Ryzen 7 5700X3D with AMD-V;
- `/dev/kvm` present and the `kvm_amd` and `kvm` modules loaded;
- AE-5 PCI function `0000:29:00.0`, Creative `1102:0012`, subsystem
  `1102:0051`;
- the AE-5 is the only function in IOMMU group 28;
- the host currently binds it to `snd_hda_intel`;
- the function exposes the sysfs reset attribute and reports
  `reset_method=bus`;
- `qemu-kvm` 10.2.2, libvirt 12.0.0, `virt-install` 5.1.0, OVMF
  20260508, and `swtpm` 0.10.1 are installed;
- `scripts/check-vfio-host.sh --require-tools` reports
  `vfio_preflight=ready`.

An isolated IOMMU group is the important safety property: VFIO treats the
group as the unit of ownership. The kernel's
[VFIO documentation](https://docs.kernel.org/driver-api/vfio.html) explains
that every device in a group must be available to VFIO before userspace can
obtain the group. Here, assigning group 28 does not also assign a GPU, storage
controller, or another host device.

The reset result is encouraging but not a guarantee. Every guest shutdown must
prove that the function can return to `snd_hda_intel`, recreate the ALSA card,
and play normally before another cycle is attempted.

## Boundaries

- The host cannot use the AE-5 while the guest owns it. The Fifine USB
  microphone remains available to the host for acoustic measurements.
- Docker, a normal WSL environment, and emulated QEMU audio devices cannot
  exercise the AE-5 PCI function, codec, or DSP.
- No GPU passthrough is needed. A guest can use SPICE or another virtual
  display while only the AE-5 is assigned.
- VFIO gives the guest direct device ownership; it is useful for behavioral
  A/B tests, not for intercepting proprietary device transactions.
- A VM reboot and PCI bus reset do not reproduce removal of power from the
  physical card. Host cold boots remain the authoritative cold-start test.
- A Windows guest may provide a controlled behavior comparison using Creative's
  normally installed driver, subject to its licence. It is not required to
  validate the Linux ACP headphone-path fix.

## Staged setup

1. The QEMU/KVM, libvirt, `virt-install`, UEFI firmware, and software TPM
   packages are installed.
2. The `ae5-kernel-test-f44` Fedora 44 session guest and the
   `ae5-kernel-test-f44-system` system guest are available, powered off, and
   recoverable from the original flattened image.
3. The Fifine microphone remains available to the host. Its acoustic test path
   and the AE-5's restored host playback path have already been validated.
4. Three initialization/control cycles passed with `0000:29:00.0` attached
   temporarily as a managed PCI host device. Libvirt's
   [`hostdev` documentation](https://www.libvirt.org/formatdomain.html#host-device-assignment)
   describes `managed='yes'`, which detaches the function from the host before
   guest startup and reattaches it after guest shutdown.
5. The Windows comparison guest now uses the same managed, on-demand
   assignment model. Persistent early-boot VFIO binding remains unnecessary.
   Its remaining hardware step is a guarded acoustic comparison, followed by
   shutdown and the same exact host-recovery audit.

The PCI address is a host address; it must not be copied into an unrelated
machine's configuration.

### Validated guest

The guest uses Fedora Cloud Base 44-1.7 from the
[official Fedora Cloud download](https://fedoraproject.org/cloud/download/).
The signed checksum file verified with Fedora 44 key fingerprint
`36F612DCF27F7D1A48A835E4DBFCF71C6D9F90A6`, and the QEMU image matched
SHA-256
`28680fe5b371a5a82ebf43a31926e086a168e59949d03969c5093e7071f90b7f`.

It runs under the unprivileged `qemu:///session` connection with:

- four host-passthrough vCPUs, 4 GiB RAM, KVM, Q35, and UEFI;
- a 40 GiB copy-on-write disk backed by the verified Fedora image;
- `passt` user networking and SSH bound only to `127.0.0.1:2222`;
- no emulated audio device and no PCI host device;
- Fedora 44 with stock kernel `6.19.10-300.fc44.x86_64` retained as a fallback;
- the existing `rog5_linux` SSH public key and no generated password.

```sh
virsh --connect qemu:///session list --all
ssh -i ~/.ssh/rog5_linux -p 2222 fedora@127.0.0.1
virsh --connect qemu:///session snapshot-list ae5-kernel-test-f44
```

The session guest proves KVM, image boot, UEFI, storage, networking, and guest
access. An unprivileged session daemon cannot safely perform managed host
detachment. The physical-card tests therefore use the separately inspected
`qemu:///system` guest.

### Installed Windows comparison guest

On 2026-07-26, the separate `ae5-windows-compare` guest was installed under
`qemu:///session` from Microsoft's
[Windows 11 Enterprise Evaluation](https://www.microsoft.com/en-us/evalcenter/download-windows-11-enterprise)
media. The English 25H2 V2 ISO matched Microsoft's published SHA-256:

```text
a61adeab895ef5a4db436e0a7011c92a2ff17bb0357f58b13bbc4062e535e7b9
```

The installed image is Windows 11 Enterprise Evaluation x64, version
`10.0.26200`, build `26200`. The original session guest is powered off, has
autostart disabled, and retains:

- six host-passthrough vCPUs and 12 GiB RAM;
- Q35, Secure Boot OVMF, a private variable store, and an emulated TPM 2.0
  CRB device;
- a 100 GiB qcow2 disk;
- user-mode networking through an emulated E1000E adapter;
- a SPICE display listening only on `127.0.0.1`;
- QEMU Guest Agent `110.0.2` and signed Red Hat VirtIO guest drivers
  `100.101.104.28500`;
- no emulated sound device, PCI host device, USB host device, physical disk,
  mounted CD image, or autostart.

The original NTFS partition was never attached to the guest. Its three
required Sound Blaster Command setting scopes were copied from the host's
read-only mount into a private read-only transfer ISO. That ISO has SHA-256
`642f48ba3b37d28905d7885da92d4e1e345f9578c7af0ea2669b3154d98b1ee3`
and contains exactly 126 regular files totaling 183,478 bytes. A sorted
relative-path, length, and SHA-256 manifest has SHA-256
`0ca0d5a09f6848af6ff05dc38dad766719711ef06dbb25bdd567534e9b3dfda4`.
Every transferred file matched its source before the transfer CD was ejected.

The existing Creative installers were checked against official sources and
then validated with Windows Authenticode before execution:

| Component | Version | SHA-256 |
|---|---:|---|
| Sound Blaster Command master installer | 3.4.92.00 | `de40104936219bd1ae32304405e1144f07cd6b81cd1d219cf2bacc7991b70be3` |
| AE-Series driver installer | 1.0.01.06 | `e57a2291728dad192adc191c6fafe1d5bfabfbbbf77a1765e12cda80b3af74da` |
| Command update installer | 3.5.10.00 | `2b6d9bcd9f0a436c90a34e3af5ee3706c3e749cbe703ebddccfad9c21747a722` |

Creative's live
[Command release manifest](https://files.creative.com/creative/bin/apps/swureleases/win/sbcommand/release/SBCDProducts.json)
identifies version `3.5.10.00` and MD5
`ac68d97716a59d9588cf9e8562f7d1c0`; the downloaded update matched that
value. The installed `Creative.SBCommand.exe` is version `3.5.10.0`, has
SHA-256
`32c71d5ad40f5d3cc1bb35f756038e3de5c08e3291550f26ac9fa1cb1cabff58`,
and is byte-identical to the executable in the inspected source Windows
installation.

The staged settings were then copied into Command's exact versioned user,
AE-5 product, and shared headphone-metadata locations. The newly installed
shared metadata was already byte-identical, so it was not overwritten. An
interactive no-device launch displayed the expected request to connect a
supported device and left all 126 imported files unchanged.

Because the files came from a read-only ISO, 93 destination user files
inherited the ReadOnly attribute. Command's first physical-device launch
detected the AE-5 but could not initialize its product view because
`user.config` was not writable. Clearing ReadOnly and Hidden only on those 93
destination copies preserved their content aggregate exactly. This transport
normalization is now a mandatory migration step; content verification alone
is insufficient.

Two powered-off recovery points exist outside the repository:

| Recovery point | SHA-256 |
|---|---|
| Clean Windows and guest tools | `80997d09344e999b2607330336cc786784b01d0bb5d9e3799d581f695840f315` |
| Command 3.5.10 and verified settings import | `a3a3f3caddacc55ad47ab1b9905ee4ed9258a751fe4a43bd4ef6c3ca2ece1183` |

Both qcow2 images pass `qemu-img check`; the milestone copy is a read-only
Btrfs reflink. No password, Windows image, Creative binary, setting file,
profile identifier, or private manifest is stored in this repository.

The session guest remains the safe installation environment;
`qemu:///session` cannot detach the host's AE-5. The exact powered-off
Command/settings milestone was uploaded to the system `default` storage pool
as `ae5-windows-compare-system.qcow2`. A streamed download matched SHA-256
`a3a3f3caddacc55ad47ab1b9905ee4ed9258a751fe4a43bd4ef6c3ca2ece1183`
and passed `qemu-img check`.

The corresponding `qemu:///system` domain is defined with autostart disabled
and no persistent host device. Each guarded cycle temporarily adds exactly one
managed hostdev to its powered-off configuration: the audited
`1102:0012/1102:0051` AE-5 function. Starting it detached the card to
`vfio-pci` without assigning the FiFine microphone, GPU, storage, or another
IOMMU-group member. Linux PipeWire and WirePlumber were intentionally stopped
while Windows owned the card, and the hostdev was removed again after clean
shutdown.

Windows installed the latest Creative-signed AE-Series driver
`6.0.105.65`. Both physical Creative PnP nodes report healthy status, the
Windows audio and Creative services run automatically, and Command 3.5.10.0
recognizes the full AE-5 product UI. After live initialization, 125 of 126
imported files remained byte-identical. Only `user.config` changed: Command
added its hardware audio-format cache and updated one saved surround runtime
flag while preserving the selected profile, presets, and route settings.

Live UI checks established that Creative's profile section type is
`Headphones = 0` and `Speakers = 1`; narrow managed reflection independently
confirmed the same values in three profile-related enum types. This corrected
the Linux importer's previously reversed selection. The speaker and headphone
views then matched their respective imported states.

All AE-5 outputs remained physically unplugged and no audio was played.
Windows render endpoints were held at exactly 20% and muted before UI checks.
Command nevertheless unmuted the endpoint when changing from Speakers to
Headphones, so the cap and mute were immediately restored and independently
verified. Every future output, profile, format, or device transition must
reapply and re-check both values before the playback preflight can pass.

After the comparison, the guest must shut down cleanly, the inactive hostdev
must be removed, and the host must prove `snd_hda_intel`, ALSA, PipeWire,
WirePlumber, routes, mixer hashes, and closed PCMs recovered before another
cycle.

### Windows playback-readiness retry

A later cycle re-ran the complete non-mutating VFIO preflight, saved the host
state, stopped the Linux audio services, attached only the audited AE-5, and
booted the system Windows guest. QEMU Guest Agent reported Windows
`10.0.26200`, the Creative controller, HDA codec, Speakers, SPDIF, and What U
Hear endpoints all healthy, and no audio player running. The cycle stopped
before fixture transfer or playback because the private test account rejected
the unattended credential retained outside the repository. The credential
was tried once and was neither printed nor copied into project evidence.

Guest Agent performed a clean Windows shutdown. The AE-5 rebound to
`snd_hda_intel`; PipeWire, PipeWire Pulse, and WirePlumber became active; the
VM returned to an off state with no hostdev and no QEMU process. The recovered
raw mixer, simple mixer, and desktop-route SHA-256 values exactly matched the
pre-cycle values:

```text
raw mixer    743e602e8066bea0aed9145669584497289fdb459c4c8450913513dbb7e15bc1
simple mixer 26a75bb94621e15023ebb28bb3a3da92c63d210f0e657b74478187256d39142c
route status 59cb50175ab2858fc7146fd33ddcd8b01627c7bd1344eb89cc9a1c5f6e5d838e
```

All four AE-5 PCM substreams were closed, Headphone and Microphone routing
remained healthy, Low headphone gain and the 20% PipeWire ceiling remained in
force, and the post-cycle VFIO preflight passed. No audio was played. Before
another Windows capture, repair the local test-account login interactively,
shut down, preserve a fresh powered-off recovery point, and verify the exact
fixture inside the guest. Do not retry or weaken the stale credential.

A later readiness retry after the `7.1.4-ae5-stable` Linux package
qualification reached the same boundary. Guest Agent reported zero logged-in
users, while `Win32_SoundDevice` reported `Sound BlasterX AE-5` status `OK`
and both `AudioEndpointBuilder` and `Audiosrv` were running automatically.
Sound Blaster Command was not running because no user session existed.
Therefore no OutFX property was changed and no Windows audio was played or
captured. The guest then shut down, its temporary hostdev was removed, the
post-cycle VFIO preflight passed, both system guests remained off, the AE-5
rebound to `snd_hda_intel`, and the user audio services were restored.

### Candidate kernel smoke test

The Wedge Angle patch was applied to `sound.git` `for-next` commit
`61471f29f315` and built in the guest as `7.2.0-rc2-ae5-wedge+`. Before any
install or firmware change, powered-off `pre-wedge-install` and
`pre-nosecure-boot` snapshots preserved the recoverable guest states.

The guest's initial signed Fedora kernel was validated with Secure Boot.
Secure Boot was then disabled for this VM only, using a separate OVMF variable
store, because the local candidate is not distribution-signed. The candidate
booted successfully with:

- the Btrfs root and EFI environment intact;
- VirtIO networking, the loopback-only SSH forward, and KVM intact;
- the matching `snd-hda-codec-ca0132` module available and loadable;
- no failed systemd units.

The guest also has `pciutils`, `alsa-utils`, SoX, and Fedora
`alsa-firmware-1.2.4-17.fc44` installed for the physical test. All four
Creative firmware hashes match [the source inventory](SOURCE_INVENTORY.md).
The powered-off `vfio-tools-ready` snapshot preserves this state.

No host device was present in the domain XML during this smoke test. The
physical AE-5 remained in IOMMU group 28 and bound to the host
`snd_hda_intel` driver. The later system-guest cycles validated the corrected
`30`-degree control default on the physical card.

The same pinned source was then rebuilt as the integrated
`7.2.0-rc2-ae5-integrated+` candidate with these four functional patches:

- `ca0132-wedge-angle-default.patch`;
- `ca0132-eq-preset-control-cache.patch`;
- `ca0132-ae5-hide-ineffective-wuh-controls.patch`;
- `ca0132-dsp-image-bounds.patch`.

The diagnostic `ca0132-speaker-eq-address-probe.patch` was deliberately
excluded. The combined source passed `git diff --check`; its CA0132 diff has
106 insertions and 61 deletions across `Kconfig`, `Makefile`, and `ca0132.c`.
The kernel image, CA0132 module, and ALSA sequencer modules built successfully.
The x86 instruction decoder exercised 8,073,002 instructions, and the random
instruction test completed 1,000,000 cases with no error.

The first boot found that Fedora's `snd-pcm` modprobe policy also loads
`snd-seq`, which the minimal guest configuration had omitted. Enabling
`CONFIG_SND_SEQUENCER=m` and `CONFIG_SND_SEQ_DEVICE=m`, rebuilding, and
reinstalling fixed the dependency rather than bypassing the distribution
policy. The final candidate booted with the Btrfs root and KVM intact,
`snd-hda-codec-ca0132` and `snd-seq` loaded, and no failed systemd unit. The
only error-priority kernel messages were the existing no-device VM warnings
for unsupported TDX, systemd's BPF filesystem restriction, and deprecated
SELinux `checkreqprot` use.

This was the build, boot, and no-device module test. The physical validation
below subsequently proved firmware initialization and the read-visible
effects of the four-patch candidate.

### Prepared system import

The earlier powered-off `vfio-tools-ready` state remains available as the
standalone Wedge-only `ae5-kernel-test-f44-system-import.qcow2`. It has no
backing file, occupies 3.81 GiB for a 40 GiB virtual disk, passes
`qemu-img check`, and has SHA-256:

```text
a7a445b06ecf7b7f6adf4827de95a75a7fad9659ccafe7a09d6242172f6c11b1
```

A second flattened image,
`ae5-kernel-test-f44-integrated-system-import.qcow2`, contains the final
four-patch candidate. After booting that exact file once, it occupies 5.14 GiB
for a 40 GiB virtual disk, has no backing file, passes `qemu-img check`, and
has SHA-256:

```text
bfca0fdfa57cc7b9fab13c91a2a58584233c257638f636573b85a29c1d091637
```

Temporary unprivileged domains booted each exact standalone image and then
shut down cleanly. For the integrated image, the check verified
`7.2.0-rc2-ae5-integrated+`, the Btrfs root, KVM, the matching loadable CA0132
module, the sequencer dependency, zero Creative PCI devices, and zero failed
systemd units. The temporary domains and their NVRAM were removed; both disks
remain powered off. Neither domain contained a host device.

The coordinated post-reboot import completed on 2026-07-24:

1. `virtqemud.socket` became active and `qemu:///system` connected;
2. the integrated standalone bytes were uploaded into a persistent
   system-owned `default` pool at `/var/lib/libvirt/images`;
3. the inactive `ae5-kernel-test-f44-system` domain was defined with
   non-Secure-Boot UEFI, four host-passthrough vCPUs, 4 GiB RAM, a headless
   display, NAT networking, no emulated audio, and no host device;
4. its complete inactive XML, storage, NVRAM, and network were inspected
   before startup;
5. the original session guest, snapshots, and standalone image were retained
   unchanged as recovery sources.

The imported volume was byte-identical to the standalone source before its
first boot. The system guest then booted `7.2.0-rc2-ae5-integrated+` from
Btrfs, loaded the matching CA0132 and ALSA sequencer modules, retained KVM and
networking, exposed zero audio or Creative PCI devices, and had zero failed
systemd units. Its error-priority log contained only the same unrelated
no-device VM messages recorded above.

After a clean shutdown, a downloaded copy of the system volume passed
`qemu-img check`, had no backing file, occupied 5.17 GiB for a 40 GiB virtual
disk, and had SHA-256:

```text
195013902470bf90ff9f506e2003668d80198c266eeec4a0583cd51a2fcccb6e
```

The system and session domains are powered off, system-domain autostart is
disabled, and no QEMU process remains. The physical AE-5 stayed bound to
`snd_hda_intel` throughout. Its saved Creative ALSA state matched exactly
after shutdown, including Headphone output, Microphone input, Front enabled
at 90, and the card-specific WirePlumber headphone port. The only state delta
observed while PipeWire opened and suspended the card was the volatile PCM
`Playback Channel Map` changing between unset and FL/FR; no user-facing mixer
control changed. The host kernel recorded no matching CA0132, HDA, VFIO,
reset, or timeout warning.

### Reviewed host-device fragment

The only planned passthrough device is the already-audited AE-5 function:

```xml
<hostdev mode='subsystem' type='pci' managed='yes'>
  <source>
    <address domain='0x0000' bus='0x29' slot='0x00' function='0x0'/>
  </source>
</hostdev>
```

This fragment was attached only to the powered-off system-domain configuration
for each physical cycle and removed immediately after each shutdown. Before
every later attachment:

1. rerun `scripts/check-vfio-host.sh --require-tools`;
2. confirm `0000:29:00.0` still resolves to `1102:0012/1102:0051`, is alone
   in its IOMMU group, and is bound to `snd_hda_intel`;
3. shut down every process using the AE-5 and confirm the Fifine path is
   available;
4. save ALSA, PipeWire, and PCI-driver state for recovery;
5. inspect the complete powered-off system-domain XML before scheduling the
   detach/start.

No persistent early-boot VFIO binding, ACS override, or manual sysfs
host-driver unbind is needed. Every future start remains subject to the same
preflight, no-open-stream, recovery, and stop conditions.

### First physical validation cycles

Before the first detach, a private recovery set captured all 46 applicable
profile controls, the raw Creative ALSA state, WirePlumber defaults and routes,
the routing state, and a full Linux report. The profile passed
`profile-check --allow-high-gain`. PipeWire and WirePlumber were then stopped,
and `fuser` confirmed that no process had an ALSA device open.

In all five cycles, managed startup rebound host `0000:29:00.0` from
`snd_hda_intel` to `vfio-pci`. The guest received the exact
`1102:0012/1102:0051` function at `0000:07:00.0`, bound it to
`snd_hda_intel`, loaded the integrated CA0132 module, and reported
`ca0132 DSP downloaded and running`.

The first, read-only cycle verified:

- Wedge Angle initialized to `30` inside the advertised `20..180` range;
- the What U Hear capture PCM remained present while its ineffective mixer
  control was absent;
- Flat and every EQ band initialized to raw value `24`;
- 72 ALSA controls, 46 simple controls, and zero failed systemd units;
- no matching CA0132, HDA, codec, DSP, firmware, or timeout warning.

The second cycle selected all ten factory EQ presets. Every ten-band vector
matched the patch table, and an ALSA monitor saw value notifications for
Acoustic's five changed bands plus the preset control, then the same events
when returning to Flat. Flat restored the complete guest mixer hash
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`
exactly.

The third cycle exercised Wedge Angle while Voice Focus was enabled. Its
initial raw and simple-mixer values were both `30`; writes of `20`, `30`, and
`180` read back exactly through both APIs. Returning to `30` restored the same
complete guest mixer hash. No PCM stream was open, no guest unit failed, and
no CA0132, DSP, or timeout warning appeared.

The fourth cycle selected Headphone, Low gain, disabled output effects, and
played the hash-verified two-second 997 Hz fixture through direct ALSA. With
the headphones beside the host's Fifine microphone and not worn, Master 65
raised mean 987–1007 Hz power by 21.75 dB over the quiet baseline and 19.59 dB
over a Front-muted negative control. A second positive capture repeated within
1.04 dB. Front was confirmed off during the negative-control stream and
restored by an exit guard.

With Front still guarded off, the guest's physical `CA0132 What U Hear` PCM
captured the same fixture at 48 kHz, signed 32-bit stereo. Its measured RMS was
-21.26 dBFS and its strongest analyzed bin was 996.09375 Hz. The ineffective
What U Hear mixer controls remained absent. Restoring the saved guest ALSA
state returned the complete mixer hash to
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`;
no unit failed and no matching CA0132, HDA, DSP, firmware, or timeout warning
appeared.

The fifth cycle exercised repeated initialization and route changes. Three
warm guest reboots each produced a new boot ID, retained
`7.2.0-rc2-ae5-integrated+`, initialized the DSP exactly once, exposed 72
controls and 46 simple controls, and restored the complete guest mixer hash.
Wedge Angle remained `30`, EQ remained Flat, the What U Hear PCM remained
available without its ineffective controls, no systemd unit failed, and no
matching driver warning appeared.

Fifty alternating `Output Select` changes then matched both the ALSA enum and
the physical codec-pin state on every transition. Headphone enabled `0x11`
while disabling `0x0b`, `0x0f`, and `0x10`; Speakers enabled `0x0b`, `0x0f`,
and `0x10` while disabling `0x11`. No PCM was open. The final Speakers state
restored the exact guest mixer hash with no relevant warning.

Each clean shutdown returned the card automatically to host `snd_hda_intel`
and recreated readable ALSA controls in about two seconds. The complete raw
Creative state matched the saved file after every cycle, so no fallback
restore ran. PipeWire and WirePlumber restarted, the AE-5 returned as the
default sink on the card-specific headphone port, the Fifine remained the
default source, and the full host mixer hash returned to
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`.
VFIO preflight passed again, the hostdev was removed, and no QEMU process
remained. The fifth cycle also preserved the exact WirePlumber default-node
and route files. Ambient captures were deleted after retaining fixture and
capture hashes plus the derived measurements.

The sixth cycle repeated the initialization and recovery gate on maintained
Linux `6.18.40`. The exact LTS stack first passed KUnit and no-device boots in
both guests. With the physical card, it reproduced the 72/46 control counts,
Wedge `30`, Flat ten-band EQ vector, What U Hear PCM, hidden ineffective
controls, one DSP initialization, and zero relevant warnings.

The LTS cycle also tested the upstream routing pair absent from 6.18.40.
Starting with HP/Speaker auto-detect on, a manual Headphone write turned
auto-detect off and enabled only codec pin `0x11`; Speakers enabled `0x0b`,
`0x0f`, and `0x10`. The exact packaged CLI safely changed Wedge to `20` and
back to `30`, and its RPM installed and removed without changing the guest
mixer hash.

Clean shutdown returned the card to host `snd_hda_intel` in about two seconds.
The host raw ALSA file, all 47 app profile controls, complete mixer hash,
WirePlumber defaults/routes, default AE-5 sink, Fifine source, and packaged
headphone port all returned without a fallback restore. The full evidence and
reproduction series are in
[`LTS_KERNEL_VALIDATION.md`](LTS_KERNEL_VALIDATION.md).

The seventh and eighth cycles tested the separate read-only SpeakerEQ address
probe on the same Linux `6.18.40` stack. Both candidates first passed a
no-device boot. The seventh queried immediately after firmware download; the
eighth moved the same one-word `MASTERCONTROL_QUERY_SPEAKER_EQ_ADDRESS` GET
until after `ae5_setup_defaults()` completed. Each physical boot initialized
the DSP once and emitted exactly one `SpeakerEQ address query failed: -5`
after `dspio_scp: send scp msg failed`. Neither returned an address. The late
result rules out incomplete AE-5 setup as the simple cause, so no undocumented
request variant or firmware upload was attempted.

The failed query did not disturb the normal card path. Both boots retained the
known 72/46 control counts, Wedge `30`, Flat EQ vector, What U Hear PCM,
hidden ineffective controls, zero failed units, and mixer hash
`c5d3a2673054ea6b71b562e3f12923c51c00af9c79af17137948e4474818de68`.
On the eighth cycle, a Front-muted direct-ALSA 997 Hz fixture was captured
through What U Hear at 48 kHz signed 32-bit stereo. The strongest bin was
`996.09375 Hz`, RMS amplitude was `0.048654`, and the saved guest state
restored the exact mixer hash.

Both shutdowns again returned the card to host `snd_hda_intel`. Raw ALSA state
was byte-identical, all 47 application controls matched, WirePlumber defaults
and routes were byte-identical, stream properties were canonically identical,
the expected AE-5/Fifine defaults and packaged headphone port returned, and
the host mixer hash was
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`.
The inactive domain had no `hostdev`, both guests were off, no QEMU process
remained, and VFIO preflight passed. Because the query failed, its three-boot
address-stability and 50-switch success gates were not applicable; the
non-probe kernels retain their separate routing and 50-switch evidence.

The ninth cycle added the five-onboard-LED candidate to the maintained
`6.18.40` production stack, without the diagnostic SpeakerEQ probe. A
card-less boot first verified the new CA0132 and multicolor LED module
dependency. The physical boot then registered exactly five standard
multicolor devices and accepted solid red, green, and blue frames, independent
per-LED values, and an individual brightness off/on transition. The DSP
initialized once, the complete guest mixer hash stayed unchanged, no PCM was
open, and no relevant warning appeared. Visible color confirmation remains
required.

That shutdown again returned the exact raw ALSA and 47-control app state,
complete mixer hash, all three WirePlumber files, AE-5/Fifine defaults,
`snd_hda_intel` binding, clean inactive domain XML, and ready VFIO preflight.
The kernel artifacts and detailed LED-class evidence are recorded in
[`LTS_KERNEL_VALIDATION.md`](LTS_KERNEL_VALIDATION.md).

The tenth cycle installed the normal-user RGB application RPM on that same
kernel. Its exact udev match exposed only the five validated
`brightness`/`multi_intensity` pairs; the headless non-`audio` user then
applied, persisted, restored, and failure-tested unified and independent
colors without `sudo`. The complete guest mixer hash never changed, and RPM
removal returned all ten sysfs attributes to root-owned mode `0644` while
preserving the user configuration.

After shutdown, all 47 writable host controls, WirePlumber defaults, the
AE-5/Fifine default devices, and driver binding recovered. The sole raw-mixer
difference was the expected read-only PCM channel map changing from the active
pre-test stereo stream to no stream. The hostdev was removed, the guest was
off, and VFIO preflight passed.

The eleventh cycle used the production RGB kernel for an external headphone
level, mute, and gain matrix. With output effects off and Low gain, repeated
997 Hz captures at Master raw 55, 60, and 65 measured `+5.21 dB` and
`+4.34 dB` for the two advertised `+5.00 dB` steps, with repeat spreads from
`0.18` to `0.58 dB`. Master mute matched the quiet baseline within `0.88 dB`;
a confirmed Front-muted repeat suppressed the tone by more than 34 dB. At the
attenuated Master 55 setting, Medium and High gain measured `+1.28 dB` and
`+7.04 dB` relative to Low.

The same guarded cycle intentionally stopped short of claiming filter
validation: at 18 kHz the microphone path put Slow and Fast only 1.33 and
5.10 dB above its baseline, and Minimum Phase varied by 14.44 dB between
repeats. An attenuated electrical capture is required. Guest restoration
returned the exact known mixer hash, one DSP initialization, zero failed
units, and no relevant warning. Shutdown returned all current host profile
controls and all three WirePlumber files exactly, restored the known
no-stream mixer hash and AE-5/Fifine defaults, removed the hostdev, and left
VFIO preflight ready. Raw ambient recordings were deleted after reduction.

The twelfth cycle validated the application's Linux-driver-default reset on
the same production RGB kernel. The current application first rejected the
stock host's invalid Wedge `11` before creating a backup or changing hardware.
In the guest, Wedge initialized to valid value `30`; a controlled state then
set Wedge `20`, Surround off, low headphone gain, and Master `0`. The
restorable-backup preflight accepted all 29 reset controls. Applying defaults
changed Wedge to `30` and enabled Surround while Master remained `0`. The
47-control automatic backup restored complete raw mixer hash
`6d188e32efe043e3224f5f8b5a69e23c7c52ecb85f0bd16cabff95968e75bee8`
and simple mixer hash
`3f063558459510a7133c8c81bd7031d23726a29d00070dc4ba0d9b6e1510c88b`
exactly.

The audio gate used a two-second 997 Hz fixture with a `-20.00 dBFS` peak
(10% digital amplitude), low gain, and Master `0`. The physical output was
therefore effectively silent and below the explicit 20% ceiling. The
card's What U Hear PCM captured the stream with an exact `997.000 Hz` FFT
peak. Every PCM closed afterward. Restoring the guest's original ALSA state
also reproduced both complete original mixer hashes. The DSP initialized
once, no unit failed, and no relevant CA0132/HDA warning appeared.

Clean shutdown returned the host's complete raw mixer hash
`3e595532348efe1e2e9c066039131e97505cb9b71bc6bfd8fa8a59301091e802`,
simple mixer, output and input route reports, three WirePlumber files, and PCM
state byte-for-byte. The AE-5 rebound to `snd_hda_intel`, all desktop audio
services became active, the inactive domain had no hostdev, no QEMU process
remained, and VFIO preflight was ready.

The powered-off system volume passed `qemu-img check` after all five cycles
and had SHA-256
`d7ee6ed48b3ba5800e5c93576fdbbec76bbe0eb81d2708c59dd600058262a664`.
The image is root-owned mode `0600`; the check could not be repeated after the
sixth through eleventh cycles without interactive host authorization. Libvirt
completed each shutdown without a storage error.
The untouched standalone recovery image retained SHA-256
`bfca0fdfa57cc7b9fab13c91a2a58584233c257638f636573b85a29c1d091637`.
Voice Focus recording, speaker/line-out and digital playback, guest suspend,
and repeated host cold boots remain separate gates.

## Per-kernel test matrix

Run the same sequence first on a known-good kernel and then on one candidate
kernel:

1. Start the guest and verify `1102:0012/1102:0051` with `lspci -nnk`.
2. Confirm the CA0132 DSP and ALSA controls initialize without new kernel
   warnings.
3. Select Speakers, Line Out, and Headphones in turn; verify the codec route,
   `Output Select`, Front switch, and audible output.
4. For Headphones, run the guest's
   `scripts/audio-parity.sh playback-preflight direct` against the exact
   hash-verified 997 Hz fixture. Only after it passes, record a short baseline,
   playback sample, and Front-muted negative control through the host's Fifine
   microphone. Retain derived measurements, not ambient recordings.
5. Exercise stop/start, guest reboot, and guest suspend/resume where supported.
6. Shut down the guest. Verify the AE-5 rebinds to `snd_hda_intel`, the ALSA
   card reappears, WirePlumber restores the intended profile, and host playback
   works.
7. Repeat enough times to expose reset or state-restoration failures before
   moving to the candidate kernel.

For each run, retain kernel version, boot journal excerpts, PCI driver, ALSA
control snapshots, PipeWire route, fixture hash, derived acoustic result, and
whether host reattachment passed.

The same preflight is mandatory before the host-recovery playback in step 6
and before every other non-silent guest or host stream. A failed preflight is a
stop condition, not an instruction to lower values automatically.

## Stop conditions

Stop the experiment and leave the guest off if:

- another device appears in IOMMU group 28;
- the host cannot detach the card cleanly;
- the guest cannot reset or initialize it;
- guest shutdown does not return it to `snd_hda_intel`;
- ALSA or PipeWire does not recover on the host;
- a test requires adding an unsafe ACS override or assigning the host's only
  usable display or storage controller.

The fallback is a host reboot into the previously working kernel. A VFIO test
is successful only when both the guest result and host recovery are recorded.
