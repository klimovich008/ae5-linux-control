# Nobara/Fedora RPM

The RPM is built from a temporary source archive containing the locked Cargo
dependencies. The repository itself does not vendor crates, and `rpmbuild`
runs Cargo in offline/frozen mode.

Install the build tools:

```sh
sudo dnf install rpm-build cargo rust alsa-lib-devel gtk4-devel \
  desktop-file-utils appstream
```

Build from the repository root:

```sh
bash scripts/build-rpm.sh
```

The binary RPM and source RPM are written to `dist/`. The binary package
contains the GTK application, CLI, desktop integration, and the
privacy-conscious `ae5-collect-report` diagnostics command. Install it with:

```sh
sudo dnf install ./dist/ae5-control-0.1.0-1.*.x86_64.rpm
```

The RPM license expression accounts for the statically linked Rust dependency
set. Those crates can be distributed under MIT terms, with `unicode-ident`
additionally requiring the Unicode-3.0 license shipped in the package.

Normal use does not require root, a daemon, a setuid helper, or extra device
rules. Uninstall with `sudo dnf remove ae5-control`.

The reproducible build, clean Fedora 44 install/removal transaction, and
read-only execution of the exact package payload on the target AE-5 are
recorded in
[`docs/PACKAGING_VALIDATION.md`](../docs/PACKAGING_VALIDATION.md). An
authenticated host install and desktop-menu launch remain part of the final
release gate.
