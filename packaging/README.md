# Nobara/Fedora RPM

The RPM is built from a temporary source archive containing the locked Cargo
dependencies. The repository itself does not vendor crates, and `rpmbuild`
runs Cargo in offline/frozen mode.

Install the build tools:

```sh
sudo dnf install rpm-build cargo rust alsa-lib-devel gtk4-devel \
  desktop-file-utils appstream systemd-udev
```

Build from the repository root:

```sh
bash scripts/build-rpm.sh
```

The binary RPM and source RPM are written to `dist/`. The binary package
contains the GTK application, CLI, desktop integration, the privacy-conscious
`ae5-collect-report` diagnostics command, and a card-scoped PipeWire ACP
profile that prevents the generic headphone route from muting the AE-5's
shared Front DAC. The same profile exposes exact Microphone, Front Microphone,
and Line In routes for the card's `Input Source` enum. It also installs the
exact onboard-LED udev rule and hidden desktop autostart entry used to restore
saved colors. Install it with:

```sh
sudo dnf install ./dist/ae5-control-0.1.0-1.*.x86_64.rpm
```

The RPM license expression accounts for the statically linked Rust dependency
set. Those crates can be distributed under MIT terms, with `unicode-ident`
additionally requiring the Unicode-3.0 license shipped in the package.

Normal use does not require root, a project daemon, or a setuid helper.
WirePlumber reads the packaged profile on its next start; log out and back in,
or restart the user WirePlumber service when no audio stream is active.

## Per-user source installation

If the account cannot perform a system package transaction, install a local
release build into the standard XDG user directories:

```sh
bash scripts/install-user.sh
```

The installer builds with Cargo, copies a private payload under
`~/.local/share/ae5-control/user-install`, and creates only the required
per-user binary, self-contained uninstaller, desktop, AppStream, icon,
autostart, WirePlumber, and ACP links. Existing byte-identical routing files
are retained, any conflicting path aborts the operation before installation,
and missing system ACP includes are rejected instead of producing dangling
links. It does not restart WirePlumber automatically.

Remove only installer-owned integration and payload files with:

```sh
ae5-control-user-install --uninstall
```

Native profiles and `lighting.json` are deliberately preserved. The user
installation cannot install the kernel RGB patch or a udev rule; use the RPM
when those system pieces are required. `scripts/check-user-install.sh`
validates an isolated install, idempotent reinstall, metadata and command
execution, dependency/conflict refusal, invalid-marker refusal before unlink,
checkout-independent removal, and state preservation.

On a kernel containing the project's onboard-RGB patch, the package rule
matches only the original AE-5 `1102:0012/1102:0051`, all five exact
`hdaudioC*D*:rgb:ae5-[1-5]` names, and the `red green blue` channel order. It
makes only each LED's `brightness` and `multi_intensity` values writable; no
PCI resource or unrestricted MMIO is exposed. Final package removal returns
those attributes to mode `0644` and preserves the user's
`~/.config/ae5-control/lighting.json`. Uninstall with
`sudo dnf remove ae5-control`.

Pull-request CI and pushes to `main` build the package in Fedora 44, install
the resulting binary RPM into that clean container, verify its files and
commands, remove it, and confirm that package removal preserves both user
profiles and ALSA state. The lifecycle verifier is intentionally restricted
to a disposable container because it creates test state and performs a real
package transaction. Inside that container, run:

```sh
bash scripts/check-rpm-lifecycle.sh \
  dist/ae5-control-0.1.0-1.*.x86_64.rpm
```

The reproducible build, automated clean Fedora 44 install/removal gate, and
normal-user installation, lighting, rollback, and removal of an exact package
on the target AE-5 are
recorded in
[`docs/PACKAGING_VALIDATION.md`](../docs/PACKAGING_VALIDATION.md). An
installed per-user desktop-menu launch has passed on the host; an authenticated
host RPM install/upgrade/remove cycle remains part of the final release gate.
