Name:           ae5-control
Version:        0.1.0
Release:        1%{?dist}
Summary:        Linux control software for the Creative Sound BlasterX AE-5

License:        MIT AND Unicode-3.0
URL:            https://github.com/klimovich008/ae5-linux-control
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  appstream
BuildRequires:  cargo
BuildRequires:  desktop-file-utils
BuildRequires:  pkgconfig(alsa)
BuildRequires:  pkgconfig(gtk4) >= 4.10
BuildRequires:  rust
Requires:       hicolor-icon-theme

ExclusiveArch:  %{rust_arches}

%description
AE-5 Control provides native GTK and command-line interfaces for the verified
ALSA controls exposed by Linux for the Creative Sound BlasterX AE-5. It
supports hardware routing, DSP effects, equalizer settings, native profiles,
and conversion of compatible Sound Blaster Command JSON profiles.

%prep
%autosetup

%build
export CARGO_NET_OFFLINE=true
export RUSTFLAGS="%{build_rustflags}"
cargo build --frozen --offline --release --all-features

%install
install -Dm0755 target/release/ae5-control \
  %{buildroot}%{_bindir}/ae5-control
install -Dm0755 target/release/ae5ctl \
  %{buildroot}%{_bindir}/ae5ctl
install -Dm0644 packaging/io.github.klimovich008.ae5control.desktop \
  %{buildroot}%{_datadir}/applications/io.github.klimovich008.ae5control.desktop
install -Dm0644 packaging/io.github.klimovich008.ae5control.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/io.github.klimovich008.ae5control.svg
install -Dm0644 packaging/io.github.klimovich008.ae5control.metainfo.xml \
  %{buildroot}%{_metainfodir}/io.github.klimovich008.ae5control.metainfo.xml

%check
export CARGO_NET_OFFLINE=true
cargo test --frozen --offline --release --all-features
desktop-file-validate packaging/io.github.klimovich008.ae5control.desktop
appstreamcli validate --no-net --strict \
  packaging/io.github.klimovich008.ae5control.metainfo.xml

%files
%license LICENSE-APACHE LICENSE-MIT vendor/unicode-ident/LICENSE-UNICODE
%doc README.md PORT_PLAN.md docs
%{_bindir}/ae5-control
%{_bindir}/ae5ctl
%{_datadir}/applications/io.github.klimovich008.ae5control.desktop
%{_datadir}/icons/hicolor/scalable/apps/io.github.klimovich008.ae5control.svg
%{_metainfodir}/io.github.klimovich008.ae5control.metainfo.xml

%changelog
* Fri Jul 24 2026 AE-5 Control contributors <klimovich008@users.noreply.github.com> - 0.1.0-1
- Initial hardware-tested package for the original AE-5.
