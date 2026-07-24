#!/usr/bin/env bash
set -uo pipefail

usage() {
	printf 'usage: %s [--self-test | [label] [--append]]\n' "$0" >&2
}

section() {
	printf '\n## %s\n' "$1"
}

find_ae5_card() {
	local card_path vendor device subsystem_vendor subsystem_device

	shopt -s nullglob
	for card_path in /sys/class/sound/card[0-9]*; do
		[[ -r "$card_path/device/vendor" ]] || continue
		read -r vendor < "$card_path/device/vendor" || continue
		read -r device < "$card_path/device/device" || continue
		read -r subsystem_vendor < "$card_path/device/subsystem_vendor" || continue
		read -r subsystem_device < "$card_path/device/subsystem_device" || continue

		if [[ ${vendor,,} == 0x1102 &&
			${device,,} == 0x0012 &&
			${subsystem_vendor,,} == 0x1102 &&
			${subsystem_device,,} == 0x0051 ]]; then
			printf '%s\n' "${card_path##*card}"
			return 0
		fi
	done
	return 1
}

wait_for_alsa_control() {
	local card_index=$1 attempt

	for ((attempt = 1; attempt <= 50; attempt++)); do
		if amixer -c "$card_index" info >/dev/null 2>&1; then
			return 0
		fi
		(( attempt == 50 )) || sleep 0.1
	done
	return 1
}

collect_codec_pins() {
	local card_index=$1 codec found=0

	shopt -s nullglob
	for codec in "/proc/asound/card$card_index"/codec#*; do
		[[ -r "$codec" ]] || continue
		found=1
		printf 'codec=%s\n' "$codec"
		awk '
			/^Node 0x/ {
				wanted = ($2 == "0x0b" || $2 == "0x0f" || $2 == "0x10" || $2 == "0x11")
			}
			wanted && /^Node 0x/ {
				print
				next
			}
			wanted && (/Pin Default/ || /Pin-ctls/ || /Unsolicited/) {
				print
			}
		' "$codec"
	done

	(( found )) || printf '[no readable codec data]\n'
}

collect_pipewire() {
	local card_index=$1 sinks cards

	if ! command -v systemctl >/dev/null 2>&1 ||
		! systemctl --user is-active --quiet pipewire.service; then
		printf '[PipeWire is not active; query skipped to avoid socket activation]\n'
		return
	fi
	if ! command -v pactl >/dev/null 2>&1; then
		printf '[pactl unavailable]\n'
		return
	fi
	if ! command -v jq >/dev/null 2>&1; then
		printf '[jq unavailable]\n'
		return
	fi

	if sinks=$(pactl --format=json list sinks 2>/dev/null); then
		jq -r --arg card "$card_index" '
			.[]
			| select(.properties["alsa.card"] == $card)
			| "sink_name=\(.name)",
			  "sink_state=\(.state)",
			  "sink_sample_specification=\(.sample_specification)",
			  "sink_active_port=\(.active_port)",
			  (.ports[] |
				"sink_port=\(.name);availability=\(.availability)")
		' <<< "$sinks"
	else
		printf '[unable to query PipeWire sinks]\n'
	fi

	if cards=$(pactl --format=json list cards 2>/dev/null); then
		jq -r --arg card "$card_index" '
			.[]
			| select(.properties["alsa.card"] == $card)
			| "pipewire_card=\(.name)",
			  "pipewire_active_profile=\(.active_profile)"
		' <<< "$cards"
	else
		printf '[unable to query PipeWire cards]\n'
	fi
}

collect_service_state() {
	if ! command -v systemctl >/dev/null 2>&1; then
		printf '[systemctl unavailable]\n'
		return
	fi

	printf 'system alsa-state.service:\n'
	systemctl show alsa-state.service \
		--property=ActiveState \
		--property=SubState \
		--property=ActiveEnterTimestampMonotonic \
		--no-pager 2>&1 || true
	printf 'user pipewire.service and wireplumber.service:\n'
	systemctl --user show pipewire.service wireplumber.service \
		--property=Id \
		--property=ActiveState \
		--property=SubState \
		--property=ActiveEnterTimestampMonotonic \
		--no-pager 2>&1 || true
}

collect() {
	local label=$1 card_index boot_id

	printf '# AE-5 routing state\n'
	printf 'label=%s\n' "$label"
	printf 'generated_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
	printf 'kernel=%s\n' "$(uname -r)"
	if [[ -r /proc/sys/kernel/random/boot_id ]]; then
		read -r boot_id < /proc/sys/kernel/random/boot_id
		printf 'boot_id=%s\n' "$boot_id"
	fi

	if ! card_index=$(find_ae5_card); then
		printf 'error=no supported AE-5 (1102:0012 subsystem 1102:0051) found\n'
		return 1
	fi
	printf 'alsa_card=%s\n' "$card_index"

	section 'ALSA route controls'
	if command -v amixer >/dev/null 2>&1; then
		if wait_for_alsa_control "$card_index"; then
			printf 'alsa_control_ready=yes\n'
			amixer -c "$card_index" sget 'Output Select' 2>&1 || true
			amixer -c "$card_index" sget 'HP/Speaker Auto Detect' 2>&1 || true
			amixer -c "$card_index" \
				cget "iface=CARD,name='Headphone Jack'" 2>&1 || true
		else
			printf '[ALSA control did not become readable within 5 seconds]\n'
		fi
	else
		printf '[amixer unavailable]\n'
	fi

	section 'Codec routing pins'
	collect_codec_pins "$card_index"

	section 'Service timing and state'
	collect_service_state

	section 'PipeWire route'
	collect_pipewire "$card_index"
}

run_self_test() (
	local calls=0

	sleep() {
		:
	}
	amixer() {
		calls=$((calls + 1))
		(( calls >= 3 ))
	}

	if ! wait_for_alsa_control 0 || (( calls != 3 )); then
		printf 'self-test failed: ALSA readiness retry\n' >&2
		return 1
	fi

	calls=0
	amixer() {
		calls=$((calls + 1))
		return 1
	}

	if wait_for_alsa_control 0 || (( calls != 50 )); then
		printf 'self-test failed: ALSA readiness timeout\n' >&2
		return 1
	fi

	printf 'routing state self-test passed\n'
)

if [[ ${1:-} == --self-test ]]; then
	[[ $# -eq 1 ]] || {
		usage
		exit 2
	}
	run_self_test
	exit
fi

label=${1:-manual}
append=false

if [[ $# -gt 2 ]]; then
	usage
	exit 2
fi
if [[ $# -eq 2 ]]; then
	[[ $2 == --append ]] || {
		usage
		exit 2
	}
	append=true
fi

umask 077
if $append; then
	state_root=${XDG_STATE_HOME:-"$HOME/.local/state"}
	output=${AE5_ROUTING_LOG:-"$state_root/ae5-control/routing-boot.log"}
	mkdir -p -- "$(dirname -- "$output")"
	collect "$label" >> "$output"
	printf 'routing state appended to %s\n' "$output" >&2
else
	collect "$label"
fi
