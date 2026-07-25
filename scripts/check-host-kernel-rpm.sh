#!/usr/bin/env bash
set -euo pipefail

usage() {
	printf 'usage: %s KERNEL_RPM [EXPECTED_RELEASE]\n' "$0" >&2
}

fail() {
	printf 'error: %s\n' "$1" >&2
	exit 1
}

need_tool() {
	command -v "$1" >/dev/null 2>&1 ||
		fail "required tool is unavailable: $1"
}

[[ $# -ge 1 && $# -le 2 ]] || {
	usage
	exit 2
}

for tool in cpio depmod file find grep modinfo mktemp readlink rpm rpm2cpio \
	sha256sum strings zstd; do
	need_tool "$tool"
done

rpm_path=$(readlink -f -- "$1")
expected_release=${2:-}
[[ -f $rpm_path ]] || fail "RPM does not exist: $1"

package_name=$(rpm -qp --qf '%{NAME}' "$rpm_path")
package_arch=$(rpm -qp --qf '%{ARCH}' "$rpm_path")
package_nevra=$(rpm -qp --qf '%{NAME}-%{VERSION}-%{RELEASE}.%{ARCH}' \
	"$rpm_path")
[[ $package_name == kernel ]] ||
	fail "expected the kernel package, found $package_name"
[[ $package_arch == x86_64 ]] ||
	fail "expected x86_64, found $package_arch"

conflicts=$(rpm -qp --conflicts "$rpm_path")
obsoletes=$(rpm -qp --obsoletes "$rpm_path")
[[ -z $conflicts ]] ||
	fail "kernel RPM declares conflicts: $conflicts"
[[ -z $obsoletes ]] ||
	fail "kernel RPM declares obsoletes: $obsoletes"

temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-kernel-rpm-check.XXXXXX")
trap 'rm -rf -- "$temporary_root"' EXIT

(
	cd -- "$temporary_root"
	rpm2cpio "$rpm_path" | cpio -idm --quiet
)

mapfile -t module_roots < <(
	find "$temporary_root/lib/modules" -mindepth 1 -maxdepth 1 \
		-type d -print
)
(( ${#module_roots[@]} == 1 )) ||
	fail "expected one module tree, found ${#module_roots[@]}"
module_root=${module_roots[0]}
release=${module_root##*/}
[[ -z $expected_release || $release == "$expected_release" ]] ||
	fail "expected release $expected_release, found $release"

for artifact in config System.map vmlinuz; do
	[[ -s $module_root/$artifact ]] ||
		fail "missing packaged $artifact"
done

required_config=(
	'CONFIG_EFI_STUB=y'
	'CONFIG_MODULE_SIG=y'
	'CONFIG_MODULE_SIG_ALL=y'
	'CONFIG_MODULE_COMPRESS_ZSTD=y'
	'CONFIG_MODULE_DECOMPRESS=y'
	'CONFIG_BLK_DEV_NVME=y'
	'CONFIG_BTRFS_FS=y'
	'CONFIG_EXT4_FS=y'
	'CONFIG_VFAT_FS=m'
	'CONFIG_R8169=m'
	'CONFIG_IWLWIFI=m'
	'CONFIG_IWLMVM=m'
	'CONFIG_DRM_AMDGPU=m'
	'CONFIG_SND_HDA_INTEL=m'
	'CONFIG_SND_HDA_CODEC_CA0132=m'
	'CONFIG_USB_XHCI_HCD=y'
	'CONFIG_USB_XHCI_PCI=y'
)
for setting in "${required_config[@]}"; do
	grep -qxF -- "$setting" "$module_root/config" ||
		fail "required kernel setting is missing: $setting"
done

image_description=$(file -b -- "$module_root/vmlinuz")
grep -Fq -- "version $release " <<< "$image_description" ||
	fail "kernel image does not report release $release"

mapfile -t ca0132_modules < <(
	find "$module_root/kernel/sound" -type f \
		-name 'snd-hda-codec-ca0132.ko*' -print
)
(( ${#ca0132_modules[@]} == 1 )) ||
	fail "expected one CA0132 module, found ${#ca0132_modules[@]}"
ca0132_module=${ca0132_modules[0]}
[[ $ca0132_module == *.ko.zst ]] ||
	fail "expected a zstd-compressed CA0132 module"

vermagic=$(modinfo -F vermagic "$ca0132_module")
signer=$(modinfo -F signer "$ca0132_module")
signature_id=$(modinfo -F sig_id "$ca0132_module")
[[ $vermagic == "$release "* ]] ||
	fail "CA0132 vermagic does not match $release: $vermagic"
[[ -n $signer && $signature_id == PKCS#7 ]] ||
	fail 'CA0132 module is not PKCS#7 signed'

zstd -d -q -f "$ca0132_module" -o "$temporary_root/ca0132.ko"
module_strings=$(strings -- "$temporary_root/ca0132.ko")
for marker in \
	'AE-5: Direct Mode Playback Switch' \
	'%s:rgb:ae5-%u' \
	'FX: Equalizer Preset Switch' \
	'Invalid DSP image'; do
	grep -Fq -- "$marker" <<< "$module_strings" ||
		fail "CA0132 feature marker is missing: $marker"
done

depmod -b "$temporary_root" "$release"
grep -Fq -- \
	'kernel/sound/hda/codecs/snd-hda-codec-ca0132.ko.zst:' \
	"$module_root/modules.dep" ||
	fail 'depmod did not index the CA0132 module'

install_scripts=$(rpm -qp --scripts "$rpm_path")
grep -Fq -- "kernel-install add $release " <<< "$install_scripts" ||
	fail 'RPM post-install script does not invoke kernel-install'
grep -Fq -- "/boot/\${file}-$release" <<< "$install_scripts" ||
	fail 'RPM post-install script does not name the versioned boot files'

read -r rpm_sha256 _ < <(sha256sum "$rpm_path")
module_count=$(find "$module_root/kernel" -type f -name '*.ko.zst' -print |
	wc -l)

printf 'package=%s\n' "$package_nevra"
printf 'release=%s\n' "$release"
printf 'rpm_sha256=%s\n' "$rpm_sha256"
printf 'module_count=%s\n' "$module_count"
printf 'ca0132_vermagic=%s\n' "$vermagic"
printf 'ca0132_signer=%s\n' "$signer"
printf 'install_performed=no\n'
printf 'verification=passed\n'
