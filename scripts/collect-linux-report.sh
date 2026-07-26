#!/usr/bin/env bash
set -uo pipefail

section() {
	printf '\n## %s\n' "$1"
}

run() {
	local title=$1
	shift
	section "$title"
	if command -v "$1" >/dev/null 2>&1; then
		"$@" 2>&1 || printf '[command exited %d]\n' "$?"
	else
		printf '[%s unavailable]\n' "$1"
	fi
}

collect_pipewire() {
	local status

	section 'Creative PipeWire objects'
	if ! command -v wpctl >/dev/null 2>&1; then
		printf '[wpctl unavailable]\n'
		return
	fi
	if ! status=$(wpctl status --name 2>/dev/null); then
		printf '[unable to query PipeWire]\n'
		return
	fi
	if ! grep -Ei 'creative|sound blaster|ae-5' <<< "$status"; then
		printf '[no Creative PipeWire objects found]\n'
	fi
}

collect_lighting() {
	local led name channels brightness intensity mode
	local -a leds=()

	section 'AE-5 onboard lighting'
	shopt -s nullglob
	leds=("${AE5_LED_ROOT:-/sys/class/leds}"/hdaudioC*D*:rgb:ae5-[1-5])
	if (( ${#leds[@]} == 0 )); then
		printf '[no AE-5 onboard LED-class devices found]\n'
		return
	fi
	for led in "${leds[@]}"; do
		name=${led##*/}
		channels=$(<"$led/multi_index") || channels=unreadable
		brightness=$(<"$led/brightness") || brightness=unreadable
		intensity=$(<"$led/multi_intensity") || intensity=unreadable
		mode=$(stat -c '%a' "$led/brightness" "$led/multi_intensity" 2>/dev/null |
			paste -sd, -) || mode=unreadable
		printf '%s channels=%s intensity=%s brightness=%s modes=%s\n' \
			"$name" "$channels" "$intensity" "$brightness" "$mode"
	done
}

collect() {
	local card card_index vendor codec found=0

	printf '# AE-5 Linux hardware report\n'
	printf 'generated_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'privacy=non-Creative PipeWire objects, hostname, user, storage, and network data omitted\n'

	section 'Operating system'
	if [[ -r /etc/os-release ]]; then
		(
			. /etc/os-release
			printf 'distribution=%s\nversion=%s\n' \
				"${NAME:-unknown}" "${VERSION_ID:-unknown}"
		)
	fi
	printf 'kernel=%s\n' "$(uname -srmv)"
	if [[ -r /proc/sys/kernel/tainted ]]; then
		printf 'kernel_tainted=%s\n' "$(< /proc/sys/kernel/tainted)"
	else
		printf 'kernel_tainted=unavailable\n'
	fi

	run 'Creative PCI devices' lspci -nnk -d 1102:
	run 'ALSA cards' sh -c 'cat /proc/asound/cards'
	run 'Playback devices' aplay -l
	run 'Capture devices' arecord -l
	collect_pipewire
	run 'AE-5 Control route health' ae5ctl route-status
	collect_lighting
	run 'CA0132 module information' modinfo snd_hda_codec_ca0132
	run 'Loaded sound modules' sh -c \
		'lsmod | grep -E "^(snd|soundcore)" || true'
	run 'Failed system units' systemctl --failed --no-legend --plain
	run 'Failed user units' systemctl --user --failed --no-legend --plain
	run 'Relevant warning-level kernel log' sh -c \
		'journalctl -k -b -p warning..alert -o cat --no-pager 2>/dev/null |
		 grep -Ei "ca0132|sound blaster|snd_hda|hdaudio|firmware" ||
		 true'
	run 'Relevant kernel log' sh -c \
		'journalctl -k -b -o cat --no-pager 2>/dev/null |
		 grep -Ei "ca0132|sound blaster|snd_hda|hdaudio|firmware" ||
		 dmesg 2>/dev/null |
		 grep -Ei "ca0132|sound blaster|snd_hda|hdaudio|firmware" ||
		 true'

	shopt -s nullglob
	for card in /sys/class/sound/card[0-9]*; do
		[[ -r "$card/device/vendor" ]] || continue
		read -r vendor < "$card/device/vendor"
		[[ ${vendor,,} == 0x1102 ]] || continue

		found=1
		card_index=${card##*card}
		run "ALSA controls for card $card_index" \
			amixer -c "$card_index" controls
		run "ALSA values for card $card_index" \
			amixer -c "$card_index" contents

		for codec in "/proc/asound/card$card_index"/codec*; do
			[[ -r "$codec" ]] || continue
			section "Codec data: ${codec##*/}"
			cat "$codec"
		done
	done

	if (( ! found )); then
		section 'Creative ALSA controls'
		printf '[no ALSA card with PCI vendor 0x1102 found]\n'
	fi
}

self_test() {
	local test_dir report hostname_value
	test_dir=$(mktemp -d)
	trap 'rm -rf -- "$test_dir"' RETURN
	report=$test_dir/report.txt
	collect > "$report"
	grep -q '^# AE-5 Linux hardware report$' "$report"
	grep -q '^## Operating system$' "$report"
	grep -q '^## Creative PCI devices$' "$report"
	grep -q '^## Creative PipeWire objects$' "$report"
	grep -q '^## AE-5 Control route health$' "$report"
	grep -q '^## AE-5 onboard lighting$' "$report"
	grep -q '^kernel_tainted=' "$report"
	grep -q '^## Failed system units$' "$report"
	grep -q '^## Failed user units$' "$report"
	grep -q '^## Relevant warning-level kernel log$' "$report"
	! grep -Fq "$HOME" "$report"
	if [[ -r /proc/sys/kernel/hostname ]]; then
		read -r hostname_value < /proc/sys/kernel/hostname
		[[ -z $hostname_value ]] || ! grep -Fq "$hostname_value" "$report"
	fi
	printf 'self-test passed\n'
}

case ${1:-} in
--self-test)
	[[ $# -eq 1 ]] || {
		printf 'usage: %s [--self-test|output-file]\n' "$0" >&2
		exit 2
	}
	self_test
	;;
*)
	[[ $# -le 1 ]] || {
		printf 'usage: %s [--self-test|output-file]\n' "$0" >&2
		exit 2
	}
	umask 077
	output=${1:-ae5-report-$(date -u +%Y%m%d-%H%M%S).txt}
	[[ ! -e "$output" ]] || {
		printf 'refusing to overwrite %s\n' "$output" >&2
		exit 2
	}
	collect > "$output"
	printf 'report written to %s; review it before sharing\n' "$output"
	;;
esac
