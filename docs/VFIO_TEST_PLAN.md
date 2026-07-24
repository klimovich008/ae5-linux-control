# AE-5 VFIO kernel A/B test plan

PCI passthrough is feasible on the current development host, but it is an
optional test environment rather than part of the application. It can compare
two Linux kernels against the same physical AE-5 without repeatedly changing
the host kernel. It cannot replace final cold-boot, suspend, and recovery tests
on the host.

The virtualization stack and a guest without passthrough were installed on
2026-07-24. No AE-5 host-device configuration has been attached or started.

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
2. The `ae5-kernel-test-f44` Fedora 44 guest is running without passthrough,
   and its clean `clean-fedora44` snapshot is available.
3. The Fifine microphone remains available to the host. Its acoustic test path
   and the AE-5's restored host playback path have already been validated.
4. The next hardware step is to add `0000:29:00.0` as a managed PCI host
   device. Libvirt's
   [`hostdev` documentation](https://www.libvirt.org/formatdomain.html#host-device-assignment)
   describes `managed='yes'`, which detaches the function from the host before
   guest startup and reattaches it after guest shutdown.
5. Do not make persistent early-boot VFIO binding the first experiment.
   Managed, on-demand assignment keeps recovery simpler and limits the period
   in which the host loses the card.

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
detachment. The first physical-card test must therefore use
`qemu:///system`. Its socket units are installed and enabled but were inactive
at the end of this audit because they were installed after the current host
boot.

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

No host device is present in the domain XML. During this smoke test the
physical AE-5 remained in IOMMU group 28 and bound to the host
`snd_hda_intel` driver. Physical validation of the corrected `30`-degree
control default remains gated on the reviewed `qemu:///system` passthrough
transition.

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

This is a build, boot, and no-device module test. It does not prove DSP
firmware loading or the four fixes on the physical AE-5.

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

After the coordinated host reboot:

1. confirm `virtqemud.socket` is active and `qemu:///system` connects;
2. upload the integrated standalone image into a system-owned libvirt storage
   pool rather than granting the system QEMU account access through the
   user's home;
3. define an inactive system domain with ordinary UEFI, no emulated audio, and
   no host device;
4. inspect its complete inactive XML, storage, NVRAM, and network settings
   without starting it;
5. retain the session guest and its snapshots as the recovery source until the
   system copy has passed a no-device boot.

### Reviewed host-device fragment

The only planned passthrough device is the already-audited AE-5 function:

```xml
<hostdev mode='subsystem' type='pci' managed='yes'>
  <source>
    <address domain='0x0000' bus='0x29' slot='0x00' function='0x0'/>
  </source>
</hostdev>
```

This fragment has not been attached to any domain. Before it is added:

1. rerun `scripts/check-vfio-host.sh --require-tools`;
2. confirm `0000:29:00.0` still resolves to `1102:0012/1102:0051`, is alone
   in its IOMMU group, and is bound to `snd_hda_intel`;
3. shut down every process using the AE-5 and confirm the Fifine path is
   available;
4. save ALSA, PipeWire, and PCI-driver state for recovery;
5. define the system guest while it is off, inspect its complete inactive XML,
   and only then schedule the first detach/start.

No persistent early-boot VFIO binding, ACS override, host-driver unbind, reset,
or guest start is authorized by this reviewed fragment.

## Per-kernel test matrix

Run the same sequence first on a known-good kernel and then on one candidate
kernel:

1. Start the guest and verify `1102:0012/1102:0051` with `lspci -nnk`.
2. Confirm the CA0132 DSP and ALSA controls initialize without new kernel
   warnings.
3. Select Speakers, Line Out, and Headphones in turn; verify the codec route,
   `Output Select`, Front switch, and audible output.
4. For Headphones, play a low-level 997 Hz fixture. Record a short baseline,
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
