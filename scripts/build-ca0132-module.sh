#!/usr/bin/env bash
set -euo pipefail

usage() {
	printf 'usage: %s PATCHED_SOURCE_TREE [KERNEL_RELEASE]\n' "$0" >&2
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

for tool in grep make modinfo readlink sha256sum; do
	need_tool "$tool"
done

source_tree=$(readlink -e -- "$1") ||
	fail "source tree does not exist: $1"
kernel_release=${2:-$(uname -r)}
kernel_build=$(readlink -e -- "/lib/modules/$kernel_release/build") ||
	fail "matching kernel-devel tree is unavailable for $kernel_release"
codec_dir=$source_tree/sound/hda/codecs
source_file=$codec_dir/ca0132.c
module_path=$codec_dir/snd-hda-codec-ca0132.ko

[[ -f $source_file && -f $codec_dir/Makefile ]] ||
	fail "patched CA0132 source is incomplete: $codec_dir"
[[ -f $kernel_build/Module.symvers && -f $kernel_build/.config ]] ||
	fail "kernel-devel tree is incomplete: $kernel_build"
grep -qxF 'CONFIG_SND_HDA_CODEC_CA0132=m' "$kernel_build/.config" ||
	fail "target kernel does not build CA0132 as a module"

required_markers=(
	'ca0132_dsp_image_validate'
	'AE-5: Direct Mode Playback Switch'
	'AE5_INTERNAL_LED_COUNT'
	'ca0132_alt_svm_restore'
	'Wedge Angle defaults to 30 degrees.'
)
for marker in "${required_markers[@]}"; do
	grep -Fq -- "$marker" "$source_file" ||
		fail "patched source marker is missing: $marker"
done

make -C "$kernel_build" M="$codec_dir" W=1 KCFLAGS=-Werror \
	snd-hda-codec-ca0132.ko
[[ -s $module_path ]] || fail "module build produced no $module_path"

vermagic=$(modinfo -F vermagic "$module_path")
[[ $vermagic == "$kernel_release "* ]] ||
	fail "module vermagic does not match $kernel_release: $vermagic"
signer=$(modinfo -F signer "$module_path")
source_sha256=$(sha256sum "$source_file" | awk '{ print $1 }')
module_sha256=$(sha256sum "$module_path" | awk '{ print $1 }')

printf 'kernel_release=%s\n' "$kernel_release"
printf 'kernel_build=%s\n' "$kernel_build"
printf 'source_sha256=%s\n' "$source_sha256"
printf 'module=%s\n' "$module_path"
printf 'module_sha256=%s\n' "$module_sha256"
printf 'module_vermagic=%s\n' "$vermagic"
printf 'module_signed=%s\n' "$([[ -n $signer ]] && printf yes || printf no)"
printf 'module_signer=%s\n' "${signer:-none}"
printf 'warnings_as_errors=passed\n'
printf 'install_performed=no\n'
printf 'module_loaded=no\n'
