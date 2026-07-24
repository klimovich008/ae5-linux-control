#!/usr/bin/env bash
set -uo pipefail

sysfs_root=${AE5_VFIO_SYSFS_ROOT:-/sys}
dev_root=${AE5_VFIO_DEV_ROOT:-/dev}
require_tools=false
failures=0
ae5_path=
ae5_bdf=

usage() {
	printf 'usage: %s [--require-tools|--self-test]\n' "$0" >&2
}

fail() {
	printf 'error=%s\n' "$1"
	failures=$((failures + 1))
}

read_value() {
	local path=$1 value

	[[ -r $path ]] || return 1
	read -r value < "$path" || return 1
	printf '%s\n' "${value,,}"
}

discover_ae5() {
	local path vendor device subsystem_vendor subsystem_device
	local -a matches=()

	shopt -s nullglob
	for path in "$sysfs_root"/bus/pci/devices/*; do
		vendor=$(read_value "$path/vendor") || continue
		device=$(read_value "$path/device") || continue
		subsystem_vendor=$(read_value "$path/subsystem_vendor") || continue
		subsystem_device=$(read_value "$path/subsystem_device") || continue
		if [[ $vendor == 0x1102 && $device == 0x0012 &&
			$subsystem_vendor == 0x1102 && $subsystem_device == 0x0051 ]]; then
			matches+=("$path")
		fi
	done

	if (( ${#matches[@]} != 1 )); then
		fail "expected exactly one AE-5 1102:0012/1102:0051, found ${#matches[@]}"
		return 1
	fi

	ae5_path=${matches[0]}
	ae5_bdf=${ae5_path##*/}
	printf 'device=%s\n' "$ae5_bdf"
}

check_kvm() {
	local kvm=$dev_root/kvm

	if [[ ! -e $kvm ]]; then
		fail 'KVM device is unavailable'
	elif [[ ! -r $kvm || ! -w $kvm ]]; then
		fail 'current user cannot access the KVM device'
	else
		printf 'kvm=ready\n'
	fi
}

check_group() {
	local group_path member
	local -a members=()

	if [[ ! -L $ae5_path/iommu_group ]]; then
		fail 'AE-5 has no IOMMU group'
		return
	fi
	group_path=$(readlink -f -- "$ae5_path/iommu_group") || {
		fail 'cannot resolve AE-5 IOMMU group'
		return
	}
	shopt -s nullglob
	for member in "$group_path"/devices/*; do
		members+=("${member##*/}")
	done

	printf 'iommu_group=%s\n' "${group_path##*/}"
	printf 'iommu_members=%s\n' "${members[*]:-none}"
	if (( ${#members[@]} != 1 )) || [[ ${members[0]:-} != "$ae5_bdf" ]]; then
		fail 'AE-5 IOMMU group is not isolated'
	fi
}

check_driver() {
	local driver=unbound

	if [[ -L $ae5_path/driver ]]; then
		driver=$(basename -- "$(readlink -f -- "$ae5_path/driver")")
	fi
	printf 'host_driver=%s\n' "$driver"
	if [[ $driver != snd_hda_intel ]]; then
		fail 'AE-5 is not in the expected recovered host-driver state'
	fi
}

check_reset() {
	local methods normalized

	if [[ ! -e $ae5_path/reset ]]; then
		fail 'AE-5 exposes no PCI reset attribute'
		return
	fi
	methods=$(read_value "$ae5_path/reset_method") || {
		fail 'AE-5 reset method is unavailable'
		return
	}
	normalized=${methods//\[/}
	normalized=${normalized//\]/}
	printf 'reset_methods=%s\n' "$methods"
	if [[ " $normalized " != *' bus '* ]]; then
		fail 'AE-5 does not advertise the audited PCI bus reset method'
	fi
}

check_tools() {
	local tool
	local -a missing=()

	for tool in virsh virt-install qemu-img; do
		command -v "$tool" >/dev/null 2>&1 || missing+=("$tool")
	done
	if (( ${#missing[@]} )); then
		printf 'vm_tools=missing:%s\n' "${missing[*]}"
		$require_tools && fail 'required VM tools are not installed'
	else
		printf 'vm_tools=ready\n'
	fi
}

run_checks() {
	failures=0
	printf '# AE-5 VFIO host preflight\n'
	check_kvm
	if discover_ae5; then
		check_group
		check_driver
		check_reset
	fi
	check_tools

	if (( failures )); then
		printf 'vfio_preflight=blocked\n'
		return 1
	fi
	printf 'vfio_preflight=ready\n'
}

self_test() (
	local test_root test_device test_group fake_bin output

	test_root=$(mktemp -d)
	trap 'rm -rf -- "$test_root"' EXIT
	test_device=$test_root/sys/bus/pci/devices/0000:29:00.0
	test_group=$test_root/sys/kernel/iommu_groups/28
	fake_bin=$test_root/bin
	mkdir -p -- "$test_device" "$test_group/devices" \
		"$test_root/sys/bus/pci/drivers/snd_hda_intel" "$test_root/dev" "$fake_bin"
	printf '0x1102\n' > "$test_device/vendor"
	printf '0x0012\n' > "$test_device/device"
	printf '0x1102\n' > "$test_device/subsystem_vendor"
	printf '0x0051\n' > "$test_device/subsystem_device"
	printf 'bus\n' > "$test_device/reset_method"
	: > "$test_device/reset"
	: > "$test_root/dev/kvm"
	ln -s -- "$test_group" "$test_device/iommu_group"
	ln -s -- "$test_device" "$test_group/devices/0000:29:00.0"
	ln -s -- "$test_root/sys/bus/pci/drivers/snd_hda_intel" "$test_device/driver"
	for tool in virsh virt-install qemu-img; do
		printf '#!/usr/bin/env sh\nexit 0\n' > "$fake_bin/$tool"
		chmod +x -- "$fake_bin/$tool"
	done

	sysfs_root=$test_root/sys
	dev_root=$test_root/dev
	PATH=$fake_bin:$PATH
	require_tools=true
	output=$(run_checks) || {
		printf '%s\n' "$output" >&2
		return 1
	}
	grep -q '^device=0000:29:00.0$' <<< "$output"
	grep -q '^iommu_members=0000:29:00.0$' <<< "$output"
	grep -q '^host_driver=snd_hda_intel$' <<< "$output"
	grep -q '^vfio_preflight=ready$' <<< "$output"

	ln -s -- "$test_device" "$test_group/devices/0000:2a:00.0"
	if output=$(run_checks); then
		printf 'self-test failed: shared group was accepted\n' >&2
		return 1
	fi
	grep -q 'error=AE-5 IOMMU group is not isolated' <<< "$output"
	grep -q '^vfio_preflight=blocked$' <<< "$output"
	printf 'VFIO host preflight self-test passed\n'
)

case ${1:-} in
'')
	run_checks
	;;
--require-tools)
	[[ $# -eq 1 ]] || {
		usage
		exit 2
	}
	require_tools=true
	run_checks
	;;
--self-test)
	[[ $# -eq 1 ]] || {
		usage
		exit 2
	}
	self_test
	;;
*)
	usage
	exit 2
	;;
esac
