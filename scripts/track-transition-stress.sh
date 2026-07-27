#!/usr/bin/env bash
# Exercise real PipeWire client transition shapes while the AE-5 analog path is
# hard-muted. The command never enables S32; it only runs when the already-live
# sink format matches the explicitly requested baseline.
set -euo pipefail

script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
sys_root=${AE5_SYSFS_ROOT:-/sys}
proc_root=${AE5_PROCFS_ROOT:-/proc}
state_root=${XDG_STATE_HOME:-"$HOME/.local/state"}
evidence_root=${AE5_TRANSITION_ROOT:-"$state_root/ae5-control/transition-stress"}
trials=${AE5_TRANSITION_TRIALS:-5}
tap_threshold=${AE5_TRANSITION_THRESHOLD:--25}
ae5ctl=${AE5CTL:-ae5ctl}
pactl=${AE5_PACTL:-pactl}
sink_name=
card_index=
run_root=
parent_pid=$$
declare -a monitor_pids=()
declare -a client_pids=()

usage() {
	cat >&2 <<EOF
usage:
  $(basename "$0") --dry-run
  AE5_TRANSITION_ACK=hard-muted $(basename "$0") run s16le|s32le
  $(basename "$0") --self-test

The run command:
  - targets only the exact 1102:0012 / 1102:0051 AE-5 sink;
  - hard-mutes and continuously watches Master and Front;
  - sets only that sink to 20% and leaves it muted afterward;
  - refuses to change the live hardware format or run fewer than five trials.

Set AE5_TRANSITION_TAP_PROBE=front-muted to sample What U Hear after each
transition. The probe keeps Front off, mutes the PipeWire sink, briefly enables
Master for the capture, then disables Master again.
EOF
}

fail() {
	printf 'error: %s\n' "$*" >&2
	exit 1
}

need_tool() {
	command -v "$1" >/dev/null 2>&1 ||
		fail "required tool is unavailable: $1"
}

find_ae5_card() {
	local card_path vendor device subsystem_vendor subsystem_device identity
	local -a matches=()

	shopt -s nullglob
	for card_path in "$sys_root"/class/sound/card[0-9]*; do
		[[ -r $card_path/device/vendor ]] || continue
		read -r vendor < "$card_path/device/vendor"
		read -r device < "$card_path/device/device"
		read -r subsystem_vendor < "$card_path/device/subsystem_vendor"
		read -r subsystem_device < "$card_path/device/subsystem_device"
		identity=${vendor,,}:${device,,}:${subsystem_vendor,,}:${subsystem_device,,}
		if [[ $identity == 0x1102:0x0012:0x1102:0x0051 ]]; then
			matches+=("${card_path##*card}")
		fi
	done
	((${#matches[@]} == 1)) ||
		fail "expected exactly one AE-5 1102:0012/1102:0051, found ${#matches[@]}"
	printf '%s\n' "${matches[0]}"
}

sink_snapshot() {
	"$pactl" --format=json list sinks
}

find_ae5_sink() {
	local card=$1 snapshot=$2
	local -a matches=()

	mapfile -t matches < <(
		jq -r --arg card "$card" '
			.[]
			| select(
				(.properties["alsa.card"] | tostring) == $card
				and ((.properties["alsa.components"] // "")
					| startswith("HDA:11020011,11020051,"))
				and .properties["api.alsa.pcm.stream"] == "playback"
			)
			| [
				.name,
				.state,
				.sample_specification,
				(.mute | tostring),
				([.volume[].value] | max | tostring),
				.properties["api.alsa.soft-mixer"],
				.properties["audio.format"],
				.active_port
			]
			| @tsv
		' <<< "$snapshot"
	)
	((${#matches[@]} == 1)) ||
		fail "expected exactly one live PipeWire sink for ALSA card $card, found ${#matches[@]}"
	printf '%s\n' "${matches[0]}"
}

pcm_states() {
	local card=$1 path state found=0
	local -a statuses=()

	shopt -s nullglob
	statuses=("$proc_root/asound/card$card"/pcm*p/sub*/status)
	for path in "${statuses[@]}"; do
		found=1
		read -r state < "$path" || state=unreadable
		printf '%s=%s\n' "${path#"$proc_root/asound/card$card/"}" "$state"
	done
	((found)) || return 1
}

all_playback_pcms_closed() {
	local card=$1 snapshot

	snapshot=$(pcm_states "$card") || return 1
	! grep -v '=closed$' <<< "$snapshot" | grep -q .
}

switch_is_off() {
	"$ae5ctl" get "$1" 2>/dev/null | grep -Fq 'playback off'
}

hard_mute() {
	local failed=0

	"$ae5ctl" set-playback-switch Master off >/dev/null || failed=1
	"$ae5ctl" set-playback-switch Front off >/dev/null || failed=1
	switch_is_off Master || failed=1
	switch_is_off Front || failed=1
	((failed == 0))
}

log_event() {
	local event=$1 detail=${2:-}

	printf '%s\t%s\t%s\t%s\n' \
		"$(date --iso-8601=ns)" \
		"$(cut -d' ' -f1 "$proc_root/uptime")" \
		"$event" "$detail" >> "$run_root/events.tsv"
}

stop_monitors() {
	local pid

	for pid in "${monitor_pids[@]}"; do
		kill "$pid" 2>/dev/null || true
	done
	for pid in "${monitor_pids[@]}"; do
		wait "$pid" 2>/dev/null || true
	done
	monitor_pids=()
}

stop_clients() {
	local pid

	for pid in "${client_pids[@]}"; do
		kill "$pid" 2>/dev/null || true
	done
	for pid in "${client_pids[@]}"; do
		wait "$pid" 2>/dev/null || true
	done
	client_pids=()
}

forget_client() {
	local completed=$1 pid
	local -a active=()

	for pid in "${client_pids[@]}"; do
		[[ $pid == "$completed" ]] || active+=("$pid")
	done
	client_pids=("${active[@]}")
}

cleanup() {
	local status=$?

	stop_clients
	stop_monitors
	if [[ -n $sink_name ]]; then
		"$pactl" set-sink-mute "$sink_name" 1 >/dev/null 2>&1 || true
	fi
	if [[ -n $card_index ]]; then
		"$ae5ctl" set-playback-switch Master off >/dev/null 2>&1 || true
		"$ae5ctl" set-playback-switch Front off >/dev/null 2>&1 || true
	fi
	return "$status"
}

pcm_watch() {
	local path state details
	local -a statuses=()

	shopt -s nullglob
	statuses=("$proc_root/asound/card$card_index"/pcm*p/sub*/status)
	while true; do
		details=
		for path in "${statuses[@]}"; do
			state=$(tr '\n' ';' < "$path" 2>/dev/null || printf unreadable)
			details+="${path#"$proc_root/asound/card$card_index/"}:$state "
		done
		printf '%s\t%s\t%s\n' \
			"$(date --iso-8601=ns)" \
			"$(cut -d' ' -f1 "$proc_root/uptime")" \
			"$details"
		sleep 0.05
	done
}

mute_watchdog() {
	while kill -0 "$parent_pid" 2>/dev/null; do
		if ! switch_is_off Front ||
			{ ! switch_is_off Master &&
				[[ ! -e $run_root/master-probe-active ]]; }; then
			hard_mute >/dev/null 2>&1 || true
			log_event safety-anomaly \
				'Master or Front left the hard-muted state; run terminated'
			kill -TERM "$parent_pid" 2>/dev/null || true
			return
		fi
		sleep 0.1
	done
}

start_monitors() {
	pcm_watch > "$run_root/pcm-timeline.tsv" 2>&1 &
	monitor_pids+=("$!")
	mute_watchdog &
	monitor_pids+=("$!")
	pw-dump -m -N > "$run_root/pipewire-events.json" 2>&1 &
	monitor_pids+=("$!")
	pw-mon -p > "$run_root/pipewire-events.log" 2>&1 &
	monitor_pids+=("$!")
	pw-top -b -n 100000 > "$run_root/pipewire-top.log" 2>&1 &
	monitor_pids+=("$!")
	journalctl --user -f -n 0 --no-pager --output=short-monotonic \
		-u pipewire.service -u wireplumber.service \
		> "$run_root/pipewire-journal.log" 2>&1 &
	monitor_pids+=("$!")
	journalctl -k -f -n 0 --no-pager --output=short-monotonic \
		> "$run_root/kernel-journal.log" 2>&1 &
	monitor_pids+=("$!")
}

save_snapshot() {
	local label=$1 path

	mkdir -p "$run_root/snapshots/$label"
	sink_snapshot > "$run_root/snapshots/$label/sinks.json"
	pw-dump > "$run_root/snapshots/$label/pw-dump.json"
	wpctl status -n > "$run_root/snapshots/$label/wpctl.txt"
	amixer -c "$card_index" contents \
		> "$run_root/snapshots/$label/mixer.txt"
	for path in "$proc_root/asound/card$card_index"/pcm*p/sub*/status \
		"$proc_root/asound/card$card_index"/pcm*p/sub*/hw_params \
		"$proc_root/asound/card$card_index"/pcm*p/sub*/sw_params; do
		[[ -r $path ]] || continue
		printf '== %s\n' "$path"
		cat "$path"
	done > "$run_root/snapshots/$label/pcm.txt"
}

run_client() {
	local label=$1 fixture=$2 limit=${3:-} status
	local stderr=$run_root/clients/"$label".log
	local -a command=(pw-play --target "$sink_name" "$fixture")

	log_event client-start "$label ${fixture##*/} limit=${limit:-none}"
	set +e
	if [[ -n $limit ]]; then
		timeout --foreground --signal=TERM "$limit" "${command[@]}" \
			>/dev/null 2> "$stderr"
		status=$?
	else
		timeout --foreground --signal=TERM 5 "${command[@]}" \
			>/dev/null 2> "$stderr"
		status=$?
	fi
	set -e
	if [[ -n $limit ]]; then
		[[ $status == 0 || $status == 124 || $status == 143 ]] ||
			return "$status"
	else
		[[ $status == 0 ]] || return "$status"
	fi
	log_event client-end "$label status=$status"
}

run_overlap() {
	local label=$1 first=$2 second=$3 first_pid second_pid first_status second_status

	log_event client-start "$label-a ${first##*/}"
	timeout --foreground --signal=TERM 5 \
	pw-play --target "$sink_name" "$first" \
		>/dev/null 2> "$run_root/clients/$label-a.log" &
	first_pid=$!
	client_pids+=("$first_pid")
	sleep 0.6
	log_event client-start "$label-b ${second##*/}"
	timeout --foreground --signal=TERM 5 \
	pw-play --target "$sink_name" "$second" \
		>/dev/null 2> "$run_root/clients/$label-b.log" &
	second_pid=$!
	client_pids+=("$second_pid")
	set +e
	wait "$first_pid"
	first_status=$?
	forget_client "$first_pid"
	wait "$second_pid"
	second_status=$?
	forget_client "$second_pid"
	set -e
	log_event client-end "$label-a status=$first_status"
	log_event client-end "$label-b status=$second_status"
	((first_status == 0 && second_status == 0))
}

transition_plan() {
	cat <<'EOF'
reconnect	complete close, 50 ms gap, new client
abrupt	TERM during playback, immediate new client
rate-format	44.1 kHz S16 client followed by 48 kHz S32 client
gapless	overlapping 48 kHz S32 and 96 kHz S32 clients
suspend-boundary	short client, one-second idle, new client
EOF
}

what_u_hear_device() {
	arecord -l 2>/dev/null | awk -v card="$card_index" '
		$0 ~ "^card " card ":" && tolower($0) ~ /what u hear/ {
			if (match($0, /device ([0-9]+)/, found)) {
				print card "," found[1]
				exit
			}
		}
	'
}

rms_is_anomaly() {
	local rms=$1

	[[ $rms != -inf && $rms != nan ]] &&
		awk -v rms="$rms" -v threshold="$tap_threshold" \
			'BEGIN { exit !(rms > threshold) }'
}

probe_dsp() {
	local label=$1 device capture rms

	[[ ${AE5_TRANSITION_TAP_PROBE:-off} == front-muted ]] || return 0
	"$pactl" set-sink-mute "$sink_name" 1
	for _ in {1..40}; do
		all_playback_pcms_closed "$card_index" && break
		sleep 0.05
	done
	all_playback_pcms_closed "$card_index" ||
		fail 'playback PCM did not close before the What U Hear probe'
	switch_is_off Front || fail 'Front is not hard-muted before tap probe'
	device=$(what_u_hear_device)
	[[ -n $device ]] || fail 'What U Hear capture device was not found'
	capture=$run_root/tap/"$label".wav
	: > "$run_root/master-probe-active"
	"$ae5ctl" set-playback-switch Master on >/dev/null
	if ! arecord -q -D "hw:$device" -f S32_LE -c 2 -r 48000 \
		-d 1 "$capture"; then
		hard_mute
		fail 'What U Hear capture failed'
	fi
	"$ae5ctl" set-playback-switch Master off >/dev/null
	switch_is_off Master || fail 'Master did not return to hard mute'
	rm -f -- "$run_root/master-probe-active"
	rms=$(sox "$capture" -n stats 2>&1 |
		awk '/RMS lev dB/ { print $4; found=1 }
			END { if (!found) print "nan" }')
	printf '%s\t%s\n' "$label" "$rms" >> "$run_root/tap-rms.tsv"
	log_event tap-rms "$label rms_dbfs=$rms"
	if rms_is_anomaly "$rms"; then
		save_snapshot "anomaly-$label"
		fail "DSP anomaly detected after $label: $rms dBFS"
	fi
	"$pactl" set-sink-mute "$sink_name" 0
}

run_scenarios() {
	local trial=$1 fixtures=$run_root/fixtures
	local prefix

	prefix=$(printf 'trial-%02d' "$trial")
	run_client "$prefix-reconnect-a" \
		"$fixtures/transition-a-44100-s16.wav"
	sleep 0.05
	run_client "$prefix-reconnect-b" \
		"$fixtures/transition-b-48000-s16.wav"
	probe_dsp "$prefix-reconnect"

	run_client "$prefix-abrupt-a" \
		"$fixtures/transition-c-48000-s32.wav" 0.35
	run_client "$prefix-abrupt-b" \
		"$fixtures/transition-d-96000-s32.wav"
	probe_dsp "$prefix-abrupt"

	run_client "$prefix-rate-a" \
		"$fixtures/transition-a-44100-s16.wav"
	run_client "$prefix-rate-b" \
		"$fixtures/transition-c-48000-s32.wav"
	probe_dsp "$prefix-rate-format"

	run_overlap "$prefix-gapless" \
		"$fixtures/transition-c-48000-s32.wav" \
		"$fixtures/transition-d-96000-s32.wav"
	probe_dsp "$prefix-gapless"

	run_client "$prefix-suspend-a" \
		"$fixtures/transition-b-48000-s16.wav" 0.35
	sleep 1
	run_client "$prefix-suspend-b" \
		"$fixtures/transition-c-48000-s32.wav"
	probe_dsp "$prefix-suspend-boundary"
}

describe_live_state() {
	local snapshot record node state sample_spec muted volume soft_mixer format port

	card_index=$(find_ae5_card)
	snapshot=$(sink_snapshot)
	record=$(find_ae5_sink "$card_index" "$snapshot")
	IFS=$'\t' read -r node state sample_spec muted volume soft_mixer format port \
		<<< "$record"
	printf 'AE-5 ALSA card: %s\n' "$card_index"
	printf 'PipeWire target: %s\n' "$node"
	printf 'Sink state: %s\n' "$state"
	printf 'Live sample specification: %s\n' "$sample_spec"
	printf 'Managed audio.format: %s\n' "$format"
	printf 'Soft mixer: %s\n' "$soft_mixer"
	printf 'Sink mute: %s\n' "$muted"
	printf 'Sink volume: %s\n' "$(LC_ALL=C awk -v value="$volume" \
		'BEGIN { printf "%.1f%%", value * 100 / 65536 }')"
	printf 'Active port: %s\n' "$port"
	printf 'Playback PCMs:\n'
	pcm_states "$card_index" | sed 's/^/  /'
	printf 'Transition plan:\n'
	transition_plan | sed 's/^/  /'
	printf 'HDA position trace (separate root terminal):\n'
	printf '  sudo %q record %q 180 > hda-position.log\n' \
		"$script_root/hda-position-trace.sh" "$card_index"
}

run_stress() {
	local expected=$1 snapshot record state sample_spec muted volume soft_mixer format port
	local trial

	[[ ${AE5_TRANSITION_ACK:-} == hard-muted ]] ||
		fail 'set AE5_TRANSITION_ACK=hard-muted to acknowledge the fail-closed run'
	[[ $expected == s16le || $expected == s32le ]] ||
		fail 'expected format must be s16le or s32le'
	if [[ ! $trials =~ ^[0-9]+$ ]] || ((trials < 5)); then
		fail 'AE5_TRANSITION_TRIALS must be at least 5'
	fi
	for tool in "$ae5ctl" "$pactl" jq pw-play pw-dump pw-mon pw-top \
		wpctl amixer journalctl sox arecord sha256sum; do
		need_tool "$tool"
	done

	card_index=$(find_ae5_card)
	snapshot=$(sink_snapshot)
	record=$(find_ae5_sink "$card_index" "$snapshot")
	IFS=$'\t' read -r sink_name state sample_spec muted volume soft_mixer format port \
		<<< "$record"
	[[ $soft_mixer == true ]] ||
		fail 'the exact AE-5 sink is not using PipeWire software volume'
	[[ ${sample_spec%% *} == "$expected" ]] ||
		fail "live sink is ${sample_spec%% *}; refusing requested $expected run"
	all_playback_pcms_closed "$card_index" ||
		fail 'an AE-5 playback PCM is already open'

	run_root=$evidence_root/$(date +%Y%m%d-%H%M%S)-"$expected"
	[[ ! -e $run_root ]] || fail "evidence directory already exists: $run_root"
	mkdir -p "$run_root"/{clients,fixtures,snapshots,tap}
	trap cleanup EXIT
	trap 'exit 130' HUP INT TERM
	save_snapshot original
	hard_mute || fail 'Master and Front did not enter the hard-muted state'
	"$pactl" set-sink-mute "$sink_name" 1
	"$pactl" set-sink-volume "$sink_name" 20%
	"$script_root/audio-parity.sh" generate-transitions "$run_root/fixtures" \
		> "$run_root/fixture-generation.log"

	{
		printf 'generated=%s\n' "$(date --iso-8601=seconds)"
		printf 'kernel=%s\n' "$(uname -r)"
		printf 'kernel_taint=%s\n' "$(cat "$proc_root/sys/kernel/tainted")"
		printf 'card=%s\n' "$card_index"
		printf 'target=%s\n' "$sink_name"
		printf 'live_sample_spec=%s\n' "$sample_spec"
		printf 'initial_sink_state=%s\n' "$state"
		printf 'initial_sink_muted=%s\n' "$muted"
		printf 'initial_sink_volume_raw=%s\n' "$volume"
		printf 'soft_mixer=%s\n' "$soft_mixer"
		printf 'managed_audio_format=%s\n' "$format"
		printf 'expected_format=%s\n' "$expected"
		printf 'trials=%s\n' "$trials"
		printf 'tap_probe=%s\n' "${AE5_TRANSITION_TAP_PROBE:-off}"
		printf 'active_port=%s\n' "$port"
	} > "$run_root/manifest.txt"
	printf 'wall_time\tmonotonic_s\tevent\tdetail\n' > "$run_root/events.tsv"
	printf 'label\trms_dbfs\n' > "$run_root/tap-rms.tsv"
	save_snapshot before
	start_monitors
	"$pactl" set-sink-mute "$sink_name" 0
	log_event run-start "format=$expected trials=$trials target=$sink_name"
	for ((trial = 1; trial <= trials; trial++)); do
		log_event trial-start "$trial"
		run_scenarios "$trial"
		log_event trial-end "$trial"
	done
	"$pactl" set-sink-mute "$sink_name" 1
	hard_mute
	stop_monitors
	save_snapshot after
	log_event run-complete 'sink muted; Master and Front off'
	printf 'transition stress completed; evidence: %s\n' "$run_root"
	printf 'the AE-5 sink, Master, and Front were left muted\n'
}

self_test() (
	local test_root record output

	test_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-transition-test.XXXXXX")
	trap 'find "$test_root" -depth -delete' EXIT
	mkdir -p \
		"$test_root/sys/class/sound/card7/device" \
		"$test_root/proc/asound/card7/pcm0p/sub0" \
		"$test_root/proc/asound/card7/pcm1p/sub0"
	printf '0x1102\n' > "$test_root/sys/class/sound/card7/device/vendor"
	printf '0x0012\n' > "$test_root/sys/class/sound/card7/device/device"
	printf '0x1102\n' > "$test_root/sys/class/sound/card7/device/subsystem_vendor"
	printf '0x0051\n' > "$test_root/sys/class/sound/card7/device/subsystem_device"
	printf 'closed\n' > "$test_root/proc/asound/card7/pcm0p/sub0/status"
	printf 'closed\n' > "$test_root/proc/asound/card7/pcm1p/sub0/status"
	sys_root=$test_root/sys
	proc_root=$test_root/proc
	card_index=$(find_ae5_card)
	[[ $card_index == 7 ]]
	all_playback_pcms_closed "$card_index"
	printf 'RUNNING\n' > "$test_root/proc/asound/card7/pcm1p/sub0/status"
	if all_playback_pcms_closed "$card_index"; then
		fail 'open PCM unexpectedly passed'
	fi

	record=$(find_ae5_sink 7 '[
	  {
	    "name": "alsa_output.pci-ae5.analog-stereo",
	    "state": "SUSPENDED",
	    "sample_specification": "s32le 2ch 48000Hz",
	    "mute": true,
	    "volume": {
	      "front-left": {"value": 13107},
	      "front-right": {"value": 13107}
	    },
	    "properties": {
	      "alsa.card": "7",
	      "alsa.components": "HDA:11020011,11020051,00100918",
	      "api.alsa.pcm.stream": "playback",
	      "api.alsa.soft-mixer": "true",
	      "audio.format": "S32LE"
	    },
	    "active_port": "sound-blaster-ae5-output-headphones;output-headphones"
	  }
	]')
	IFS=$'\t' read -r output _ <<< "$record"
	[[ $output == alsa_output.pci-ae5.analog-stereo ]]
	[[ $(transition_plan | wc -l) == 5 ]]
	rms_is_anomaly -6
	if rms_is_anomaly -38 || rms_is_anomaly -inf; then
		fail 'clean tap level was classified as an anomaly'
	fi
	printf 'track transition stress self-test passed\n'
)

case ${1:-} in
--dry-run)
	[[ $# -eq 1 ]] || {
		usage
		exit 2
	}
	for tool in "$pactl" jq; do
		need_tool "$tool"
	done
	describe_live_state
	;;
run)
	[[ $# -eq 2 ]] || {
		usage
		exit 2
	}
	run_stress "$2"
	;;
--self-test)
	[[ $# -eq 1 ]] || {
		usage
		exit 2
	}
	need_tool jq
	self_test
	;;
*)
	usage
	exit 2
	;;
esac
