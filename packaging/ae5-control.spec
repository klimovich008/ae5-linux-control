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
BuildRequires:  systemd-udev
Requires:       hicolor-icon-theme
Requires:       pipewire-libs
Requires:       pipewire-utils
Requires:       systemd-udev
Requires:       wireplumber

ExclusiveArch:  %{rust_arches}

%description
AE-5 Control provides native GTK and command-line interfaces for the verified
ALSA controls exposed by Linux for the Creative Sound BlasterX AE-5. It
supports hardware routing, DSP effects, equalizer settings, native profiles,
onboard lighting through the kernel LED class, and conversion of compatible
Sound Blaster Command JSON profiles.

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
install -Dm0644 packaging/io.github.klimovich008.ae5control-lighting.desktop \
  %{buildroot}%{_sysconfdir}/xdg/autostart/io.github.klimovich008.ae5control-lighting.desktop
install -Dm0644 packaging/udev/70-ae5-control-leds.rules \
  %{buildroot}%{_udevrulesdir}/70-ae5-control-leds.rules
install -Dm0644 \
  packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf \
  %{buildroot}%{_datadir}/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf
for path in microphone front-microphone line-in; do
  install -Dm0644 \
    packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-${path}.conf \
    %{buildroot}%{_datadir}/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-${path}.conf
done
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
desktop-file-validate packaging/io.github.klimovich008.ae5control-lighting.desktop
appstreamcli validate --no-net --strict \
  packaging/io.github.klimovich008.ae5control.metainfo.xml
udevadm verify --resolve-names=never \
  packaging/udev/70-ae5-control-leds.rules
if grep -aF "$PWD" \
  %{buildroot}%{_bindir}/ae5-control \
  %{buildroot}%{_bindir}/ae5ctl >/dev/null; then
  echo "installed binaries contain the private RPM build path" >&2
  exit 1
fi

%post
udevadm control --reload-rules >/dev/null 2>&1 || :
udevadm trigger --action=add --subsystem-match=leds >/dev/null 2>&1 || :

%preun
if [ "$1" -eq 0 ]; then
  for led in /sys/class/leds/hdaudioC*D*:rgb:ae5-[1-5]; do
    [ -r "$led/multi_index" ] || continue
    [ "$(cat "$led/multi_index")" = "red green blue" ] || continue
    chmod 0644 "$led/brightness" "$led/multi_intensity" || :
  done
fi

%postun
udevadm control --reload-rules >/dev/null 2>&1 || :

%files
%license LICENSE-APACHE LICENSE-MIT vendor/unicode-ident/LICENSE-UNICODE
%doc README.md PORT_PLAN.md docs
%{_sysconfdir}/xdg/autostart/io.github.klimovich008.ae5control-lighting.desktop
%{_bindir}/ae5-control
%{_bindir}/ae5-collect-report
%{_bindir}/ae5ctl
%{_datadir}/applications/io.github.klimovich008.ae5control.desktop
%{_datadir}/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-front-microphone.conf
%{_datadir}/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-line-in.conf
%{_datadir}/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-microphone.conf
%{_datadir}/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf
%{_datadir}/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf
%{_datadir}/icons/hicolor/scalable/apps/io.github.klimovich008.ae5control.svg
%{_metainfodir}/io.github.klimovich008.ae5control.metainfo.xml
%{_datadir}/wireplumber/wireplumber.conf.d/90-ae5-control.conf
%{_udevrulesdir}/70-ae5-control-leds.rules

%changelog
* Fri Jul 24 2026 AE-5 Control contributors <klimovich008@users.noreply.github.com> - 0.1.0-1
- Initial hardware-tested package for the original AE-5.
