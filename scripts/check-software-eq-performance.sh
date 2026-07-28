#!/usr/bin/env bash
# Measure the in-place software EQ without exposing an analog output.
set -euo pipefail

readonly sample_rate=48000
readonly tone_hz=997
readonly tone_level_db=-30
readonly default_soak_seconds=600
readonly long_duration_seconds=7200
readonly benchmark_seconds=${AE5_EQ_BENCHMARK_SECONDS:-15}
readonly top_iterations=${AE5_EQ_TOP_ITERATIONS:-12}
readonly cpu_budget_percent=${AE5_EQ_CPU_BUDGET_PERCENT:-2}
readonly busy_budget_percent=${AE5_EQ_BUSY_BUDGET_PERCENT:-5}
readonly ae5ctl=${AE5CTL:-ae5ctl}

card_index=
sink_id=
sink_name=
sink_serial=
original_volume=
original_mute=
master_state=
front_state=
evidence_root=
stream_pid=
watchdog_pid=
eq_owned=false
matrix_passed=false
soak_seconds=
qualification=smoke

usage() {
	cat >&2 <<'EOF'
usage:
  check-software-eq-performance.sh PROFILE.json [SOAK_SECONDS]
  check-software-eq-performance.sh --dry-run PROFILE.json [SOAK_SECONDS]
  check-software-eq-performance.sh --self-test

Live runs require AE5_ANALOG_OUTPUTS_UNPLUGGED=1 after physically checking
that every AE-5 analog output is unplugged. The default smoke soak is 600
seconds; 7200 seconds or more satisfies this harness's long-duration gate.
EOF
	exit 2
}

fail() {
	printf 'error: %s\n' "$1" >&2
	exit 1
}

require_command() {
	command -v "$1" >/dev/null 2>&1 ||
		fail "required command is unavailable: $1"
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

number_is_at_most() {
	awk -v actual="$1" -v limit="$2" \
		'BEGIN { exit !(actual + 0 <= limit + 0) }'
}

playback_switch_state() {
	local control=$1 states

	states=$(LC_ALL=C amixer -c "$card_index" sget "$control" 2>/dev/null |
		sed -n 's/.*Playback.*\[\(on\|off\)\]$/\1/p' |
		sort -u) || return
	case $states in
	on | off)
		printf '%s\n' "$states"
		;;
	*)
		return 1
		;;
	esac
}

restore_switch() {
	local control=$1 state=$2 action=mute

	[[ $state == on ]] && action=unmute
	LC_ALL=C amixer -q -c "$card_index" sset "$control" "$action" >/dev/null
}

hard_mute() {
	[[ -n $card_index ]] || return 0
	LC_ALL=C amixer -q -c "$card_index" sset Master mute >/dev/null
	LC_ALL=C amixer -q -c "$card_index" sset Front mute >/dev/null
}

pcm_is_closed() {
	local status

	for status in "/proc/asound/card$card_index"/pcm0p/sub*/status \
		"/proc/asound/card$card_index"/pcm1p/sub*/status; do
		[[ -r $status ]] || return 1
		grep -Fqx closed "$status" || return 1
	done
}

wait_for_pcm_close() {
	for _ in {1..50}; do
		pcm_is_closed && return
		sleep 0.1
	done
	return 1
}

sink_snapshot() {
	LC_ALL=C wpctl get-volume "$sink_id"
}

sink_volume() {
	awk '$1 == "Volume:" && $2 ~ /^[0-9]+([.][0-9]+)?$/ {
		if (++count == 1)
			volume = $2
	}
	END {
		if (count != 1)
			exit 1
		print volume
	}' <<< "$1"
}

sink_mute() {
	if grep -Fq '[MUTED]' <<< "$1"; then
		printf '1\n'
	else
		printf '0\n'
	fi
}

node_snapshot() {
	pw-dump |
		jq -r --arg card "$card_index" '
			[
				.[] |
				select(
					.type == "PipeWire:Interface:Node" and
					.info.props["media.class"] == "Audio/Sink" and
					(.info.props["alsa.card"] | tostring) == $card and
					.info.props["alsa.device"] == 0
				) |
				[
					.id,
					.info.props["object.serial"],
					.info.props["node.name"]
				] |
				@tsv
			] |
			if length == 1 then .[0] else empty end
		'
}

cpu_snapshot() {
	local pid=$1 user system uptime

	read -r _ _ _ _ _ _ _ _ _ _ _ _ _ user system _ < "/proc/$pid/stat"
	read -r uptime _ < /proc/uptime
	printf '%s\t%s\n' "$((user + system))" "$uptime"
}

cpu_percent() {
	local before=$1 after=$2 hertz before_ticks before_time
	local after_ticks after_time

	hertz=$(getconf CLK_TCK)
	IFS=$'\t' read -r before_ticks before_time <<< "$before"
	IFS=$'\t' read -r after_ticks after_time <<< "$after"
	awk -v ticks="$((after_ticks - before_ticks))" -v hertz="$hertz" \
		-v elapsed="$(awk -v start="$before_time" -v end="$after_time" \
			'BEGIN { print end - start }')" '
		BEGIN {
			if (elapsed <= 0)
				exit 1
			printf "%.4f\n", ticks * 100 / (hertz * elapsed)
		}
	'
}

top_stats() {
	local input=$1 node_id=$2

	LC_ALL=C awk -v node_id="$node_id" '
		function time_us(value) {
			if (value ~ /us$/) {
				sub(/us$/, "", value)
				return value + 0
			}
			if (value ~ /ms$/) {
				sub(/ms$/, "", value)
				return (value + 0) * 1000
			}
			return -1
		}
		$1 == "R" && $2 == node_id && $3 ~ /^[0-9]+$/ &&
				$4 ~ /^[0-9]+$/ {
			busy = time_us($6)
			if (busy < 0)
				next
			samples++
			quantum[$3] = 1
			rate[$4] = 1
			format[$10 " " $11 " " $12] = 1
			busy_total += busy
			if (busy > busy_max)
				busy_max = busy
			if (($9 + 0) > errors)
				errors = $9 + 0
		}
		END {
			for (value in quantum) {
				quantum_count++
				quantum_value = value
			}
			for (value in rate) {
				rate_count++
				rate_value = value
			}
			for (value in format) {
				format_count++
				format_value = value
			}
			if (samples < 3 || quantum_count != 1 || rate_count != 1 ||
					format_count != 1)
				exit 1
			printf "samples=%d\n", samples
			printf "quantum_frames=%s\n", quantum_value
			printf "rate_hz=%s\n", rate_value
			printf "format=%s\n", format_value
			printf "busy_mean_us=%.3f\n", busy_total / samples
			printf "busy_max_us=%.3f\n", busy_max
			printf "errors=%d\n", errors
		}
	' "$input"
}

compare_benchmarks() {
	local baseline=$1 equalized=$2
	local baseline_quantum baseline_rate baseline_format baseline_busy
	local equalized_quantum equalized_rate equalized_format equalized_busy
	local baseline_errors equalized_errors added_busy quantum_us busy_percent

	baseline_quantum=$(snapshot_value quantum_frames "$baseline")
	baseline_rate=$(snapshot_value rate_hz "$baseline")
	baseline_format=$(snapshot_value format "$baseline")
	baseline_busy=$(snapshot_value busy_mean_us "$baseline")
	baseline_errors=$(snapshot_value errors "$baseline")
	equalized_quantum=$(snapshot_value quantum_frames "$equalized")
	equalized_rate=$(snapshot_value rate_hz "$equalized")
	equalized_format=$(snapshot_value format "$equalized")
	equalized_busy=$(snapshot_value busy_mean_us "$equalized")
	equalized_errors=$(snapshot_value errors "$equalized")

	[[ $baseline_quantum == "$equalized_quantum" &&
		$baseline_rate == "$equalized_rate" &&
		$baseline_format == "$equalized_format" ]] || return 1
	[[ $baseline_errors == 0 && $equalized_errors == 0 ]] || return 1
	added_busy=$(awk -v baseline="$baseline_busy" -v equalized="$equalized_busy" \
		'BEGIN {
			delta = equalized - baseline
			printf "%.3f", (delta > 0 ? delta : 0)
		}')
	quantum_us=$(awk -v frames="$baseline_quantum" -v rate="$baseline_rate" \
		'BEGIN { printf "%.3f", frames * 1000000 / rate }')
	busy_percent=$(awk -v busy="$added_busy" -v quantum="$quantum_us" \
		'BEGIN { printf "%.4f", busy * 100 / quantum }')

	printf 'added_pipewire_buffer_frames=0\n'
	printf 'added_pipewire_buffer_ms=0.000\n'
	printf 'added_busy_us=%s\n' "$added_busy"
	printf 'added_busy_percent_of_quantum=%s\n' "$busy_percent"
	number_is_at_most "$busy_percent" "$busy_budget_percent"
}

require_safe_state() {
	local snapshot volume gain

	"$ae5ctl" route-status >/dev/null ||
		fail 'AE-5 ALSA and PipeWire routes must match'
	[[ $(playback_switch_state 'Enable OutFX') == off ]] ||
		fail 'hardware OutFX must remain off'
	gain=$(LC_ALL=C amixer -c "$card_index" \
		sget 'AE-5: Headphone Gain' 2>/dev/null) ||
		fail 'headphone gain is unreadable'
	grep -Fq "Item0: 'Low (16-31  Ohms)'" <<< "$gain" ||
		fail 'headphone gain must be Low'
	snapshot=$(sink_snapshot) || fail 'AE-5 sink volume is unreadable'
	volume=$(sink_volume "$snapshot") ||
		fail 'AE-5 sink volume is unparseable'
	number_is_at_most "$volume" 0.20 ||
		fail 'AE-5 sink volume exceeds 20%'
}

watch_safety() {
	local parent_pid=$1 snapshot volume

	while kill -0 "$parent_pid" 2>/dev/null; do
		if [[ $(playback_switch_state Master 2>/dev/null || true) != off ||
			$(playback_switch_state Front 2>/dev/null || true) != off ||
			$(playback_switch_state 'Enable OutFX' 2>/dev/null || true) != off ]]; then
			printf 'hardware mute or OutFX changed during the run\n' \
				> "$evidence_root/safety-anomaly.txt"
		elif ! snapshot=$(sink_snapshot 2>/dev/null); then
			printf 'AE-5 sink state became unreadable during the run\n' \
				> "$evidence_root/safety-anomaly.txt"
		elif ! volume=$(sink_volume "$snapshot" 2>/dev/null); then
			printf 'AE-5 sink volume became unparseable during the run\n' \
				> "$evidence_root/safety-anomaly.txt"
		elif ! number_is_at_most "$volume" 0.20; then
			printf 'AE-5 sink volume exceeded 20%% during the run\n' \
				> "$evidence_root/safety-anomaly.txt"
		else
			sleep 0.2
			continue
		fi
		hard_mute >/dev/null 2>&1 || true
		kill -TERM "$parent_pid" 2>/dev/null || true
		return
	done
}

start_tone_stream() {
	local duration=$1 label=$2

	{
		sox -q -n -r "$sample_rate" -b 16 -e signed-integer -c 2 \
			-t raw - synth "$duration" sine "$tone_hz" \
			gain "$tone_level_db" |
			pw-play --target "$sink_name" --latency 2048 \
				--rate "$sample_rate" --channels 2 --format s16 \
				--raw --volume 1 -
	} > "$evidence_root/$label-client.log" 2>&1 &
	stream_pid=$!
}

run_measurement() {
	local label=$1 duration=$2 iterations=$3 pipewire_pid
	local before after cpu stats

	start_tone_stream "$duration" "$label"
	sleep 1
	kill -0 "$stream_pid" 2>/dev/null ||
		fail "$label stream exited during startup"
	pipewire_pid=$(pgrep -x pipewire)
	[[ $pipewire_pid =~ ^[0-9]+$ ]] ||
		fail 'expected one PipeWire process'
	before=$(cpu_snapshot "$pipewire_pid")
	LC_ALL=C pw-top -b -n "$iterations" > "$evidence_root/$label-pw-top.log"
	after=$(cpu_snapshot "$pipewire_pid")
	cpu=$(cpu_percent "$before" "$after")
	if ! wait "$stream_pid"; then
		stream_pid=
		fail "$label stream failed"
	fi
	stream_pid=
	[[ ! -s $evidence_root/$label-client.log ]] ||
		fail "$label stream wrote diagnostics"
	stats=$(top_stats "$evidence_root/$label-pw-top.log" "$sink_id") ||
		fail "$label PipeWire timing data is incomplete"
	printf '%s\npipewire_cpu_percent=%s\n' "$stats" "$cpu" \
		> "$evidence_root/$label-stats.txt"
	printf '%s: CPU=%s%% busy=%sus/%sus errors=%s\n' \
		"$label" "$cpu" \
		"$(snapshot_value busy_mean_us "$stats")" \
		"$(snapshot_value busy_max_us "$stats")" \
		"$(snapshot_value errors "$stats")"
}

cleanup() {
	local status=$? cleanup_failed=false

	trap - EXIT INT TERM
	if [[ -n $stream_pid ]]; then
		kill "$stream_pid" >/dev/null 2>&1 || true
		wait "$stream_pid" >/dev/null 2>&1 || true
	fi
	if [[ -n $watchdog_pid ]]; then
		kill "$watchdog_pid" >/dev/null 2>&1 || true
		wait "$watchdog_pid" >/dev/null 2>&1 || true
	fi
	hard_mute >/dev/null 2>&1 || cleanup_failed=true
	if [[ $eq_owned == true ]]; then
		"$ae5ctl" eq-chain-disable >/dev/null 2>&1 || cleanup_failed=true
	fi
	if [[ -n $sink_id ]]; then
		wpctl set-mute "$sink_id" 1 >/dev/null 2>&1 || cleanup_failed=true
		wpctl set-volume "$sink_id" "$original_volume" >/dev/null 2>&1 ||
			cleanup_failed=true
	fi
	if [[ -n $master_state ]]; then
		restore_switch Master "$master_state" >/dev/null 2>&1 ||
			cleanup_failed=true
	fi
	if [[ -n $front_state ]]; then
		restore_switch Front "$front_state" >/dev/null 2>&1 ||
			cleanup_failed=true
	fi
	if [[ -n $sink_id ]]; then
		wpctl set-mute "$sink_id" "$original_mute" >/dev/null 2>&1 ||
			cleanup_failed=true
	fi
	if [[ -n $card_index ]] && ! wait_for_pcm_close; then
		cleanup_failed=true
	fi
	if [[ -n $evidence_root ]]; then
		LC_ALL=C amixer -c "$card_index" contents \
			> "$evidence_root/mixer-after.txt" 2>/dev/null ||
			cleanup_failed=true
		cmp -s "$evidence_root/mixer-before.txt" \
			"$evidence_root/mixer-after.txt" || cleanup_failed=true
		[[ $(sink_snapshot 2>/dev/null) == \
			"Volume: $original_volume$([[ $original_mute == 1 ]] && printf ' [MUTED]')" ]] ||
			cleanup_failed=true
		[[ $("$ae5ctl" eq-chain-status 2>/dev/null | sed -n '1p') == \
			'PipeWire in-place software equalizer: not configured' ]] ||
			cleanup_failed=true
		"$ae5ctl" route-status > "$evidence_root/route-after.txt" 2>&1 ||
			cleanup_failed=true
		[[ $(node_snapshot 2>/dev/null) == \
			"$sink_id"$'\t'"$sink_serial"$'\t'"$sink_name" ]] ||
			cleanup_failed=true
	fi
	if [[ $cleanup_failed == true ]]; then
		status=1
	fi
	if [[ -n $evidence_root ]]; then
		if [[ $status -eq 0 && $matrix_passed == true ]]; then
			printf 'result=pass\nrecovery=pass\nsoak_seconds=%s\nqualification=%s\n' \
				"$soak_seconds" "$qualification" > "$evidence_root/result.txt"
		else
			printf 'result=fail\nrecovery=%s\n' \
				"$([[ $cleanup_failed == false ]] && printf pass || printf fail)" \
				> "$evidence_root/result.txt"
		fi
		printf 'evidence=%s\n' "$evidence_root"
	fi
	exit "$status"
}

self_test() (
	local baseline equalized comparison temporary

	temporary=$(mktemp -d "${TMPDIR:-/tmp}/ae5-eq-performance-test.XXXXXX")
	trap 'find "$temporary" -depth -delete' EXIT
	baseline="$temporary/baseline.log"
	equalized="$temporary/equalized.log"
	printf '%s\n' \
		'R 62 2048 48000 10.0us 20.0us 0.00 0.00 0 S16LE 2 48000 sink' \
		'R 62 2048 48000 11.0us 22.0us 0.00 0.00 0 S16LE 2 48000 sink' \
		'R 62 2048 48000 9.0us 21.0us 0.00 0.00 0 S16LE 2 48000 sink' \
		> "$baseline"
	printf '%s\n' \
		'R 62 2048 48000 10.0us 220.0us 0.00 0.00 0 S16LE 2 48000 sink' \
		'R 62 2048 48000 11.0us 222.0us 0.00 0.00 0 S16LE 2 48000 sink' \
		'R 62 2048 48000 9.0us 221.0us 0.00 0.00 0 S16LE 2 48000 sink' \
		> "$equalized"
	[[ $(snapshot_value busy_mean_us "$(top_stats "$baseline" 62)") == 21.000 ]]
	comparison=$(compare_benchmarks \
		"$(top_stats "$baseline" 62)" \
		"$(top_stats "$equalized" 62)")
	[[ $(snapshot_value added_pipewire_buffer_frames "$comparison") == 0 ]]
	[[ $(snapshot_value added_busy_us "$comparison") == 200.000 ]]
	[[ $(snapshot_value added_busy_percent_of_quantum "$comparison") == 0.4687 ]]
	sed -i 's/2048 48000/1024 48000/' "$equalized"
	if compare_benchmarks \
		"$(top_stats "$baseline" 62)" \
		"$(top_stats "$equalized" 62)" >/dev/null 2>&1; then
		fail 'mismatched scheduling geometry passed'
	fi
	printf 'software EQ performance self-test passed\n'
)

main() {
	local dry_run=$1 profile=$2 requested_soak_seconds=$3 node before_node
	local active_node
	local sink_state mixer_before mixer_after baseline_stats equalized_stats
	local comparison baseline_cpu equalized_cpu cpu_delta start_epoch cache_root
	local kernel_log user_log relevant_log

	[[ ${AE5_ANALOG_OUTPUTS_UNPLUGGED:-} == 1 ]] ||
		fail 'set AE5_ANALOG_OUTPUTS_UNPLUGGED=1 only after physically verifying every AE-5 analog output is unplugged'
	[[ -f $profile ]] || fail "profile is not a regular file: $profile"
	[[ $requested_soak_seconds =~ ^[0-9]+$ &&
		$requested_soak_seconds -ge 60 && $requested_soak_seconds -le 14400 ]] ||
		fail 'SOAK_SECONDS must be between 60 and 14400'
	soak_seconds=$requested_soak_seconds
	if ((soak_seconds >= long_duration_seconds)); then
		qualification=long-duration
	fi
	[[ $benchmark_seconds =~ ^[0-9]+$ &&
		$benchmark_seconds -ge 10 && $benchmark_seconds -le 120 ]] ||
		fail 'AE5_EQ_BENCHMARK_SECONDS must be between 10 and 120'
	[[ $top_iterations =~ ^[0-9]+$ &&
		$top_iterations -ge 5 && $top_iterations -lt $benchmark_seconds ]] ||
		fail 'AE5_EQ_TOP_ITERATIONS must be at least 5 and shorter than the benchmark'
	for command in amixer awk cmp getconf jq journalctl pactl pgrep pw-dump \
		pw-play pw-top sed sox sort wpctl; do
		require_command "$command"
	done
	command -v "$ae5ctl" >/dev/null 2>&1 ||
		fail "ae5ctl is unavailable: $ae5ctl"
	[[ $("$ae5ctl" eq-chain-status | sed -n '1p') == \
		'PipeWire in-place software equalizer: not configured' ]] ||
		fail 'disable the existing managed software EQ before this test'

	card_index=$("$ae5ctl" status |
		sed -n 's/^  ALSA card: \([0-9][0-9]*\) .*/\1/p')
	[[ $card_index =~ ^[0-9]+$ ]] ||
		fail 'unable to resolve the exact AE-5 ALSA card'
	node=$(node_snapshot)
	[[ -n $node ]] || fail 'unable to resolve one exact AE-5 PipeWire sink'
	IFS=$'\t' read -r sink_id sink_serial sink_name <<< "$node"
	[[ $sink_id =~ ^[0-9]+$ && $sink_serial =~ ^[0-9]+$ &&
		-n $sink_name ]] || fail 'AE-5 PipeWire sink identity is invalid'
	require_safe_state
	[[ -z $(pactl list short sink-inputs) ]] ||
		fail 'close all playback applications before this test'
	pcm_is_closed || fail 'AE-5 playback PCM must be closed before this test'
	sink_state=$(sink_snapshot)
	original_volume=$(sink_volume "$sink_state")
	original_mute=$(sink_mute "$sink_state")
	master_state=$(playback_switch_state Master) ||
		fail 'Master switch state is ambiguous'
	front_state=$(playback_switch_state Front) ||
		fail 'Front switch state is ambiguous'

	printf 'card=%s sink=%s serial=%s profile=%s soak=%ss\n' \
		"$card_index" "$sink_name" "$sink_serial" "$profile" "$soak_seconds"
	printf 'safety: analog outputs acknowledged unplugged; sink=%s; Master/Front will be hard-muted\n' \
		"$sink_state"
	if [[ $dry_run == true ]]; then
		return
	fi

	cache_root=${XDG_CACHE_HOME:-"$HOME/.cache"}/ae5-control
	mkdir -p "$cache_root"
	evidence_root=$(mktemp -d \
		"$cache_root/eq-performance-$(date +%Y%m%d-%H%M%S).XXXXXX")
	chmod 0700 "$evidence_root"
	trap cleanup EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM
	before_node=$node
	mixer_before=$(LC_ALL=C amixer -c "$card_index" contents 2>/dev/null)
	printf '%s\n' "$mixer_before" > "$evidence_root/mixer-before.txt"
	pw-dump > "$evidence_root/pw-dump-before.json"
	start_epoch=$(date +%s)

	hard_mute
	[[ $(playback_switch_state Master) == off &&
		$(playback_switch_state Front) == off ]] ||
		fail 'hardware mute did not read back'
	wpctl set-volume "$sink_id" 5%
	[[ $(sink_volume "$(sink_snapshot)") == 0.05 ]] ||
		fail 'AE-5 sink did not read back at 5%'
	watch_safety "$$" &
	watchdog_pid=$!
	wpctl set-mute "$sink_id" 0

	run_measurement baseline "$benchmark_seconds" "$top_iterations"
	wpctl set-mute "$sink_id" 1
	"$ae5ctl" eq-chain-enable "$profile"
	eq_owned=true
	"$ae5ctl" eq-chain-activate
	active_node=$(node_snapshot)
	[[ $active_node == "$before_node" ]] ||
		fail 'software EQ changed the physical sink identity'
	"$ae5ctl" eq-chain-status > "$evidence_root/eq-active.txt"
	grep -Fq 'Runtime graph: current' "$evidence_root/eq-active.txt" ||
		fail 'software EQ runtime signature is not current'
	pw-dump > "$evidence_root/pw-dump-active.json"

	wpctl set-mute "$sink_id" 0
	run_measurement equalized "$benchmark_seconds" "$top_iterations"
	baseline_stats=$(< "$evidence_root/baseline-stats.txt")
	equalized_stats=$(< "$evidence_root/equalized-stats.txt")
	comparison=$(compare_benchmarks "$baseline_stats" "$equalized_stats") ||
		fail 'software EQ changed scheduling geometry, reported errors, or exceeded its busy-time budget'
	baseline_cpu=$(snapshot_value pipewire_cpu_percent "$baseline_stats")
	equalized_cpu=$(snapshot_value pipewire_cpu_percent "$equalized_stats")
	cpu_delta=$(awk -v baseline="$baseline_cpu" -v equalized="$equalized_cpu" \
		'BEGIN {
			delta = equalized - baseline
			printf "%.4f", (delta > 0 ? delta : 0)
		}')
	number_is_at_most "$cpu_delta" "$cpu_budget_percent" ||
		fail "software EQ added $cpu_delta% PipeWire CPU, above the $cpu_budget_percent% budget"
	printf '%s\n' "$comparison" > "$evidence_root/comparison.txt"
	printf 'pipewire_cpu_delta_percent=%s\n' "$cpu_delta" \
		>> "$evidence_root/comparison.txt"
	printf 'latency: same node and quantum; 0 added PipeWire buffer frames\n'
	printf 'CPU: +%s percentage points; busy: +%s us (%s%% of quantum)\n' \
		"$cpu_delta" \
		"$(snapshot_value added_busy_us "$comparison")" \
		"$(snapshot_value added_busy_percent_of_quantum "$comparison")"

	printf 'starting %ss nonzero software-EQ soak at 5%% with hardware hard-muted\n' \
		"$soak_seconds"
	run_measurement soak "$soak_seconds" "$((soak_seconds - 2))"
	[[ $(snapshot_value errors "$(< "$evidence_root/soak-stats.txt")") == 0 ]] ||
		fail 'PipeWire reported an error during the soak'
	[[ $(node_snapshot) == "$before_node" ]] ||
		fail 'physical sink identity changed during the soak'
	"$ae5ctl" eq-chain-status > "$evidence_root/eq-after-soak.txt"
	grep -Fq 'Runtime graph: current' "$evidence_root/eq-after-soak.txt" ||
		fail 'software EQ runtime signature changed during the soak'
	wpctl set-mute "$sink_id" 1

	kernel_log="$evidence_root/kernel-since-start.log"
	user_log="$evidence_root/pipewire-since-start.log"
	relevant_log="$evidence_root/relevant-warnings.log"
	journalctl -k -b --since "@$start_epoch" --no-pager > "$kernel_log" \
		2>/dev/null || true
	journalctl --user --since "@$start_epoch" --no-pager \
		-u pipewire.service -u wireplumber.service > "$user_log" \
		2>/dev/null || true
	grep -Ei 'ca0132|snd_hda|xrun|underrun|overrun|filter-graph|assertion.*failed' \
		"$kernel_log" "$user_log" > "$relevant_log" || true
	[[ ! -s $relevant_log ]] ||
		fail 'relevant kernel or PipeWire warnings appeared during the soak'

	"$ae5ctl" eq-chain-disable
	eq_owned=false
	[[ $(node_snapshot) == "$before_node" ]] ||
		fail 'physical sink identity changed after EQ cleanup'
	mixer_after=$(LC_ALL=C amixer -c "$card_index" contents 2>/dev/null)
	printf '%s\n' "$mixer_after" > "$evidence_root/mixer-before-recovery.txt"
	matrix_passed=true
	printf 'software EQ performance and %s soak passed\n' "$qualification"
}

case ${1:-} in
--self-test)
	[[ $# == 1 ]] || usage
	self_test
	;;
--dry-run)
	[[ $# == 2 || $# == 3 ]] || usage
	main true "$2" "${3:-"$default_soak_seconds"}"
	;;
-h | --help)
	usage
	;;
"")
	usage
	;;
*)
	[[ $# == 1 || $# == 2 ]] || usage
	main false "$1" "${2:-"$default_soak_seconds"}"
	;;
esac
