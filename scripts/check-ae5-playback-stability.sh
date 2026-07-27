#!/usr/bin/env bash
# Fail-closed physical-host acceptance for the stable AE-5 PCM-lifetime fix.
set -euo pipefail

expected_release=7.1.4-ae5-stable
tone_hz=1000
tone_level=0.0316227766
warm_trials=${AE5_STABILITY_WARM_TRIALS:-12}
post_outfx_trials=${AE5_STABILITY_OUTFX_TRIALS:-8}
thd_limit_percent=1
minimum_peak_percent=0.01
headless_mode=${AE5_STABILITY_HEADLESS:-0}
here=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
project_root=$(cd "$here/.." && pwd)
analyzer="$project_root/tools/tone-thd.py"
runtime_gate="$here/check-ae5-kernel-runtime.sh"
evidence_root=
fixture=
card_index=
capture_pid=
services_stopped=false
desktop_audio_was_active=false
matrix_passed=false
declare -a active_audio_units=()

usage() {
	printf 'usage: %s [EXPECTED_RELEASE] | --self-test\n' "$0" >&2
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

analysis_value() {
	snapshot_value "$1" "$2"
}

number_is_at_most() {
	awk -v actual="$1" -v limit="$2" \
		'BEGIN { exit !(actual + 0 <= limit + 0) }'
}

number_is_at_least() {
	awk -v actual="$1" -v limit="$2" \
		'BEGIN { exit !(actual + 0 >= limit + 0) }'
}

find_what_u_hear_device() {
	local card=$1

	LC_ALL=C arecord -l 2>/dev/null |
		awk -v card="$card" '
			$0 ~ ("^card " card ":") && tolower($0) ~ /what u hear/ {
				if (match($0, /device ([0-9]+)/, found)) {
					count++
					device = found[1]
				}
			}
			END {
				if (count != 1)
					exit 1
				print device
			}
		'
}

find_exact_ae5_card() {
	local card_path vendor device subsystem_vendor subsystem_device
	local matched_card=

	shopt -s nullglob
	for card_path in /sys/class/sound/card[0-9]*; do
		[[ -r $card_path/device/vendor ]] || continue
		read -r vendor < "$card_path/device/vendor" || continue
		read -r device < "$card_path/device/device" || continue
		read -r subsystem_vendor < "$card_path/device/subsystem_vendor" ||
			continue
		read -r subsystem_device < "$card_path/device/subsystem_device" ||
			continue
		if [[ ${vendor,,} == 0x1102 &&
			${device,,} == 0x0012 &&
			${subsystem_vendor,,} == 0x1102 &&
			${subsystem_device,,} == 0x0051 ]]; then
			[[ -z $matched_card ]] || return 1
			matched_card=${card_path##*card}
		fi
	done
	[[ -n $matched_card ]] || return 1
	printf '%s\n' "$matched_card"
}

require_control_off() {
	local control=$1 state

	state=$(LC_ALL=C amixer -c "$card_index" sget "$control" 2>/dev/null) ||
		fail "$control is unreadable"
	grep -Eq 'Playback.*\[off\]' <<< "$state" ||
		fail "$control must remain off"
}

hard_mute() {
	[[ -n $card_index ]] || return 0
	LC_ALL=C amixer -q -c "$card_index" sset Master mute >/dev/null
	LC_ALL=C amixer -q -c "$card_index" sset Front mute >/dev/null
}

set_low_headphone_gain() {
	[[ -n $card_index ]] || return 0
	LC_ALL=C amixer -q -c "$card_index" \
		sset 'AE-5: Headphone Gain' 'Low (16-31  Ohms)' >/dev/null
}

enforce_safe_hardware_state() {
	hard_mute
	set_low_headphone_gain
}

start_audio_services() {
	local unit

	[[ $services_stopped == true ]] || return 0
	for unit in \
		pipewire.socket \
		pipewire-pulse.socket \
		pipewire.service \
		pipewire-pulse.service \
		wireplumber.service; do
		if [[ " ${active_audio_units[*]} " == *" $unit "* ]]; then
			systemctl --user start "$unit"
		fi
	done
	services_stopped=false
}

find_ae5_sink_id() {
	local id details

	while read -r id; do
		[[ $id =~ ^[0-9]+$ ]] || continue
		details=$(wpctl inspect "$id" 2>/dev/null) || continue
		grep -Fq "alsa.card = \"$card_index\"" <<< "$details" || continue
		grep -Fq 'alsa.device = "0"' <<< "$details" || continue
		printf '%s\n' "$id"
	done < <(
		wpctl status -n 2>/dev/null |
			awk '
				/Sinks:/ { in_sinks = 1; next }
				in_sinks && /Sources:/ { exit }
				in_sinks && match($0, /[* ]+([0-9]+)\./, found) {
					print found[1]
				}
			'
	)
}

mute_desktop_sink() {
	local sink_id

	[[ $desktop_audio_was_active == true ]] || return 0
	for _ in 1 2 3 4 5; do
		sink_id=$(find_ae5_sink_id | head -n 1) || true
		if [[ -n $sink_id ]]; then
			if wpctl set-volume "$sink_id" 5% >/dev/null 2>&1 &&
				wpctl set-mute "$sink_id" 1 >/dev/null 2>&1; then
				return
			fi
		fi
		sleep 1
	done
	printf 'warning: could not restore the AE-5 desktop sink to 5%% and muted\n' >&2
	return 1
}

cleanup() {
	local status=$?
	local recovery_failed=false
	local recovery_result=incomplete

	trap - EXIT INT TERM
	if [[ -n $capture_pid ]]; then
		kill "$capture_pid" >/dev/null 2>&1 || true
		wait "$capture_pid" >/dev/null 2>&1 || true
	fi
	if ! enforce_safe_hardware_state; then
		printf 'warning: failed to hard-mute the AE-5 before audio-service recovery\n' >&2
		recovery_failed=true
	fi
	if ! start_audio_services; then
		printf 'warning: failed to restore the prior user audio services\n' >&2
		recovery_failed=true
	fi
	if ! mute_desktop_sink; then
		recovery_failed=true
	fi
	if [[ $desktop_audio_was_active == true ]]; then
		sleep 1
	fi
	if ! enforce_safe_hardware_state; then
		printf 'warning: failed to enforce the final AE-5 hardware mute and Low gain\n' >&2
		recovery_failed=true
	fi
	if [[ $recovery_failed == true ]]; then
		status=1
		recovery_result=fail
	fi
	if [[ -n $evidence_root && $status -eq 0 && $matrix_passed == true ]]; then
		printf 'result=pass\nrecovery=pass\n' > "$evidence_root/result.txt"
		printf 'AE-5 stable-playback host acceptance passed\n'
	elif [[ -n $evidence_root ]]; then
		printf 'result=fail\nrecovery=%s\n' "$recovery_result" \
			> "$evidence_root/result.txt"
	fi
	if [[ -n $evidence_root ]]; then
		printf 'evidence=%s\n' "$evidence_root"
	fi
	exit "$status"
}

stop_audio_services() {
	local unit

	if ! systemctl --user show-environment >/dev/null 2>&1; then
		return 0
	fi
	active_audio_units=()
	for unit in \
		pipewire.socket \
		pipewire.service \
		pipewire-pulse.socket \
		pipewire-pulse.service \
		wireplumber.service; do
		if systemctl --user is-active --quiet "$unit"; then
			active_audio_units+=("$unit")
		fi
	done
	if [[ " ${active_audio_units[*]} " == *" pipewire.service "* ||
		" ${active_audio_units[*]} " == *" wireplumber.service "* ]]; then
		desktop_audio_was_active=true
	fi
	services_stopped=true
	systemctl --user stop \
		wireplumber.service \
		pipewire-pulse.service \
		pipewire.service \
		pipewire-pulse.socket \
		pipewire.socket
}

require_safe_controls() {
	local gain

	require_control_off Master
	require_control_off Front
	require_control_off 'Enable OutFX'
	gain=$(LC_ALL=C amixer -c "$card_index" \
		sget 'AE-5: Headphone Gain' 2>/dev/null) ||
		fail 'AE-5 headphone gain is unreadable'
	grep -Fq "Item0: 'Low (16-31  Ohms)'" <<< "$gain" ||
		fail 'AE-5 headphone gain must remain Low'
}

capture_probe() {
	local label=$1 capture analysis thd peak

	capture="$evidence_root/$label.wav"
	LC_ALL=C arecord -q -D "hw:$card_index,$what_u_hear_device" \
		-f S32_LE -c 2 -r 48000 -d 5 "$capture" &
	capture_pid=$!
	sleep 0.3
	if ! LC_ALL=C aplay -q -D "hw:$card_index,0" \
		-f S16_LE -c 2 -r 48000 \
		--period-size=6016 --buffer-size=24064 "$fixture"; then
		kill "$capture_pid" >/dev/null 2>&1 || true
		wait "$capture_pid" >/dev/null 2>&1 || true
		capture_pid=
		fail "$label playback failed"
	fi
	if ! wait "$capture_pid"; then
		capture_pid=
		fail "$label What U Hear capture failed"
	fi
	capture_pid=

	analysis=$("$analyzer" "$capture" --tone-hz "$tone_hz") ||
		fail "$label THD analysis failed"
	printf '%s\n' "$analysis" > "$evidence_root/$label.txt"
	thd=$(analysis_value thd_percent "$analysis") ||
		fail "$label analysis has no unique THD result"
	peak=$(analysis_value signal_peak_percent "$analysis") ||
		fail "$label analysis has no unique signal peak"
	number_is_at_least "$peak" "$minimum_peak_percent" ||
		fail "$label is silent or below the validated capture floor ($peak%)"
	number_is_at_most "$peak" 99 ||
		fail "$label capture clipped ($peak%)"
	number_is_at_most "$thd" "$thd_limit_percent" ||
		fail "$label is corrupt ($thd% THD exceeds $thd_limit_percent%)"
	require_safe_controls
	printf '%-24s peak=%9s%% THD=%11s%% pass\n' "$label" "$peak" "$thd"
}

reject_hardware_outfx() {
	local controls control_line outfx_numid output status

	controls=$(LC_ALL=C amixer -c "$card_index" controls) ||
		fail 'unable to enumerate ALSA controls'
	control_line=$(grep -F \
		"iface=MIXER,name='Enable OutFX Playback Switch'" \
		<<< "$controls") ||
		fail 'Enable OutFX Playback Switch control is missing'
	[[ $(wc -l <<< "$control_line") -eq 1 ]] ||
		fail 'Enable OutFX Playback Switch control is ambiguous'
	outfx_numid=${control_line#numid=}
	outfx_numid=${outfx_numid%%,*}
	[[ $outfx_numid =~ ^[0-9]+$ ]] ||
		fail "Enable OutFX has invalid numid: $outfx_numid"

	set +e
	output=$(LC_ALL=C amixer -c "$card_index" \
		cset "numid=$outfx_numid" 1 2>&1)
	status=$?
	set -e
	printf 'numid=%s\n%s\n' "$outfx_numid" "$output" \
		> "$evidence_root/outfx-enable-rejection.txt"
	(( status != 0 )) || fail 'unsafe hardware OutFX enable unexpectedly succeeded'
	grep -Fq 'Operation not supported' <<< "$output" ||
		fail 'hardware OutFX enable did not return EOPNOTSUPP'
	require_safe_controls
	printf 'hardware OutFX numid=%s enable rejected with EOPNOTSUPP; readback remains off\n' \
		"$outfx_numid"
}

self_test() (
	local temporary_root clean fundamental harmonic distorted analysis thd

	require_command sox
	require_command python3
	python3 -c 'import numpy' 2>/dev/null ||
		fail 'python3 numpy is required'
	[[ -x $analyzer ]] || fail "THD analyzer is not executable: $analyzer"
	temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-stability-test.XXXXXX")
	trap 'find "$temporary_root" -depth -delete' EXIT
	clean="$temporary_root/clean.wav"
	fundamental="$temporary_root/fundamental.wav"
	harmonic="$temporary_root/harmonic.wav"
	distorted="$temporary_root/distorted.wav"

	sox -q -n -r 48000 -c 2 -b 16 "$clean" \
		synth 2 sine "$tone_hz" vol "$tone_level"
	analysis=$("$analyzer" "$clean" --tone-hz "$tone_hz")
	thd=$(analysis_value thd_percent "$analysis")
	number_is_at_most "$thd" 0.1 ||
		fail "clean analyzer fixture measured $thd% THD"

	sox -q -n -r 48000 -c 2 -b 32 "$fundamental" \
		synth 2 sine "$tone_hz" vol "$tone_level"
	sox -q -n -r 48000 -c 2 -b 32 "$harmonic" \
		synth 2 sine "$((tone_hz * 2))" vol 0.0063245553
	sox -q -m "$fundamental" "$harmonic" -b 32 "$distorted"
	analysis=$("$analyzer" "$distorted" --tone-hz "$tone_hz")
	thd=$(analysis_value thd_percent "$analysis")
	number_is_at_least "$thd" 10 ||
		fail "distorted analyzer fixture measured only $thd% THD"

	[[ $(analysis_value thd_percent \
		$'sample_rate=48000\nthd_percent=0.004245000') == 0.004245000 ]] ||
		fail 'analysis parser self-test failed'
	printf 'AE-5 playback stability self-test passed\n'
)

main() {
	local runtime_snapshot cache_root source_analysis source_thd trial
	local headless_card runtime_fixture

	[[ ${AE5_ANALOG_OUTPUTS_UNPLUGGED:-} == 1 ]] ||
		fail 'set AE5_ANALOG_OUTPUTS_UNPLUGGED=1 only after physically verifying every AE-5 analog output is unplugged'
	[[ $headless_mode == 0 || $headless_mode == 1 ]] ||
		fail "invalid AE5_STABILITY_HEADLESS value: $headless_mode"
	if [[ $headless_mode == 1 && $EUID -ne 0 ]]; then
		fail 'AE5_STABILITY_HEADLESS=1 requires root for direct ALSA access'
	fi
	[[ $expected_release =~ ^[A-Za-z0-9][A-Za-z0-9._+-]*$ ]] ||
		usage
	[[ $warm_trials =~ ^[1-9][0-9]*$ && $warm_trials -le 100 ]] ||
		fail "invalid AE5_STABILITY_WARM_TRIALS: $warm_trials"
	[[ $post_outfx_trials =~ ^[1-9][0-9]*$ &&
		$post_outfx_trials -le 100 ]] ||
		fail "invalid AE5_STABILITY_OUTFX_TRIALS: $post_outfx_trials"
	for command in amixer aplay arecord awk grep python3 sox; do
		require_command "$command"
	done
	if [[ $headless_mode == 0 ]]; then
		require_command systemctl
		require_command wpctl
	fi
	python3 -c 'import numpy' 2>/dev/null ||
		fail 'python3 numpy is required'
	[[ -x $analyzer ]] || fail "THD analyzer is not executable: $analyzer"

	if [[ $headless_mode == 1 ]]; then
		headless_card=$(find_exact_ae5_card) ||
			fail 'headless gate did not find one exact AE-5 card'
		runtime_fixture=$(mktemp \
			"${TMPDIR:-/tmp}/ae5-headless-runtime.XXXXXX")
		printf '# Headless AE-5 runtime snapshot\nkernel=%s\nalsa_card=%s\n' \
			"$(uname -r)" "$headless_card" > "$runtime_fixture"
		if ! runtime_snapshot=$(
			AE5_RUNTIME_SNAPSHOT=$runtime_fixture \
				bash "$runtime_gate" "$expected_release"
		); then
			find "$runtime_fixture" -delete
			fail 'headless kernel runtime gate failed'
		fi
		find "$runtime_fixture" -delete
	else
		runtime_snapshot=$(bash "$runtime_gate" "$expected_release")
	fi
	card_index=$(snapshot_value alsa_card "$runtime_snapshot") ||
		fail 'runtime gate did not report one ALSA card'
	[[ $card_index =~ ^[0-9]+$ ]] ||
		fail "runtime gate returned invalid ALSA card $card_index"
	what_u_hear_device=$(find_what_u_hear_device "$card_index") ||
		fail "card $card_index has no unique What U Hear capture PCM"

	cache_root=${XDG_CACHE_HOME:-"$HOME/.cache"}
	mkdir -p "$cache_root/ae5-control"
	evidence_root=$(mktemp -d \
		"$cache_root/ae5-control/host-stability-$(date +%Y%m%d-%H%M%S).XXXXXX")
	chmod 0700 "$evidence_root"
	printf '%s\n' "$runtime_snapshot" > "$evidence_root/runtime-gate.txt"
	journalctl -k -b --no-pager > "$evidence_root/kernel-before.log" 2>/dev/null ||
		true
	trap cleanup EXIT
	trap 'exit 130' INT
	trap 'exit 143' TERM

	enforce_safe_hardware_state
	require_safe_controls
	stop_audio_services
	require_safe_controls

	fixture="$evidence_root/tone-1000hz-minus30dbfs.wav"
	sox -q -n -r 48000 -c 2 -b 16 "$fixture" \
		synth 4 sine "$tone_hz" vol "$tone_level"
	source_analysis=$("$analyzer" "$fixture" --tone-hz "$tone_hz") ||
		fail 'source fixture analysis failed'
	printf '%s\n' "$source_analysis" > "$evidence_root/source-fixture.txt"
	source_thd=$(analysis_value thd_percent "$source_analysis") ||
		fail 'source fixture analysis has no unique THD result'
	number_is_at_most "$source_thd" 0.1 ||
		fail "source fixture is unexpectedly distorted ($source_thd% THD)"

	printf 'kernel=%s card=%s What_U_Hear=hw:%s,%s\n' \
		"$expected_release" "$card_index" "$card_index" "$what_u_hear_device"
	printf 'analog stages are hard-muted; fixture peak is -30 dBFS\n'
	capture_probe first-open
	for (( trial = 1; trial <= warm_trials; trial++ )); do
		capture_probe "warm-$(printf '%02d' "$trial")"
	done
	printf 'idle for 20 seconds with every playback PCM closed\n'
	sleep 20
	capture_probe post-idle-20s
	reject_hardware_outfx
	for (( trial = 1; trial <= post_outfx_trials; trial++ )); do
		capture_probe "post-outfx-$(printf '%02d' "$trial")"
	done

	journalctl -k -b --no-pager > "$evidence_root/kernel-after.log" 2>/dev/null ||
		true
	matrix_passed=true
}

case ${1:-} in
--self-test)
	self_test
	;;
-h | --help)
	usage
	;;
"")
	main
	;;
*)
	expected_release=$1
	main
	;;
esac
