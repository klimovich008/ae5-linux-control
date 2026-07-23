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

collect() {
	local card card_index vendor codec found=0

	printf '# AE-5 Linux hardware report\n'
	printf 'generated_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"

	section 'Operating system'
	if [[ -r /etc/os-release ]]; then
		(
			. /etc/os-release
			printf 'distribution=%s\nversion=%s\n' \
				"${NAME:-unknown}" "${VERSION_ID:-unknown}"
		)
	fi
	printf 'kernel=%s\n' "$(uname -srmv)"

	run 'Creative PCI devices' lspci -nnk -d 1102:
	run 'ALSA cards' sh -c 'cat /proc/asound/cards'
	run 'Playback devices' aplay -l
	run 'Capture devices' arecord -l
	run 'PipeWire status' wpctl status --name
	run 'CA0132 module information' modinfo snd_hda_codec_ca0132
	run 'Loaded sound modules' sh -c \
		'lsmod | grep -E "^(snd|soundcore)" || true'
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
	local test_dir report
	test_dir=$(mktemp -d)
	trap 'rm -rf -- "$test_dir"' RETURN
	report=$test_dir/report.txt
	collect > "$report"
	grep -q '^# AE-5 Linux hardware report$' "$report"
	grep -q '^## Operating system$' "$report"
	grep -q '^## Creative PCI devices$' "$report"
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
