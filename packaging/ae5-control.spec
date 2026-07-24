%undefine _debugsource_packages

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
Requires:       pipewire-libs
Requires:       wireplumber

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
export RUSTFLAGS="%{build_rustflags} --remap-path-prefix=$PWD=."
cargo build --frozen --offline --release --all-features

%install
install -Dm0755 target/release/ae5-control \
  %{buildroot}%{_bindir}/ae5-control
install -Dm0755 target/release/ae5ctl \
  %{buildroot}%{_bindir}/ae5ctl
install -Dm0755 scripts/collect-linux-report.sh \
  %{buildroot}%{_bindir}/ae5-collect-report
install -Dm0644 packaging/io.github.klimovich008.ae5control.desktop \
  %{buildroot}%{_datadir}/applications/io.github.klimovich008.ae5control.desktop
install -Dm0644 packaging/io.github.klimovich008.ae5control.svg \
  %{buildroot}%{_datadir}/icons/hicolor/scalable/apps/io.github.klimovich008.ae5control.svg
install -Dm0644 packaging/io.github.klimovich008.ae5control.metainfo.xml \
  %{buildroot}%{_metainfodir}/io.github.klimovich008.ae5control.metainfo.xml
install -Dm0644 \
  packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf \
  %{buildroot}%{_datadir}/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf
install -Dm0644 \
  packaging/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf \
  %{buildroot}%{_datadir}/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf
install -Dm0644 packaging/wireplumber/90-ae5-control.conf \
  %{buildroot}%{_datadir}/wireplumber/wireplumber.conf.d/90-ae5-control.conf

%check
export CARGO_NET_OFFLINE=true
export RUSTFLAGS="%{build_rustflags} --remap-path-prefix=$PWD=."
cargo test --frozen --offline --release --all-features
bash scripts/check-ae5-acp-profile.sh
bash scripts/collect-linux-report.sh --self-test
desktop-file-validate packaging/io.github.klimovich008.ae5control.desktop
appstreamcli validate --no-net --strict \
  packaging/io.github.klimovich008.ae5control.metainfo.xml
if grep -aF "$PWD" \
  %{buildroot}%{_bindir}/ae5-control \
  %{buildroot}%{_bindir}/ae5ctl >/dev/null; then
  echo "installed binaries contain the private RPM build path" >&2
  exit 1
fi

%files
%license LICENSE-APACHE LICENSE-MIT vendor/unicode-ident/LICENSE-UNICODE
%doc README.md PORT_PLAN.md docs
%{_bindir}/ae5-control
%{_bindir}/ae5-collect-report
%{_bindir}/ae5ctl
%{_datadir}/applications/io.github.klimovich008.ae5control.desktop
%{_datadir}/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf
%{_datadir}/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf
%{_datadir}/icons/hicolor/scalable/apps/io.github.klimovich008.ae5control.svg
%{_metainfodir}/io.github.klimovich008.ae5control.metainfo.xml
%{_datadir}/wireplumber/wireplumber.conf.d/90-ae5-control.conf

%changelog
* Fri Jul 24 2026 AE-5 Control contributors <klimovich008@users.noreply.github.com> - 0.1.0-1
- Initial hardware-tested package for the original AE-5.
