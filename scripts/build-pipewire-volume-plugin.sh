#!/usr/bin/env bash
set -euo pipefail

need_tool() {
	command -v "$1" >/dev/null 2>&1 || {
		printf 'error: required tool is unavailable: %s\n' "$1" >&2
		exit 1
	}
}

for tool in cpio dnf gcc meson ninja patch rpm rpm2cpio tar; do
	need_tool "$tool"
done

script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root=$(dirname -- "$script_root")
patch_path=$project_root/pipewire/ae5-windows-volume-curve.patch
[[ -f $patch_path ]] || {
	printf 'error: PipeWire volume patch is missing: %s\n' "$patch_path" >&2
	exit 1
}

installed_version=$(rpm -q --qf '%{VERSION}' pipewire.x86_64)
installed_release=$(rpm -q --qf '%{RELEASE}' pipewire.x86_64)
output_root=${AE5_PIPEWIRE_PLUGIN_OUTPUT_ROOT:-"$project_root/dist/pipewire-$installed_version-ae5"}
[[ $output_root == /* && $output_root != / ]] || {
	printf 'error: output path must be absolute and narrower than /: %s\n' "$output_root" >&2
	exit 1
}
[[ ! -e $output_root ]] || {
	printf 'error: refusing to overwrite existing output: %s\n' "$output_root" >&2
	exit 1
}

temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-pipewire-build.XXXXXX")
cleanup() {
	find "$temporary_root" -depth -delete
}
trap cleanup EXIT

dnf download --source \
	"pipewire-$installed_version-$installed_release" \
	--destdir "$temporary_root"
source_rpm=$(find "$temporary_root" -maxdepth 1 -type f -name 'pipewire-*.src.rpm' -print -quit)
[[ -n $source_rpm ]] || {
	printf 'error: the installed PipeWire source RPM was not downloaded\n' >&2
	exit 1
}
[[ $(rpm -qp --qf '%{VERSION}' "$source_rpm") == "$installed_version" ]] || {
	printf 'error: downloaded PipeWire source does not match the installed version\n' >&2
	exit 1
}

source_payload=$temporary_root/source-payload
install -d -m0755 "$source_payload"
(
	cd -- "$source_payload"
	rpm2cpio "$source_rpm" | cpio -idm --quiet
)
source_archive=$(find "$source_payload" -maxdepth 1 -type f \
	-name "pipewire-$installed_version.tar.*" -print -quit)
[[ -n $source_archive ]] || {
	printf 'error: PipeWire source archive is missing from the source RPM\n' >&2
	exit 1
}
source_tree=$temporary_root/source
install -d -m0755 "$source_tree"
tar -xf "$source_archive" -C "$source_tree" --strip-components=1
patch --batch --forward -d "$source_tree" -p1 < "$patch_path"

meson setup "$temporary_root/build" "$source_tree" \
	-Ddocs=disabled -Dman=disabled -Dtests=enabled -Dinstalled_tests=disabled \
	-Dpipewire-alsa=disabled -Dpipewire-jack=disabled -Djack=disabled \
	-Dbluez5=disabled -Dlibcamera=disabled -Dv4l2=disabled \
	-Dpipewire-v4l2=disabled -Dgstreamer=disabled \
	-Dsystemd-system-service=disabled -Dsystemd-user-service=disabled \
	-Dlibsystemd=disabled -Dsdl2=disabled -Dsession-managers=[] \
	-Dexamples=disabled -Dpw-cat=disabled -Dreadline=disabled \
	-Ddbus=disabled -Dudev=disabled -Dlibpulse=disabled \
	-Davahi=disabled -Draop=disabled -Dx11=disabled \
	-Dlibmysofa=disabled -Dlv2=disabled -Droc=disabled \
	-Dlibffado=disabled -Dvulkan=disabled \
	-Decho-cancel-webrtc=disabled -Dlibusb=disabled \
	-Dcompress-offload=disabled -Donnxruntime=disabled

ninja -C "$temporary_root/build" \
	spa/plugins/audioconvert/libspa-audioconvert.so \
	spa/plugins/audioconvert/test-audioconvert \
	spa/plugins/audioconvert/test-channelmix \
	spa/plugins/audioconvert/test-windows-volume-curve
meson test -C "$temporary_root/build" \
	test-audioconvert test-channelmix test-windows-volume-curve \
	--print-errorlogs

install -d -m0755 "$output_root"
install -m0755 \
	"$temporary_root/build/spa/plugins/audioconvert/libspa-audioconvert.so" \
	"$output_root/libspa-audioconvert.so"
sha256sum "$output_root/libspa-audioconvert.so" |
	tee "$output_root/SHA256SUMS" >/dev/null

printf 'wrote tested PipeWire %s AE-5 volume plugin to %s\n' \
	"$installed_version" "$output_root"
