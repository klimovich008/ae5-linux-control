# AE-5 VFIO kernel A/B test plan

PCI passthrough is feasible on the current development host, but it is an
optional test environment rather than part of the application. It can compare
two Linux kernels against the same physical AE-5 without repeatedly changing
the host kernel. It cannot replace final cold-boot, suspend, and recovery tests
on the host.

No passthrough configuration or virtualization package was installed while
preparing this plan.

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
- no `qemu-kvm`, libvirt, `virt-install`, OVMF, or `swtpm` package is currently
  installed.

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

## One-time setup, only after explicit approval

1. Install the distribution's QEMU/KVM, libvirt, `virt-install`, and UEFI
   firmware packages.
2. Create a Linux guest without passthrough, install the baseline kernel and
   candidate kernel, and take a clean snapshot.
3. Confirm that the host has an alternate playback device and that the Fifine
   microphone records correctly.
4. Add `0000:29:00.0` as a managed PCI host device. Libvirt's
   [`hostdev` documentation](https://www.libvirt.org/formatdomain.html#host-device-assignment)
   describes `managed='yes'`, which detaches the function from the host before
   guest startup and reattaches it after guest shutdown.
5. Do not make persistent early-boot VFIO binding the first experiment.
   Managed, on-demand assignment keeps recovery simpler and limits the period
   in which the host loses the card.

The exact package transaction and libvirt XML must be reviewed against the
installed distribution versions before execution. The PCI address is a host
address; it must not be copied into an unrelated machine's configuration.

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
