#!/usr/bin/env bash
set -euo pipefail

usage() {
	printf 'usage: %s EXPECTED_RELEASE | --self-test\n' "$0" >&2
	exit 2
}

fail() {
	printf 'error: %s\n' "$1" >&2
	exit 1
}

snapshot_value() {
	local key=$1 snapshot=$2

	awk -F= -v key="$key" '
		$1 == key {
			count++
			value = substr($0, length(key) + 2)
		}
		END {
			if (count != 1)
				exit 1
			print value
		}
	' <<< "$snapshot"
}

check_runtime() {
	local expected_release=$1 current_release taint snapshot snapshot_kernel
	local card_index card_root name expected actual driver_path
	local vermagic signer filename
	local direct_state led_root index multi_index
	local -a led_matches

	[[ $expected_release =~ ^[A-Za-z0-9][A-Za-z0-9._+-]*$ &&
		${#expected_release} -le 128 ]] ||
		fail 'expected release contains unsafe characters'

	current_release=$(uname -r)
	[[ $current_release == "$expected_release" ]] ||
		fail "running kernel is $current_release, expected $expected_release"

	[[ -r "$proc_root/sys/kernel/tainted" ]] ||
		fail 'kernel taint state is unreadable'
	read -r taint < "$proc_root/sys/kernel/tainted"
	[[ $taint == 0 ]] || fail "kernel taint is $taint, expected 0"

	if [[ -n ${AE5_RUNTIME_SNAPSHOT:-} ]]; then
		[[ -r $AE5_RUNTIME_SNAPSHOT ]] ||
			fail 'runtime snapshot fixture is unreadable'
		snapshot=$(<"$AE5_RUNTIME_SNAPSHOT")
	else
		snapshot=$(bash "$routing_probe" --preflight kernel-runtime)
	fi
	snapshot_kernel=$(snapshot_value kernel "$snapshot") ||
		fail 'routing preflight did not report one kernel'
	[[ $snapshot_kernel == "$expected_release" ]] ||
		fail "routing preflight used kernel $snapshot_kernel"
	card_index=$(snapshot_value alsa_card "$snapshot") ||
		fail 'routing preflight did not report one ALSA card'
	[[ $card_index =~ ^[0-9]+$ ]] ||
		fail "invalid ALSA card index: $card_index"

	card_root="$sys_root/class/sound/card$card_index/device"
	[[ -d $card_root ]] || fail "ALSA card$card_index sysfs device is missing"
	for attribute in \
		vendor:0x1102 \
		device:0x0012 \
		subsystem_vendor:0x1102 \
		subsystem_device:0x0051; do
		IFS=: read -r name expected <<< "$attribute"
		[[ -r "$card_root/$name" ]] ||
			fail "PCI attribute is unreadable: $name"
		read -r actual < "$card_root/$name"
		[[ ${actual,,} == "$expected" ]] ||
			fail "PCI $name is $actual, expected $expected"
	done
	driver_path=$(readlink -f "$card_root/driver") ||
		fail 'PCI driver link is unreadable'
	[[ ${driver_path##*/} == snd_hda_intel ]] ||
		fail "AE-5 driver is ${driver_path##*/}, expected snd_hda_intel"

	[[ -d "$sys_root/module/snd_hda_codec_ca0132" ]] ||
		fail 'CA0132 codec module is not loaded'
	vermagic=$(modinfo -k "$expected_release" -F vermagic \
		snd-hda-codec-ca0132) ||
		fail 'unable to read CA0132 module vermagic'
	[[ ${vermagic%% *} == "$expected_release" ]] ||
		fail "CA0132 vermagic is $vermagic"
	signer=$(modinfo -k "$expected_release" -F signer \
		snd-hda-codec-ca0132) ||
		fail 'unable to read CA0132 module signer'
	[[ -n $signer ]] || fail 'CA0132 module signer is empty'
	filename=$(modinfo -k "$expected_release" -F filename \
		snd-hda-codec-ca0132) ||
		fail 'unable to locate the CA0132 module'
	[[ $filename == /lib/modules/"$expected_release"/* ]] ||
		fail "CA0132 module came from an unexpected path: $filename"

	direct_state=$(LC_ALL=C amixer -c "$card_index" \
		sget 'AE-5: Direct Mode' 2>/dev/null) ||
		fail 'AE-5 Direct Mode control is missing'
	grep -Fq "Simple mixer control 'AE-5: Direct Mode',0" \
		<<< "$direct_state" ||
		fail 'AE-5 Direct Mode control is missing'
	grep -Eq 'Playback.*\[off\]' <<< "$direct_state" ||
		fail 'AE-5 Direct Mode must be off before the first physical test'

	led_root="$sys_root/class/leds"
	shopt -s nullglob
	led_matches=("$led_root"/hdaudioC"${card_index}"D*:rgb:ae5-*)
	shopt -u nullglob
	(( ${#led_matches[@]} == 5 )) ||
		fail "found ${#led_matches[@]} AE-5 onboard LEDs, expected 5"
	for index in 1 2 3 4 5; do
		shopt -s nullglob
		led_matches=(
			"$led_root"/hdaudioC"${card_index}"D*:rgb:ae5-"$index"
		)
		shopt -u nullglob
		(( ${#led_matches[@]} == 1 )) ||
			fail "AE-5 onboard LED $index is missing or ambiguous"
		for name in brightness multi_index multi_intensity; do
			[[ -r "${led_matches[0]}/$name" ]] ||
				fail "AE-5 onboard LED $index lacks $name"
		done
		read -r multi_index < "${led_matches[0]}/multi_index"
		[[ $multi_index == 'red green blue' ]] ||
			fail "AE-5 onboard LED $index has unexpected channels: $multi_index"
	done

	printf 'runtime_kernel=%s\n' "$current_release"
	printf 'kernel_taint=%s\n' "$taint"
	printf 'alsa_card=%s\n' "$card_index"
	printf 'pci_driver=snd_hda_intel\n'
	printf 'ca0132_vermagic=%s\n' "$vermagic"
	printf 'ca0132_signer=%s\n' "$signer"
	printf 'direct_mode=off\n'
	printf 'onboard_leds=5\n'
	printf 'routing_preflight=pass\n'
	printf 'runtime_result=pass\n'
}

self_test() (
	local test_root test_release=test-kernel snapshot_file device_root
	local index

	test_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-runtime-test.XXXXXX")
	trap 'find "$test_root" -depth -delete' EXIT
	proc_root=$test_root/proc
	sys_root=$test_root/sys
	routing_probe=$test_root/unused
	snapshot_file=$test_root/snapshot
	device_root=$sys_root/class/sound/card7/device

	install -d -m0755 \
		"$proc_root/sys/kernel" \
		"$device_root" \
		"$sys_root/bus/pci/drivers/snd_hda_intel" \
		"$sys_root/module/snd_hda_codec_ca0132" \
		"$sys_root/class/leds"
	printf '0\n' > "$proc_root/sys/kernel/tainted"
	printf '0x1102\n' > "$device_root/vendor"
	printf '0x0012\n' > "$device_root/device"
	printf '0x1102\n' > "$device_root/subsystem_vendor"
	printf '0x0051\n' > "$device_root/subsystem_device"
	ln -s "$sys_root/bus/pci/drivers/snd_hda_intel" \
		"$device_root/driver"
	printf '# AE-5 routing state\nkernel=%s\nalsa_card=7\n' \
		"$test_release" > "$snapshot_file"
	for index in 1 2 3 4 5; do
		install -d -m0755 \
			"$sys_root/class/leds/hdaudioC7D1:rgb:ae5-$index"
		install -m0644 /dev/null \
			"$sys_root/class/leds/hdaudioC7D1:rgb:ae5-$index/brightness"
		install -m0644 /dev/null \
			"$sys_root/class/leds/hdaudioC7D1:rgb:ae5-$index/multi_intensity"
		printf 'red green blue\n' > \
			"$sys_root/class/leds/hdaudioC7D1:rgb:ae5-$index/multi_index"
	done

	uname() {
		[[ $1 == -r ]] || return 1
		printf '%s\n' "$test_release"
	}
	modinfo() {
		[[ $1 == -k && $2 == "$test_release" && $3 == -F ]] ||
			return 1
		case $4 in
		vermagic)
			printf '%s SMP preempt mod_unload\n' "$test_release"
			;;
		signer)
			printf 'Test kernel key\n'
			;;
		filename)
			printf '/lib/modules/%s/kernel/sound/pci/hda/snd-hda-codec-ca0132.ko.xz\n' \
				"$test_release"
			;;
		*)
			return 1
			;;
		esac
	}
	amixer() {
		printf "Simple mixer control 'AE-5: Direct Mode',0\n"
		printf '  Mono: Playback [off]\n'
	}

	AE5_RUNTIME_SNAPSHOT=$snapshot_file \
		check_runtime "$test_release" >/dev/null

	printf '1\n' > "$proc_root/sys/kernel/tainted"
	if (
		AE5_RUNTIME_SNAPSHOT=$snapshot_file \
			check_runtime "$test_release"
	) >/dev/null 2>&1; then
		printf 'self-test failed: tainted kernel was accepted\n' >&2
		return 1
	fi
	printf '0\n' > "$proc_root/sys/kernel/tainted"
	find "$sys_root/class/leds/hdaudioC7D1:rgb:ae5-5" -depth -delete
	if (
		AE5_RUNTIME_SNAPSHOT=$snapshot_file \
			check_runtime "$test_release"
	) >/dev/null 2>&1; then
		printf 'self-test failed: missing onboard LED was accepted\n' >&2
		return 1
	fi

	printf 'AE-5 kernel runtime self-test passed\n'
)

proc_root=${AE5_RUNTIME_PROC_ROOT:-/proc}
sys_root=${AE5_RUNTIME_SYS_ROOT:-/sys}
script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
routing_probe=${AE5_RUNTIME_ROUTING_PROBE:-"$script_root/collect-routing-state.sh"}

case ${1:-} in
--self-test)
	[[ $# -eq 1 ]] || usage
	self_test
	;;
'')
	usage
	;;
*)
	[[ $# -eq 1 ]] || usage
	check_runtime "$1"
	;;
esac
