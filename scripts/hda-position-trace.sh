#!/usr/bin/env bash
# Capture the kernel's existing HDA lifecycle and DMA-position tracepoints.
# Run as root in a separate terminal while track-transition-stress.sh runs as
# the desktop user. Nothing here changes mixer, routing, or playback state.
set -euo pipefail

trace_root=${AE5_TRACEFS_ROOT:-/sys/kernel/tracing}
sys_root=${AE5_SYSFS_ROOT:-/sys}
readonly -a events=(
	hda_controller/azx_pcm_open
	hda_controller/azx_pcm_close
	hda_controller/azx_pcm_hw_params
	hda_controller/azx_pcm_prepare
	hda_controller/azx_pcm_trigger
	hda_controller/azx_get_position
)
saved_tracing_on=
declare -a saved_event_values=()

usage() {
	printf 'usage: %s record ALSA_CARD DURATION_SECONDS | --self-test\n' "$0" >&2
}

fail() {
	printf 'error: %s\n' "$*" >&2
	exit 1
}

verify_card() {
	local card=$1 device_root
	local vendor device subsystem_vendor subsystem_device identity

	device_root=$sys_root/class/sound/card"$card"/device
	[[ $card =~ ^[0-9]+$ && -d $device_root ]] ||
		fail "ALSA card $card is unavailable"
	read -r vendor < "$device_root/vendor"
	read -r device < "$device_root/device"
	read -r subsystem_vendor < "$device_root/subsystem_vendor"
	read -r subsystem_device < "$device_root/subsystem_device"
	identity=${vendor,,}:${device,,}:${subsystem_vendor,,}:${subsystem_device,,}
	[[ $identity == 0x1102:0x0012:0x1102:0x0051 ]] ||
		fail "card $card is not the audited AE-5 1102:0012/1102:0051"
}

filter_trace() {
	local card=$1

	awk -v card="$card" '
		/hda_controller:azx_(get_position|pcm_trigger):/ {
			if (index($0, "[" card ":"))
				print
			next
		}
		/hda_controller:azx_pcm_(open|close|hw_params|prepare):/ {
			print
		}
	'
}

restore_trace_state() {
	local index enable

	[[ -n $saved_tracing_on ]] || return
	for index in "${!events[@]}"; do
		enable=$trace_root/events/${events[$index]}/enable
		printf '%s\n' "${saved_event_values[$index]}" > "$enable" 2>/dev/null ||
			true
	done
	printf '%s\n' "$saved_tracing_on" > "$trace_root/tracing_on" 2>/dev/null ||
		true
}

record() {
	local card=$1 duration=$2 event enable index status

	((EUID == 0)) || fail 'trace capture must run as root'
	[[ $duration =~ ^[1-9][0-9]*$ ]] ||
		fail 'duration must be a positive integer'
	verify_card "$card"
	[[ -r $trace_root/trace_pipe && -w $trace_root/tracing_on ]] ||
		fail "tracefs is unavailable at $trace_root"

	read -r saved_tracing_on < "$trace_root/tracing_on"
	for event in "${events[@]}"; do
		enable=$trace_root/events/$event/enable
		[[ -w $enable ]] || fail "required tracepoint is unavailable: $event"
		read -r status < "$enable"
		saved_event_values+=("$status")
	done
	trap restore_trace_state EXIT
	trap 'exit 130' HUP INT TERM

	for event in "${events[@]}"; do
		printf '1\n' > "$trace_root/events/$event/enable"
	done
	printf '1\n' > "$trace_root/tracing_on"

	printf '# AE-5 HDA position trace\n'
	printf '# generated=%s\n' "$(date --iso-8601=seconds)"
	printf '# card=%s\n' "$card"
	printf '# kernel=%s\n' "$(uname -r)"
	printf '# note=open/close/hw_params/prepare events lack a card field upstream; position and trigger lines are filtered to card %s\n' \
		"$card"

	set +e
	timeout --foreground "$duration" \
		stdbuf -oL cat "$trace_root/trace_pipe" |
		filter_trace "$card"
	status=${PIPESTATUS[0]}
	set -e
	[[ $status == 0 || $status == 124 || $status == 143 ]] ||
		fail "trace_pipe reader failed with status $status"
}

self_test() (
	local test_root output

	test_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-hda-trace-test.XXXXXX")
	trap 'find "$test_root" -depth -delete' EXIT
	mkdir -p "$test_root/sys/class/sound/card7/device"
	printf '0x1102\n' > "$test_root/sys/class/sound/card7/device/vendor"
	printf '0x0012\n' > "$test_root/sys/class/sound/card7/device/device"
	printf '0x1102\n' > "$test_root/sys/class/sound/card7/device/subsystem_vendor"
	printf '0x0051\n' > "$test_root/sys/class/sound/card7/device/subsystem_device"
	sys_root=$test_root/sys
	verify_card 7

	output=$(filter_trace 7 <<'EOF'
 task-1 [001] hda_controller:azx_pcm_open: stream_tag: 1
 task-1 [001] hda_controller:azx_pcm_trigger: [7:0] cmd=1
 task-1 [001] hda_controller:azx_get_position: [7:0] pos=6016, delay=0
 task-1 [001] hda_controller:azx_get_position: [3:0] pos=2048, delay=0
 unrelated event
EOF
)
	grep -Fq 'azx_pcm_open' <<< "$output"
	grep -Fq 'azx_pcm_trigger: [7:0]' <<< "$output"
	grep -Fq 'azx_get_position: [7:0]' <<< "$output"
	if grep -Fq '[3:0]' <<< "$output"; then
		fail 'trace filter retained another card'
	fi
	printf 'HDA position trace self-test passed\n'
)

case ${1:-} in
record)
	[[ $# -eq 3 ]] || {
		usage
		exit 2
	}
	record "$2" "$3"
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
