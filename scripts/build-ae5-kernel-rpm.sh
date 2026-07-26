#!/usr/bin/env bash
set -euo pipefail

usage() {
	cat >&2 <<EOF
usage: $0 PATCHED_SOURCE_TREE BUILD_TREE [BASE_CONFIG] [LOCALVERSION]

Builds a side-by-side kernel RPM without installing it. BUILD_TREE must be
empty. BASE_CONFIG defaults to /boot/config-\$(uname -r), and LOCALVERSION
defaults to -ae5.
EOF
}

fail() {
	printf 'error: %s\n' "$1" >&2
	exit 1
}

need_tool() {
	command -v "$1" >/dev/null 2>&1 ||
		fail "required tool is unavailable: $1"
}

[[ $# -ge 2 && $# -le 4 ]] || {
	usage
	exit 2
}

for tool in find grep install make nproc readlink sha256sum; do
	need_tool "$tool"
done

source_tree=$(readlink -e -- "$1") ||
	fail "source tree does not exist: $1"
build_tree=$(readlink -m -- "$2")
base_config=$(readlink -e -- "${3:-"/boot/config-$(uname -r)"}") ||
	fail "base config does not exist: ${3:-"/boot/config-$(uname -r)"}"
localversion=${4:--ae5}

[[ -f $source_tree/Makefile &&
	-f $source_tree/sound/hda/codecs/ca0132.c &&
	-x $source_tree/scripts/config ]] ||
	fail "patched Linux source tree is incomplete: $source_tree"
[[ $build_tree != / && $build_tree != "$source_tree" ]] ||
	fail "unsafe build-tree path: $build_tree"
[[ $localversion =~ ^-[A-Za-z0-9._-]+$ ]] ||
	fail "LOCALVERSION must begin with '-' and contain safe filename characters"
if [[ -f $source_tree/.config || -d $source_tree/include/config ]] ||
	find "$source_tree/arch" -path '*/include/generated' -type d \
		-print -quit | grep -q .; then
	fail "source tree contains in-tree build state; use a disposable tree and run 'make -C $source_tree mrproper'"
fi
if [[ -d $build_tree ]] &&
	find "$build_tree" -mindepth 1 -maxdepth 1 -print -quit |
	grep -q .; then
	fail "build tree is not empty: $build_tree"
fi

for marker in 'ca0132_dsp_image_validate' \
	'AE-5: Direct Mode Playback Switch' \
	'AE5_INTERNAL_LED_COUNT' 'ca0132_alt_svm_restore'; do
	grep -Fq -- "$marker" "$source_tree/sound/hda/codecs/ca0132.c" ||
		fail "patched source marker is missing: $marker"
done

mkdir -p -- "$build_tree"
install -m0644 -- "$base_config" "$build_tree/.config"
"$source_tree/scripts/config" --file "$build_tree/.config" \
	--set-str LOCALVERSION "$localversion" \
	--disable LOCALVERSION_AUTO \
	--disable WERROR \
	--enable MODULE_SIG \
	--enable MODULE_SIG_ALL \
	--enable DEBUG_INFO_NONE \
	--disable DEBUG_INFO_DWARF5 \
	--disable DEBUG_INFO_BTF \
	--disable RUST

jobs=${AE5_KERNEL_BUILD_JOBS:-$(nproc)}
[[ $jobs =~ ^[1-9][0-9]*$ ]] ||
	fail "AE5_KERNEL_BUILD_JOBS must be a positive integer"

make -C "$source_tree" O="$build_tree" olddefconfig
make -C "$source_tree" O="$build_tree" W=1 KCFLAGS=-Werror \
	sound/hda/codecs/ca0132.o
make -C "$source_tree" O="$build_tree" -j"$jobs" \
	RPMOPTS=--nodeps binrpm-pkg

mapfile -t kernel_rpms < <(
	find "$build_tree/rpmbuild/RPMS" -type f \
		-name 'kernel-*.rpm' ! -name 'kernel-headers-*' \
		! -name 'kernel-devel-*' -print | sort
)
(( ${#kernel_rpms[@]} > 0 )) ||
	fail "kernel build produced no installation-candidate RPM"

printf 'source=%s\n' "$source_tree"
printf 'base_config=%s\n' "$base_config"
printf 'base_config_sha256=%s\n' \
	"$(sha256sum "$base_config" | awk '{ print $1 }')"
printf 'build_config_sha256=%s\n' \
	"$(sha256sum "$build_tree/.config" | awk '{ print $1 }')"
printf 'build_tree=%s\n' "$build_tree"
for kernel_rpm in "${kernel_rpms[@]}"; do
	printf 'kernel_rpm=%s\n' "$kernel_rpm"
done
printf 'install_performed=no\n'
