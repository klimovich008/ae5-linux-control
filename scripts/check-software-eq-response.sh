#!/usr/bin/env bash
# Measure the in-place software EQ through the AE-5 What U Hear PCM.
set -euo pipefail

readonly default_rates='44100 48000 96000'
readonly rates_spec=${AE5_EQ_RATES:-$default_rates}
readonly test_volume_percent=${AE5_TEST_VOLUME_PERCENT:-20}
readonly sync_threshold=${AE5_EQ_SYNC_THRESHOLD:-0.005%}
readonly capture_seconds=29
readonly ae5ctl=${AE5CTL:-ae5ctl}
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
readonly script_dir
readonly parity=${AE5_AUDIO_PARITY:-"$script_dir/audio-parity.sh"}

card_index=
sink_id=
sink_name=
sink_serial=
original_volume=
original_mute=
original_force_rate=
original_clock_rate=
original_default_sink=
master_state=
front_state=
evidence_root=
temporary_root=
capture_pid=
playback_pid=
watchdog_pid=
current_rate=
eq_owned=false
matrix_passed=false
declare -a rates=()
declare -a profiles=()

usage() {
	cat >&2 <<'EOF'
usage:
  check-software-eq-response.sh PROFILE.json [PROFILE.json ...]
  check-software-eq-response.sh --dry-run PROFILE.json [PROFILE.json ...]
  check-software-eq-response.sh --self-test

Live runs require AE5_ANALOG_OUTPUTS_UNPLUGGED=1 after physically checking
that every AE-5 analog output is unplugged. AE5_EQ_RATES may contain a
space-separated subset of 44100, 48000, and 96000. AE5_TEST_VOLUME_PERCENT
defaults to 20 and must be between 1 and 20.
AE5_EQ_SYNC_THRESHOLD defaults to the measured What U Hear threshold 0.005%.
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

number_is_at_most() {
	awk -v actual="$1" -v limit="$2" \
		'BEGIN { exit !(actual + 0 <= limit + 0) }'
}

metadata_value() {
	local key=$1 snapshot=$2 values

	values=$(sed -n \
		"s/.*key:'$key' value:'\\([^']*\\)'.*/\\1/p" <<< "$snapshot")
	[[ $(wc -l <<< "$values") -eq 1 && -n $values ]] || return 1
	printf '%s\n' "$values"
}

settings_snapshot() {
	LC_ALL=C pw-metadata -n settings 0
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

test_pcms_are_closed() {
	local status

	for status in "/proc/asound/card$card_index"/pcm0p/sub*/status \
		"/proc/asound/card$card_index"/pcm1p/sub*/status \
		"/proc/asound/card$card_index"/pcm2c/sub*/status; do
		[[ -r $status ]] || return 1
		grep -Fqx closed "$status" || return 1
	done
}

playback_pcms_are_closed() {
	local status

	for status in "/proc/asound/card$card_index"/pcm0p/sub*/status \
		"/proc/asound/card$card_index"/pcm1p/sub*/status; do
		[[ -r $status ]] || return 1
		grep -Fqx closed "$status" || return 1
	done
}

capture_pcm_is_closed() {
	local status

	for status in "/proc/asound/card$card_index"/pcm2c/sub*/status; do
		[[ -r $status ]] || return 1
		grep -Fqx closed "$status" || return 1
	done
}

wait_for_pcm_close() {
	for _ in {1..100}; do
		test_pcms_are_closed && return
		sleep 0.1
	done
	return 1
}

wait_for_capture_pcm_close() {
	for _ in {1..100}; do
		capture_pcm_is_closed && return
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

default_sink_name() {
	wpctl inspect @DEFAULT_AUDIO_SINK@ |
		sed -n 's/^[[:space:]]*\* node.name = "\(.*\)"$/\1/p'
}

validate_profile() {
	jq -e '
		.format_version == 1 and
		(.name | type == "string" and length > 0) and
		([
			range(0; 10) as $band |
			.controls["EQ Band\($band)"].playback_level
		] |
			length == 10 and
			all(.[];
				type == "number" and
				floor == . and
				. >= 0 and
				. <= 48
			)
		)
	' "$1" >/dev/null
}

parse_rates() {
	local rate seen=' '

	read -r -a rates <<< "$rates_spec"
	((${#rates[@]} > 0)) || fail 'AE5_EQ_RATES is empty'
	for rate in "${rates[@]}"; do
		case $rate in
		44100 | 48000 | 96000) ;;
		*) fail "unsupported AE5_EQ_RATES entry: $rate" ;;
		esac
		[[ $seen != *" $rate "* ]] ||
			fail "duplicate AE5_EQ_RATES entry: $rate"
		seen+="$rate "
	done
}

verify_hw_rate() {
	local path=$1 expected=$2

	[[ -r $path ]] || return 1
	awk -v expected="$expected" \
		'$1 == "rate:" && $2 == expected { found = 1 }
		END { exit !found }' "$path"
}

verify_pw_top_rate() {
	local path=$1 node_id=$2 expected=$3

	[[ -r $path ]] || return 1
	LC_ALL=C awk -v node_id="$node_id" -v expected="$expected" \
		'$1 == "R" && $2 == node_id && $4 == expected { found = 1 }
		END { exit !found }' "$path"
}

set_force_rate() {
	local requested=$1 expected_clock=${2:-} snapshot force clock

	pw-metadata -n settings 0 clock.force-rate "$requested" >/dev/null
	for _ in {1..100}; do
		snapshot=$(settings_snapshot)
		force=$(metadata_value clock.force-rate "$snapshot") || {
			sleep 0.1
			continue
		}
		clock=$(metadata_value clock.rate "$snapshot") || {
			sleep 0.1
			continue
		}
		[[ $force == "$requested" &&
			(-z $expected_clock || $clock == "$expected_clock") ]] && return
		sleep 0.1
	done
	return 1
}

require_safe_state() {
	local snapshot volume gain pcm_info

	[[ $(< /proc/sys/kernel/tainted) == 0 ]] ||
		fail 'the running kernel is tainted'
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
	pcm_info=$(< "/proc/asound/card$card_index/pcm2c/info")
	grep -Fqx 'id: CA0132 What U Hear' <<< "$pcm_info" ||
		fail 'card capture device 2 is not CA0132 What U Hear'
}

watch_safety() {
	local parent_pid=$1 snapshot volume force

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
		elif ! number_is_at_most "$volume" \
			"$(awk -v percent="$test_volume_percent" \
				'BEGIN { print percent / 100 }')"; then
			printf 'AE-5 sink volume exceeded the requested test cap\n' \
				> "$evidence_root/safety-anomaly.txt"
		elif ! force=$(metadata_value clock.force-rate \
			"$(settings_snapshot 2>/dev/null)" 2>/dev/null); then
			printf 'PipeWire force-rate became unreadable during the run\n' \
				> "$evidence_root/safety-anomaly.txt"
		elif [[ -n $current_rate && $force != "$current_rate" ]]; then
			printf 'PipeWire force-rate changed during the run\n' \
				> "$evidence_root/safety-anomaly.txt"
		else
			sleep 0.2
			continue
		fi
		hard_mute >/dev/null 2>&1 || true
		wpctl set-mute "$sink_id" 1 >/dev/null 2>&1 || true
		kill -TERM "$parent_pid" 2>/dev/null || true
		return
	done
}

capture_fixture() {
	local label=$1 fixture=$2 output=$3 output_root=$4 playback_log
	local capture_log playback_hw capture_hw active_settings top_log

	[[ ! -e $output ]] || fail "refusing to overwrite capture: $output"
	capture_log="$output_root/$label-capture.log"
	playback_log="$output_root/$label-playback.log"
	playback_hw="$output_root/$label-playback-hw-params.txt"
	capture_hw="$output_root/$label-capture-hw-params.txt"
	active_settings="$output_root/$label-settings-active.txt"
	top_log="$output_root/$label-pw-top.txt"
	[[ -z $(pactl list short sink-inputs) ]] ||
		fail 'a playback application opened before the capture'

	arecord -q -D "hw:$card_index,2" -f S32_LE -r "$current_rate" \
		-c 2 -d "$capture_seconds" -t wav "$output" \
		> "$capture_log" 2>&1 &
	capture_pid=$!
	sleep 0.5
	pw-play --target "$sink_name" --volume 1 "$fixture" \
		> "$playback_log" 2>&1 &
	playback_pid=$!
	sleep 1
	kill -0 "$capture_pid" 2>/dev/null ||
		fail "$label What U Hear capture exited during startup"
	kill -0 "$playback_pid" 2>/dev/null ||
		fail "$label playback exited during startup"
	cp "/proc/asound/card$card_index/pcm0p/sub0/hw_params" "$playback_hw"
	cp "/proc/asound/card$card_index/pcm2c/sub0/hw_params" "$capture_hw"
	verify_hw_rate "$playback_hw" "$current_rate" ||
		fail "$label AE-5 playback PCM did not negotiate $current_rate Hz"
	verify_hw_rate "$capture_hw" "$current_rate" ||
		fail "$label What U Hear PCM did not negotiate $current_rate Hz"
	settings_snapshot > "$active_settings"
	[[ $(metadata_value clock.force-rate \
		"$(< "$active_settings")") == "$current_rate" ]] ||
		fail "$label PipeWire force-rate changed during playback"
	LC_ALL=C pw-top -b -n 2 > "$top_log"
	verify_pw_top_rate "$top_log" "$sink_id" "$current_rate" ||
		fail "$label PipeWire sink did not run at $current_rate Hz"
	if ! wait "$playback_pid"; then
		playback_pid=
		fail "$label playback failed"
	fi
	playback_pid=
	if ! wait "$capture_pid"; then
		capture_pid=
		fail "$label What U Hear capture failed"
	fi
	capture_pid=
	[[ ! -s $capture_log ]] || fail "$label capture wrote diagnostics"
	[[ ! -s $playback_log ]] || fail "$label playback wrote diagnostics"
	[[ $(soxi -r "$output") == "$current_rate" ]] ||
		fail "$label capture has the wrong sample rate"
	[[ $(soxi -c "$output") == 2 ]] ||
		fail "$label capture is not stereo"
	wait_for_capture_pcm_close ||
		fail "$label What U Hear PCM did not close"
}

validate_signal() {
	local capture=$1 analysis=$2 level

	AE5_SAMPLE_RATE=$current_rate AE5_SYNC_THRESHOLD=$sync_threshold \
		bash "$parity" \
		analyze-tones "$capture" > "$analysis"
	level=$(awk -F '\t' '$1 == 1000 { print $2 }' "$analysis")
	[[ $level =~ ^-?[0-9]+([.][0-9]+)?$ ]] ||
		fail "1 kHz signal is missing from $(basename "$capture")"
	awk -v level="$level" 'BEGIN { exit !(level > -100 && level < -6) }' ||
		fail "1 kHz signal is outside the -100..-6 dBFS gate in $(basename "$capture")"
}

disable_owned_eq() {
	[[ $eq_owned == true ]] || return
	"$ae5ctl" eq-chain-disable >/dev/null
	eq_owned=false
	[[ $("$ae5ctl" eq-chain-status | sed -n '1p') == \
		'PipeWire in-place software equalizer: not configured' ]] ||
		fail 'software EQ did not disable cleanly'
}

cleanup() {
	local status=$? cleanup_failed=false snapshot expected_sink

	trap - EXIT INT TERM
	for pid in "$playback_pid" "$capture_pid" "$watchdog_pid"; do
		[[ -n $pid ]] || continue
		kill "$pid" >/dev/null 2>&1 || true
		wait "$pid" >/dev/null 2>&1 || true
	done
	hard_mute >/dev/null 2>&1 || cleanup_failed=true
	if [[ $eq_owned == true ]]; then
		"$ae5ctl" eq-chain-disable >/dev/null 2>&1 || cleanup_failed=true
		eq_owned=false
	fi
	if [[ -n $sink_id ]]; then
		wpctl set-mute "$sink_id" 1 >/dev/null 2>&1 || cleanup_failed=true
	fi
	if [[ -n $original_force_rate ]]; then
		current_rate=
		set_force_rate "$original_clock_rate" "$original_clock_rate" ||
			cleanup_failed=true
	fi
	if [[ -n $sink_name ]]; then
		pactl suspend-sink "$sink_name" 1 >/dev/null 2>&1 ||
			cleanup_failed=true
		for _ in {1..100}; do
			playback_pcms_are_closed && break
			sleep 0.1
		done
		playback_pcms_are_closed || cleanup_failed=true
	fi
	if [[ -n $original_force_rate ]]; then
		set_force_rate "$original_force_rate" ||
			cleanup_failed=true
	fi
	if [[ -n $sink_id && -n $original_volume ]]; then
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
	if [[ -n $sink_id && -n $original_mute ]]; then
		wpctl set-mute "$sink_id" "$original_mute" >/dev/null 2>&1 ||
			cleanup_failed=true
	fi
	if [[ -n $sink_name ]]; then
		pactl suspend-sink "$sink_name" 0 >/dev/null 2>&1 ||
			cleanup_failed=true
		sleep 0.2
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
		snapshot=$(sink_snapshot 2>/dev/null) || cleanup_failed=true
		expected_sink="Volume: $original_volume"
		[[ $original_mute == 1 ]] && expected_sink+=' [MUTED]'
		[[ $snapshot == "$expected_sink" ]] || cleanup_failed=true
		[[ $("$ae5ctl" eq-chain-status 2>/dev/null | sed -n '1p') == \
			'PipeWire in-place software equalizer: not configured' ]] ||
			cleanup_failed=true
		[[ $(node_snapshot 2>/dev/null) == \
			"$sink_id"$'\t'"$sink_serial"$'\t'"$sink_name" ]] ||
			cleanup_failed=true
		[[ $(default_sink_name 2>/dev/null) == "$original_default_sink" ]] ||
			cleanup_failed=true
		"$ae5ctl" route-status > "$evidence_root/route-after.txt" 2>&1 ||
			cleanup_failed=true
		settings_snapshot > "$evidence_root/settings-after.txt" 2>&1 ||
			cleanup_failed=true
		[[ $(metadata_value clock.force-rate \
			"$(< "$evidence_root/settings-after.txt")" 2>/dev/null) == \
			"$original_force_rate" ]] || cleanup_failed=true
		[[ $(metadata_value clock.rate \
			"$(< "$evidence_root/settings-after.txt")" 2>/dev/null) == \
			"$original_clock_rate" ]] || cleanup_failed=true
	fi
	if [[ -n $temporary_root && -d $temporary_root ]]; then
		find "$temporary_root" -depth -delete >/dev/null 2>&1 ||
			cleanup_failed=true
	fi
	[[ $cleanup_failed == false ]] || status=1
	if [[ -n $evidence_root ]]; then
		if [[ $status -eq 0 && $matrix_passed == true ]]; then
			printf 'result=pass\nrecovery=pass\nrates=%s\nprofiles=%s\nvolume_percent=%s\n' \
				"${rates[*]}" "${#profiles[@]}" "$test_volume_percent" \
				> "$evidence_root/result.txt"
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
	local test_root profile metadata

	test_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-eq-response-test.XXXXXX")
	trap 'find "$test_root" -depth -delete' EXIT
	metadata=$'update: id:0 key:\x27clock.rate\x27 value:\x2748000\x27 type:\x27\x27\nupdate: id:0 key:\x27clock.force-rate\x27 value:\x270\x27 type:\x27\x27'
	[[ $(metadata_value clock.rate "$metadata") == 48000 ]]
	[[ $(metadata_value clock.force-rate "$metadata") == 0 ]]
	if metadata_value missing "$metadata" >/dev/null 2>&1; then
		fail 'missing metadata key passed'
	fi
	printf '%s\n' 'access: RW_INTERLEAVED' 'format: S32_LE' 'rate: 96000 (96000/1)' \
		> "$test_root/hw-params"
	verify_hw_rate "$test_root/hw-params" 96000
	if verify_hw_rate "$test_root/hw-params" 48000; then
		fail 'mismatched hardware rate passed'
	fi
	printf '%s\n' \
		'S   ID  QUANT   RATE    WAIT    BUSY   W/Q   B/Q  ERR FORMAT           NAME' \
		'R   62   2048  96000   5.0us   8.0us  0.00  0.00    0    S16LE 2 96000 ae5' \
		> "$test_root/pw-top"
	verify_pw_top_rate "$test_root/pw-top" 62 96000
	if verify_pw_top_rate "$test_root/pw-top" 62 48000; then
		fail 'mismatched PipeWire graph rate passed'
	fi
	if verify_pw_top_rate "$test_root/pw-top" 63 96000; then
		fail 'mismatched PipeWire node passed'
	fi
	profile="$test_root/profile.json"
	jq -n '
		{
			format_version: 1,
			name: "Self test",
			target: "1102:0012/1102:0051",
			controls: (
				[range(0; 10) as $band |
					{key: "EQ Band\($band)", value: {playback_level: 24}}
				] | from_entries
			)
		}
	' > "$profile"
	validate_profile "$profile"
	jq '.controls["EQ Band9"].playback_level = 49' \
		"$profile" > "$test_root/invalid-profile.json"
	if validate_profile "$test_root/invalid-profile.json"; then
		fail 'out-of-range EQ profile passed'
	fi
	printf 'software EQ response self-test passed\n'
)

main() {
	local dry_run=$1 node sink_state settings cache_root start_epoch command
	local rate rate_root fixture_root fixture neutral_a neutral_b
	local profile profile_index profile_name profile_root expected equalized
	local neutral_comparison response maximum_error kernel_log user_log
	local relevant_log

	[[ ${AE5_ANALOG_OUTPUTS_UNPLUGGED:-} == 1 ]] ||
		fail 'set AE5_ANALOG_OUTPUTS_UNPLUGGED=1 only after physically verifying every AE-5 analog output is unplugged'
	[[ $test_volume_percent =~ ^[0-9]+$ &&
		$test_volume_percent -ge 1 && $test_volume_percent -le 20 ]] ||
		fail 'AE5_TEST_VOLUME_PERCENT must be between 1 and 20'
	parse_rates
	((${#profiles[@]} > 0)) || usage
	for profile in "${profiles[@]}"; do
		[[ -f $profile ]] || fail "profile is not a regular file: $profile"
		validate_profile "$profile" ||
			fail "profile does not contain ten valid EQ bands: $profile"
	done
	for command in amixer arecord awk cmp cp jq journalctl pactl pw-dump \
		pw-metadata pw-play pw-top sed sha256sum sox soxi sort wpctl; do
		require_command "$command"
	done
	command -v "$ae5ctl" >/dev/null 2>&1 ||
		fail "ae5ctl is unavailable: $ae5ctl"
	[[ -x $parity ]] || fail "audio parity helper is unavailable: $parity"
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
	test_pcms_are_closed || fail 'AE-5 test PCMs must be closed before this test'
	sink_state=$(sink_snapshot)
	original_volume=$(sink_volume "$sink_state")
	original_mute=$(sink_mute "$sink_state")
	master_state=$(playback_switch_state Master) ||
		fail 'Master switch state is ambiguous'
	front_state=$(playback_switch_state Front) ||
		fail 'Front switch state is ambiguous'
	settings=$(settings_snapshot)
	original_force_rate=$(metadata_value clock.force-rate "$settings") ||
		fail 'PipeWire force-rate is unreadable'
	original_clock_rate=$(metadata_value clock.rate "$settings") ||
		fail 'PipeWire clock rate is unreadable'
	original_default_sink=$(default_sink_name)
	[[ -n $original_default_sink ]] ||
		fail 'the current default PipeWire sink is unreadable'

	printf 'card=%s sink=%s serial=%s rates=%s profiles=%s\n' \
		"$card_index" "$sink_name" "$sink_serial" \
		"${rates[*]}" "${#profiles[@]}"
	printf 'safety: analog outputs acknowledged unplugged; sink=%s; test cap=%s%%; Master/Front will be hard-muted\n' \
		"$sink_state" "$test_volume_percent"
	if [[ $dry_run == true ]]; then
		return
	fi

	cache_root=${XDG_CACHE_HOME:-"$HOME/.cache"}/ae5-control
	mkdir -p "$cache_root"
	evidence_root=$(mktemp -d \
		"$cache_root/eq-response-$(date +%Y%m%d-%H%M%S).XXXXXX")
	chmod 0700 "$evidence_root"
	temporary_root=$(mktemp -d \
		"$cache_root/eq-response-work-$(date +%Y%m%d-%H%M%S).XXXXXX")
	chmod 0700 "$temporary_root"
	trap cleanup EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM
	LC_ALL=C amixer -c "$card_index" contents \
		> "$evidence_root/mixer-before.txt" 2>/dev/null
	printf '%s\n' "$settings" > "$evidence_root/settings-before.txt"
	"$ae5ctl" route-status > "$evidence_root/route-before.txt"
	pw-dump > "$evidence_root/pw-dump-before.json"
	printf 'rate_hz\tprofile\tmax_eq_error_db\tresult\n' \
		> "$evidence_root/matrix.tsv"
	start_epoch=$(date +%s)

	hard_mute
	[[ $(playback_switch_state Master) == off &&
		$(playback_switch_state Front) == off ]] ||
		fail 'hardware mute did not read back'
	wpctl set-volume "$sink_id" "$test_volume_percent%"
	[[ $(sink_volume "$(sink_snapshot)") == \
		"$(awk -v percent="$test_volume_percent" \
			'BEGIN { printf "%.2f", percent / 100 }')" ]] ||
		fail 'AE-5 sink did not read back at the requested test cap'
	wpctl set-mute "$sink_id" 0
	watch_safety "$$" &
	watchdog_pid=$!

	for rate in "${rates[@]}"; do
		current_rate=
		set_force_rate "$rate" ||
			fail "PipeWire did not switch to $rate Hz"
		current_rate=$rate
		rate_root="$evidence_root/$rate"
		fixture_root="$temporary_root/$rate"
		mkdir -p "$rate_root" "$fixture_root"
		AE5_SAMPLE_RATE=$rate bash "$parity" generate "$fixture_root" \
			> "$rate_root/fixture-generation.txt"
		fixture="$fixture_root/parity-tones.wav"
		sha256sum "$fixture" > "$rate_root/fixture.sha256"
		settings_snapshot > "$rate_root/settings-active.txt"

		neutral_a="$rate_root/neutral-a.wav"
		neutral_b="$rate_root/neutral-b.wav"
		capture_fixture neutral-a "$fixture" "$neutral_a" "$rate_root"
		validate_signal "$neutral_a" "$rate_root/neutral-a-analysis.txt"
		capture_fixture neutral-b "$fixture" "$neutral_b" "$rate_root"
		validate_signal "$neutral_b" "$rate_root/neutral-b-analysis.txt"
		neutral_comparison="$rate_root/neutral-repeatability.txt"
		AE5_SAMPLE_RATE=$rate AE5_SYNC_THRESHOLD=$sync_threshold \
			bash "$parity" \
			compare-tones "$neutral_a" "$neutral_b" \
			> "$neutral_comparison" ||
			fail "$rate Hz neutral captures were not repeatable"
		grep -Fqx 'parity_result=pass' "$neutral_comparison" ||
			fail "$rate Hz neutral repeatability did not pass"

		profile_index=0
		for profile in "${profiles[@]}"; do
			((profile_index += 1))
			profile_name=$(jq -r .name "$profile")
			profile_root=$(printf '%s/profile-%02d' \
				"$rate_root" "$profile_index")
			mkdir -p "$profile_root"
			printf 'name=%s\nsource_sha256=%s\n' \
				"$profile_name" "$(sha256sum "$profile" | cut -d' ' -f1)" \
				> "$profile_root/profile.txt"
			eq_owned=true
			"$ae5ctl" eq-chain-enable "$profile" \
				> "$profile_root/eq-enable.txt"
			"$ae5ctl" eq-chain-activate \
				> "$profile_root/eq-activate.txt"
			[[ $(node_snapshot) == "$node" ]] ||
				fail "$profile_name changed the physical sink identity"
			"$ae5ctl" eq-chain-status > "$profile_root/eq-status.txt"
			grep -Fq 'Runtime graph: current' "$profile_root/eq-status.txt" ||
				fail "$profile_name runtime graph is not current"
			expected="$profile_root/expected.tsv"
			"$ae5ctl" eq-chain-response "$rate" > "$expected"
			equalized="$profile_root/equalized.wav"
			capture_fixture equalized "$fixture" "$equalized" "$profile_root"
			validate_signal "$equalized" "$profile_root/equalized-analysis.txt"
			response="$profile_root/response.txt"
			AE5_SAMPLE_RATE=$rate AE5_SYNC_THRESHOLD=$sync_threshold \
				bash "$parity" \
				compare-eq "$expected" "$neutral_a" "$equalized" \
				> "$response" ||
				fail "$profile_name response missed the 1 dB gate at $rate Hz"
			grep -Fqx 'eq_response_result=pass' "$response" ||
				fail "$profile_name response did not pass at $rate Hz"
			maximum_error=$(sed -n \
				's/^max_eq_error_db=//p' "$response")
			printf '%s\t%s\t%s\tpass\n' \
				"$rate" "$profile_name" "$maximum_error" \
				>> "$evidence_root/matrix.tsv"
			disable_owned_eq
			[[ $(node_snapshot) == "$node" ]] ||
				fail 'physical sink identity changed after EQ cleanup'
		done
	done

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
		fail 'relevant kernel or PipeWire warnings appeared during the matrix'
	matrix_passed=true
	printf 'software EQ response matrix passed at %s Hz for %s profiles\n' \
		"${rates[*]}" "${#profiles[@]}"
}

case ${1:-} in
--self-test)
	[[ $# == 1 ]] || usage
	self_test
	;;
--dry-run)
	shift
	(($# > 0)) || usage
	profiles=("$@")
	main true
	;;
-h | --help)
	usage
	;;
"")
	usage
	;;
*)
	profiles=("$@")
	main false
	;;
esac
