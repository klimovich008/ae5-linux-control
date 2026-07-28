#!/usr/bin/env bash
set -euo pipefail

readonly sample_rate=${AE5_SAMPLE_RATE:-48000}
readonly bit_depth=24
readonly channels=2
readonly sync_threshold=${AE5_SYNC_THRESHOLD:-1%}
readonly maximum_fixture_peak_db=-14
readonly ae5ctl=${AE5CTL:-ae5ctl}
readonly -a frequencies=(31 62 125 250 500 1000 2000 4000 8000 16000)
readonly -a playback_volume_controls=(Master Front Surround Center LFE PCM)

case $sample_rate in
44100 | 48000 | 96000) ;;
*)
	printf 'error: AE5_SAMPLE_RATE must be 44100, 48000, or 96000\n' >&2
	exit 2
	;;
esac

usage() {
	cat >&2 <<'EOF'
usage:
  audio-parity.sh generate OUTPUT_DIRECTORY
  audio-parity.sh generate-transitions OUTPUT_DIRECTORY
  audio-parity.sh analyze-tones CAPTURE.wav
  audio-parity.sh compare-tones WINDOWS.wav LINUX.wav
  audio-parity.sh compare-eq EXPECTED.tsv NEUTRAL.wav EQ.wav
  audio-parity.sh analyze-noise CAPTURE.wav
  audio-parity.sh compare-noise WINDOWS.wav LINUX.wav
  audio-parity.sh playback-preflight direct|pipewire FIXTURE.wav
  audio-parity.sh --self-test

Set AE5_SYNC_THRESHOLD to override the default 1% marker threshold.
Set AE5_SAMPLE_RATE to 44100, 48000, or 96000; the default is 48000.
Set AE5CTL to the ae5ctl executable when it is not installed in PATH.
EOF
}

need_tool() {
	command -v "$1" >/dev/null 2>&1 || {
		printf 'error: required tool is unavailable: %s\n' "$1" >&2
		exit 1
	}
}

new_output() {
	[[ ! -e $1 ]] || {
		printf 'error: refusing to overwrite %s\n' "$1" >&2
		exit 1
	}
}

make_silence() {
	local duration=$1 output=$2 channel_count=${3:-$channels}

	sox -n -r "$sample_rate" -b "$bit_depth" -c "$channel_count" \
		"$output" trim 0 "$duration"
}

make_tone() {
	local duration=$1 frequency=$2 level=$3 output=$4
	local channel_count=${5:-$channels}

	sox -n -r "$sample_rate" -b "$bit_depth" -c "$channel_count" \
		"$output" synth "$duration" sine "$frequency" \
		gain "$level" fade t 0.05 "$duration" 0.05
}

validate_fixture_peak() {
	local input=$1 peak

	peak=$(sox "$input" -n stats 2>&1 |
		awk '$1 == "Pk" && $2 == "lev" && $3 == "dB" { print $4 }')
	[[ -n $peak ]] || {
		printf 'error: unable to measure fixture peak: %s\n' "$input" >&2
		return 1
	}
	[[ $peak == -inf ]] && return
	awk -v peak="$peak" -v maximum="$maximum_fixture_peak_db" \
		'BEGIN { exit !(peak <= maximum) }' || {
		printf 'error: fixture peak %s dBFS exceeds the %s dBFS safety ceiling: %s\n' \
			"$peak" "$maximum_fixture_peak_db" "$input" >&2
		return 1
	}
}

validate_playback_volume_value() {
	local control=$1 channel=$2 level=$3 minimum=$4 maximum=$5 percent

	((maximum > minimum)) || {
		printf 'error: invalid AE-5 %s range [%s..%s]\n' \
			"$control" "$minimum" "$maximum" >&2
		return 1
	}
	((level >= minimum && level <= maximum)) || {
		printf 'error: AE-5 %s%s level %s is outside [%s..%s]\n' \
			"$control" "$channel" "$level" "$minimum" "$maximum" >&2
		return 1
	}
	percent=$(awk -v level="$level" -v minimum="$minimum" \
		-v maximum="$maximum" \
		'BEGIN { printf "%.1f", (level - minimum) * 100 / (maximum - minimum) }')
	awk -v level="$level" -v minimum="$minimum" -v maximum="$maximum" \
		'BEGIN { exit !((level - minimum) * 100 <= (maximum - minimum) * 20) }' ||
		{
			printf 'error: AE-5 %s%s is %s%%; maximum test level is 20%%\n' \
				"$control" "$channel" "$percent" >&2
			return 1
		}
}

validate_playback_volume_snapshot() {
	local control=$1 snapshot=$2 values level minimum maximum
	local channel_list entry channel_value

	[[ $snapshot == "$control |"* ]] || {
		printf 'error: unable to identify the AE-5 %s control\n' "$control" >&2
		return 1
	}
	values=$(sed -n \
		's/.*playback level \([-0-9][0-9]*\) \[\([-0-9][0-9]*\)\.\.\([-0-9][0-9]*\)\].*/\1 \2 \3/p' \
		<<< "$snapshot")
	[[ $(wc -w <<< "$values") -eq 3 ]] || {
		printf 'error: unable to parse the AE-5 %s level\n' "$control" >&2
		return 1
	}
	read -r level minimum maximum <<< "$values"
	validate_playback_volume_value "$control" "" \
		"$level" "$minimum" "$maximum" || return

	channel_list=$(sed -n \
		's/.* | playback \([^|]*=[-0-9][^|]*\)\( |.*\)\?$/\1/p' \
		<<< "$snapshot")
	[[ -z $channel_list ]] && return
	while IFS= read -r entry; do
		entry=${entry# }
		channel_value=${entry##*=}
		[[ $channel_value =~ ^-?[0-9]+$ ]] || {
			printf 'error: unable to parse the AE-5 %s channel levels\n' \
				"$control" >&2
			return 1
		}
		validate_playback_volume_value "$control" \
			" ${entry%=*}" "$channel_value" "$minimum" "$maximum" || return
	done < <(tr ',' '\n' <<< "$channel_list")
}

validate_pipewire_playback_volume_snapshot() {
	local control=$1 snapshot=$2

	case "$control|$snapshot" in
	'Master|Master | playback on | playback level 99 [0..99]' | \
	'Front|Front | playback on | playback level 90 [0..99] | playback Front Left=90, Front Right=90' | \
	'PCM|PCM | playback level 255 [0..255] | playback Front Left=255, Front Right=255')
		return
		;;
	esac
	validate_playback_volume_snapshot "$control" "$snapshot"
}

validate_gain_snapshot() {
	[[ $1 == 'AE-5: Headphone Gain: Low ('* ]] || {
		printf 'error: AE-5 headphone gain must be Low for playback tests\n' >&2
		return 1
	}
}

validate_default_output_snapshot() {
	grep -Fq ' [default]' <<< "$1" || {
		printf 'error: the AE-5 must be the default PipeWire output for this preflight\n' >&2
		return 1
	}
}

validate_pipewire_volume_snapshot() {
	local snapshot=$1 volume

	volume=$(awk '$1 == "Volume:" && $2 ~ /^[0-9]+([.][0-9]+)?$/ {
		print $2
	}' <<< "$snapshot")
	[[ $(wc -w <<< "$volume") -eq 1 ]] || {
		printf 'error: unable to parse the PipeWire output volume\n' >&2
		return 1
	}
	awk -v volume="$volume" 'BEGIN { exit !(volume <= 0.20) }' || {
		printf 'error: PipeWire output is %.0f%%; maximum test level is 20%%\n' \
			"$(awk -v volume="$volume" 'BEGIN { print volume * 100 }')" >&2
		return 1
	}
}

playback_preflight() {
	local mode=$1 fixture=$2 control snapshot failed=0

	validate_fixture_peak "$fixture" || failed=1
	need_tool "$ae5ctl"
	for control in "${playback_volume_controls[@]}"; do
		if snapshot=$("$ae5ctl" get "$control"); then
			if [[ $mode == pipewire ]]; then
				validate_pipewire_playback_volume_snapshot \
					"$control" "$snapshot" || failed=1
			else
				validate_playback_volume_snapshot \
					"$control" "$snapshot" || failed=1
			fi
		else
			failed=1
		fi
	done
	if snapshot=$("$ae5ctl" get 'AE-5: Headphone Gain'); then
		validate_gain_snapshot "$snapshot" || failed=1
	else
		failed=1
	fi

	if [[ $mode == pipewire ]]; then
		need_tool wpctl
		"$ae5ctl" route-status >/dev/null || failed=1
		if snapshot=$("$ae5ctl" output-status); then
			validate_default_output_snapshot "$snapshot" || failed=1
		else
			failed=1
		fi
		if snapshot=$(wpctl get-volume @DEFAULT_AUDIO_SINK@); then
			validate_pipewire_volume_snapshot "$snapshot" || failed=1
		else
			failed=1
		fi
	fi

	((failed == 0)) || return 1
	if [[ $mode == pipewire ]]; then
		printf 'playback preflight passed: PipeWire at or below 20%%, fixed 0 dB hardware stages or legacy-safe attenuation, Low gain\n'
	else
		printf 'playback preflight passed: direct path, fixture and hardware volumes at or below 20%%, Low gain\n'
	fi
}

make_marker_and_gap() {
	local temporary_root=$1

	make_tone 0.25 997 -18 "$temporary_root/marker.wav"
	make_silence 0.75 "$temporary_root/marker-gap.wav"
}

generate_tones() {
	local temporary_root=$1 output=$2 frequency
	local -a segments=(
		"$temporary_root/marker.wav"
		"$temporary_root/marker-gap.wav"
	)

	for frequency in "${frequencies[@]}"; do
		make_tone 2 "$frequency" -18 \
			"$temporary_root/tone-$frequency.wav"
		make_silence 0.5 "$temporary_root/gap-$frequency.wav"
		segments+=(
			"$temporary_root/tone-$frequency.wav"
			"$temporary_root/gap-$frequency.wav"
		)
	done

	sox "${segments[@]}" "$output"
}

generate_sweep() {
	local temporary_root=$1 output=$2

	sox -n -r "$sample_rate" -b "$bit_depth" -c "$channels" \
		"$temporary_root/sweep-body.wav" \
		synth 15 sine 20/20000 gain -18 fade t 0.05 15 0.05
	sox \
		"$temporary_root/marker.wav" \
		"$temporary_root/marker-gap.wav" \
		"$temporary_root/sweep-body.wav" \
		"$output"
}

generate_level_steps() {
	local temporary_root=$1 output=$2 level
	local -a segments=(
		"$temporary_root/marker.wav"
		"$temporary_root/marker-gap.wav"
	)

	for level in -36 -30 -24 -18; do
		make_tone 2 1000 "$level" \
			"$temporary_root/level-${level#-}.wav"
		make_silence 0.5 "$temporary_root/level-gap-${level#-}.wav"
		segments+=(
			"$temporary_root/level-${level#-}.wav"
			"$temporary_root/level-gap-${level#-}.wav"
		)
	done

	sox "${segments[@]}" "$output"
}

generate_channel_identity() {
	local temporary_root=$1 output=$2 active channel
	local -a inputs segments=()

	make_tone 1 997 -18 "$temporary_root/channel-tone.wav" 1
	make_tone 1 80 -18 "$temporary_root/channel-lfe.wav" 1
	make_silence 1 "$temporary_root/channel-silence.wav" 1
	make_silence 0.5 "$temporary_root/channel-gap.wav" 6
	for ((active = 0; active < 6; active++)); do
		inputs=()
		for ((channel = 0; channel < 6; channel++)); do
			if ((channel == active)); then
				if ((active == 3)); then
					inputs+=("$temporary_root/channel-lfe.wav")
				else
					inputs+=("$temporary_root/channel-tone.wav")
				fi
			else
				inputs+=("$temporary_root/channel-silence.wav")
			fi
		done
		sox -M "${inputs[@]}" "$temporary_root/channel-$active.wav"
		segments+=(
			"$temporary_root/channel-$active.wav"
			"$temporary_root/channel-gap.wav"
		)
	done
	sox "${segments[@]}" "$output"
}

generate_transition_tone() {
	local rate=$1 depth=$2 frequency=$3 output=$4

	sox -n -r "$rate" -b "$depth" -e signed-integer -c "$channels" \
		"$output" synth 1.5 sine "$frequency" \
		gain -18 fade t 0.05 1.5 0.05
}

generate_transitions() (
	local output_root=$1 output
	local -a outputs=(
		"$output_root/transition-a-44100-s16.wav"
		"$output_root/transition-b-48000-s16.wav"
		"$output_root/transition-c-48000-s32.wav"
		"$output_root/transition-d-96000-s32.wav"
		"$output_root/SHA256SUMS"
	)

	mkdir -p -- "$output_root"
	for output in "${outputs[@]}"; do
		new_output "$output"
	done

	generate_transition_tone 44100 16 523 \
		"$output_root/transition-a-44100-s16.wav"
	generate_transition_tone 48000 16 997 \
		"$output_root/transition-b-48000-s16.wav"
	generate_transition_tone 48000 32 1301 \
		"$output_root/transition-c-48000-s32.wav"
	generate_transition_tone 96000 32 1999 \
		"$output_root/transition-d-96000-s32.wav"
	for output in "${outputs[@]:0:4}"; do
		validate_fixture_peak "$output"
	done
	(
		cd -- "$output_root"
		sha256sum transition-*.wav > SHA256SUMS
	)

	printf 'generated four transition fixtures in %s\n' "$output_root"
)

channel_peak() {
	local input=$1 start=$2 channel=$3

	sox "$input" -n trim "$start" 0.8 remix "$channel" stats 2>&1 |
		awk '$1 == "Pk" && $2 == "lev" && $3 == "dB" { print $4 }'
}

channel_frequency() {
	local input=$1 start=$2 channel=$3

	sox "$input" -n trim "$start" 0.8 remix "$channel" stat 2>&1 |
		awk -F: '/^Rough *frequency:/ { gsub(/ /, "", $2); print $2 }'
}

validate_channel_identity() {
	local input=$1 channel_count duration active channel start peak expected

	channel_count=$(soxi -c "$input")
	duration=$(soxi -D "$input")
	[[ $channel_count == 6 ]] || {
		printf 'error: channel fixture must have 6 channels: %s\n' "$input" >&2
		return 1
	}
	awk -v duration="$duration" 'BEGIN { exit !(duration == 9) }' || {
		printf 'error: channel fixture must be 9 seconds: %s\n' "$input" >&2
		return 1
	}
	for ((active = 1; active <= 6; active++)); do
		start=$(awk -v active="$active" \
			'BEGIN { print ((active - 1) * 1.5) + 0.1 }')
		for ((channel = 1; channel <= 6; channel++)); do
			peak=$(channel_peak "$input" "$start" "$channel")
			expected=-inf
			((channel == active)) && expected=-18.00
			[[ $peak == "$expected" ]] || {
				printf 'error: channel fixture routing sequence is invalid: %s\n' \
					"$input" >&2
				return 1
			}
		done
	done
	[[ $(channel_frequency "$input" 0.1 1) == 996 &&
		$(channel_frequency "$input" 4.6 4) == 79 ]] || {
		printf 'error: channel fixture tone frequencies are invalid: %s\n' "$input" >&2
		return 1
	}
}

generate() (
	local output_root=$1 temporary_root
	local -a outputs=(
		"$output_root/parity-tones.wav"
		"$output_root/parity-sweep.wav"
		"$output_root/parity-level-steps.wav"
		"$output_root/parity-silence.wav"
		"$output_root/parity-channel-id-6ch.wav"
		"$output_root/SHA256SUMS"
	)

	mkdir -p -- "$output_root"
	for output in "${outputs[@]}"; do
		new_output "$output"
	done

	temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-audio-parity.XXXXXX")
	trap 'rm -rf -- "$temporary_root"' EXIT
	make_marker_and_gap "$temporary_root"
	generate_tones "$temporary_root" "$output_root/parity-tones.wav"
	generate_sweep "$temporary_root" "$output_root/parity-sweep.wav"
	generate_level_steps \
		"$temporary_root" "$output_root/parity-level-steps.wav"
	make_silence 15 "$output_root/parity-silence.wav"
	generate_channel_identity \
		"$temporary_root" "$output_root/parity-channel-id-6ch.wav"
	for output in "${outputs[@]:0:5}"; do
		validate_fixture_peak "$output"
	done
	validate_channel_identity "$output_root/parity-channel-id-6ch.wav"

	(
		cd -- "$output_root"
		sha256sum \
			parity-tones.wav \
			parity-sweep.wav \
			parity-level-steps.wav \
			parity-silence.wav \
			parity-channel-id-6ch.wav > SHA256SUMS
	)

	printf 'generated %s Hz, 24-bit stereo and six-channel fixtures in %s\n' \
		"$sample_rate" "$output_root"
	printf 'copy these exact files to Windows; do not regenerate them there\n'
)

validate_capture() {
	local input=$1 rate channel_count

	[[ -f $input ]] || {
		printf 'error: capture is not a regular file: %s\n' "$input" >&2
		return 1
	}
	rate=$(soxi -r "$input") || {
		printf 'error: unable to read capture metadata: %s\n' "$input" >&2
		return 1
	}
	channel_count=$(soxi -c "$input") || {
		printf 'error: unable to read capture metadata: %s\n' "$input" >&2
		return 1
	}
	[[ $rate == "$sample_rate" ]] || {
		printf 'error: expected %s Hz, got %s Hz: %s\n' \
			"$sample_rate" "$rate" "$input" >&2
		return 1
	}
	[[ $channel_count == 1 || $channel_count == "$channels" ]] || {
		printf 'error: expected mono or stereo, got %s channels: %s\n' \
			"$channel_count" "$input" >&2
		return 1
	}
}

validate_matching_capture_channels() {
	local first=$1 second=$2 first_channels second_channels

	first_channels=$(soxi -c "$first") || return
	second_channels=$(soxi -c "$second") || return
	[[ $first_channels == "$second_channels" ]] || {
		printf 'error: capture channel counts differ (%s versus %s): %s, %s\n' \
			"$first_channels" "$second_channels" "$first" "$second" >&2
		return 1
	}
}

align_capture() {
	local input=$1 output=$2 duration

	sox "$input" "$output" silence 1 0.02 "$sync_threshold" || return
	duration=$(soxi -D "$output") || return
	awk -v duration="$duration" 'BEGIN { exit !(duration >= 25.5) }' || {
		printf 'error: sync marker not found or capture is too short: %s\n' \
			"$input" >&2
		return 1
	}
}

tone_levels() (
	local input=$1 temporary_root aligned band_index frequency start stats

	validate_capture "$input" || return
	temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-audio-parity.XXXXXX")
	trap 'rm -rf -- "$temporary_root"' EXIT
	aligned="$temporary_root/aligned.wav"
	align_capture "$input" "$aligned" || return

	for band_index in "${!frequencies[@]}"; do
		frequency=${frequencies[$band_index]}
		start=$(awk -v band_index="$band_index" \
			'BEGIN { printf "%.3f", 1.5 + (band_index * 2.5) }')
		stats=$(sox "$aligned" -n trim "$start" 1 stats 2>&1) || return
		awk -v frequency="$frequency" '
			$1 == "RMS" && $2 == "lev" && $3 == "dB" {
				if (NF == 4) {
					printf "%s\t%s\t%s\tn/a\n",
						frequency, $4, $4
					next
				}
				printf "%s\t%s\t%s\t%s\n",
					frequency, $4, $5, $6
			}
		' <<< "$stats"
	done
)

analyze_tones() {
	local input=$1 levels reference

	levels=$(tone_levels "$input") || return
	reference=$(awk -F '\t' '$1 == 1000 { print $2 }' <<< "$levels")
	[[ -n $reference ]] || {
		printf 'error: 1 kHz measurement is missing: %s\n' "$input" >&2
		return 1
	}

	printf 'capture=%s\n' "$input"
	printf 'sample_rate=%s\nchannels=%s\n' \
		"$(soxi -r "$input")" "$(soxi -c "$input")"
	printf 'sync_threshold=%s\n' "$sync_threshold"
	printf 'frequency_hz\trms_dbfs\tchannel_1_dbfs\tchannel_2_dbfs\trelative_to_1khz_db\n'
	awk -F '\t' -v reference="$reference" '{
		channel_2 = $4 == "n/a" ? "n/a" : sprintf("%.2f", $4)
		printf "%s\t%.2f\t%.2f\t%s\t%+.2f\n",
			$1, $2, $3, channel_2, $2 - reference
	}' <<< "$levels"
}

compare_tones() (
	local windows=$1 linux=$2 temporary_root
	local windows_levels linux_levels windows_reference linux_reference

	validate_matching_capture_channels "$windows" "$linux" || return
	temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-audio-parity.XXXXXX")
	trap 'rm -rf -- "$temporary_root"' EXIT
	windows_levels="$temporary_root/windows.tsv"
	linux_levels="$temporary_root/linux.tsv"
	tone_levels "$windows" > "$windows_levels" || return
	tone_levels "$linux" > "$linux_levels" || return

	windows_reference=$(awk -F '\t' '$1 == 1000 { print $2 }' "$windows_levels")
	linux_reference=$(awk -F '\t' '$1 == 1000 { print $2 }' "$linux_levels")

	printf 'windows_capture=%s\nlinux_capture=%s\n' "$windows" "$linux"
	printf 'frequency_hz\twindows_dbfs\tlinux_dbfs\tlevel_delta_db\tresponse_delta_db\n'
	awk -F '\t' \
		-v windows_reference="$windows_reference" \
		-v linux_reference="$linux_reference" '
		NR == FNR {
			windows[$1] = $2
			next
		}
		{
			level_delta = $2 - windows[$1]
			response_delta = ($2 - linux_reference) - \
				(windows[$1] - windows_reference)
			absolute_response = response_delta < 0 ? \
				-response_delta : response_delta
			if (absolute_response > maximum_response)
				maximum_response = absolute_response
			printf "%s\t%.2f\t%.2f\t%+.2f\t%+.2f\n",
				$1, windows[$1], $2, level_delta, response_delta
		}
		END {
			level_delta = linux_reference - windows_reference
			absolute_level = level_delta < 0 ? -level_delta : level_delta
			printf "level_delta_1khz_db=%+.2f\n", level_delta
			printf "max_response_delta_db=%.2f\n", maximum_response
			if (absolute_level <= 0.5 && maximum_response <= 1.0) {
				print "parity_result=pass"
				exit 0
			}
			print "parity_result=investigate"
			exit 1
		}
	' "$windows_levels" "$linux_levels"
)

compare_eq() (
	local expected=$1 neutral=$2 equalized=$3 temporary_root
	local neutral_levels equalized_levels

	[[ -f $expected ]] || {
		printf 'error: expected EQ response is not a regular file: %s\n' \
			"$expected" >&2
		return 1
	}
	validate_matching_capture_channels "$neutral" "$equalized" || return
	temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-audio-parity.XXXXXX")
	trap 'rm -rf -- "$temporary_root"' EXIT
	neutral_levels="$temporary_root/neutral.tsv"
	equalized_levels="$temporary_root/equalized.tsv"
	tone_levels "$neutral" > "$neutral_levels" || return
	tone_levels "$equalized" > "$equalized_levels" || return

	printf 'expected_response=%s\nneutral_capture=%s\nequalized_capture=%s\n' \
		"$expected" "$neutral" "$equalized"
	printf 'frequency_hz\texpected_delta_db\tmeasured_delta_db\terror_db\n'
	awk -F '\t' '
		FNR == 1 { file_index++ }
		file_index == 1 && $1 ~ /^[0-9]+$/ {
			expected[$1] = $2
			expected_count++
			next
		}
		file_index == 2 {
			neutral[$1] = $2
			next
		}
		file_index == 3 {
			if (!($1 in expected) || !($1 in neutral))
				exit 2
			measured = $2 - neutral[$1]
			error = measured - expected[$1]
			absolute_error = error < 0 ? -error : error
			if (absolute_error > maximum_error)
				maximum_error = absolute_error
			measured_count++
			printf "%s\t%+.2f\t%+.2f\t%+.2f\n",
				$1, expected[$1], measured, error
		}
		END {
			if (expected_count != 10 || measured_count != 10)
				exit 2
			printf "max_eq_error_db=%.2f\n", maximum_error
			if (maximum_error <= 1.0) {
				print "eq_response_result=pass"
				exit 0
			}
			print "eq_response_result=investigate"
			exit 1
		}
	' "$expected" "$neutral_levels" "$equalized_levels"
)

noise_level() {
	local input=$1 stats

	validate_capture "$input" || return
	stats=$(sox "$input" -n stats 2>&1) || return
	awk '
		$1 == "RMS" && $2 == "lev" && $3 == "dB" {
			if (NF == 4) {
				printf "%s\t%s\tn/a\n", $4, $4
				next
			}
			printf "%s\t%s\t%s\n", $4, $5, $6
		}
	' <<< "$stats"
}

analyze_noise() {
	local input=$1 levels

	levels=$(noise_level "$input") || return
	printf 'capture=%s\n' "$input"
	printf 'rms_dbfs\tchannel_1_dbfs\tchannel_2_dbfs\n'
	printf '%s\n' "$levels"
}

compare_noise() {
	local windows=$1 linux=$2 windows_level linux_level

	validate_matching_capture_channels "$windows" "$linux" || return
	windows_level=$(noise_level "$windows" | cut -f1) || return
	linux_level=$(noise_level "$linux" | cut -f1) || return
	if [[ $windows_level == -inf && $linux_level == -inf ]]; then
		printf 'windows_noise_rms_dbfs=-inf\n'
		printf 'linux_noise_rms_dbfs=-inf\n'
		printf 'noise_delta_db=+0.00\n'
		printf 'noise_result=pass\n'
		return
	fi
	if [[ $windows_level == -inf || $linux_level == -inf ]]; then
		printf 'windows_noise_rms_dbfs=%s\n' "$windows_level"
		printf 'linux_noise_rms_dbfs=%s\n' "$linux_level"
		printf 'noise_delta_db=unbounded\n'
		printf 'noise_result=investigate\n'
		return 1
	fi
	awk -v windows="$windows_level" -v linux="$linux_level" '
		BEGIN {
			delta = linux - windows
			absolute = delta < 0 ? -delta : delta
			printf "windows_noise_rms_dbfs=%.2f\n", windows
			printf "linux_noise_rms_dbfs=%.2f\n", linux
			printf "noise_delta_db=%+.2f\n", delta
			if (absolute <= 3.0) {
				print "noise_result=pass"
				exit 0
			}
			print "noise_result=investigate"
			exit 1
		}
	'
}

self_test() (
	local test_root before_hash after_hash mismatch mono unsafe_fixture wrong_rate
	local invalid_channels flat_response mismatched_response

	test_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-audio-parity-test.XXXXXX")
	trap 'rm -rf -- "$test_root"' EXIT

	generate "$test_root/fixtures" >/dev/null
	generate_transitions "$test_root/transitions" >/dev/null
	(
		cd -- "$test_root/fixtures"
		sha256sum -c SHA256SUMS >/dev/null
	)
	(
		cd -- "$test_root/transitions"
		sha256sum -c SHA256SUMS >/dev/null
	)
	[[ $(soxi -r "$test_root/transitions/transition-a-44100-s16.wav") == 44100 &&
		$(soxi -b "$test_root/transitions/transition-a-44100-s16.wav") == 16 &&
		$(soxi -r "$test_root/transitions/transition-d-96000-s32.wav") == 96000 &&
		$(soxi -b "$test_root/transitions/transition-d-96000-s32.wav") == 32 ]] || {
		printf 'self-test: transition fixture format matrix is invalid\n' >&2
		return 1
	}
	analyze_tones "$test_root/fixtures/parity-tones.wav" >/dev/null
	compare_tones \
		"$test_root/fixtures/parity-tones.wav" \
		"$test_root/fixtures/parity-tones.wav" >/dev/null
	flat_response="$test_root/flat-response.tsv"
	printf 'frequency_hz\texpected_delta_db\n' > "$flat_response"
	for frequency in "${frequencies[@]}"; do
		printf '%s\t+0.0000\n' "$frequency"
	done >> "$flat_response"
	compare_eq "$flat_response" \
		"$test_root/fixtures/parity-tones.wav" \
		"$test_root/fixtures/parity-tones.wav" |
		grep -q '^eq_response_result=pass$'
	mismatched_response="$test_root/mismatched-response.tsv"
	sed 's/^31	+0\.0000$/31	+2.0000/' \
		"$flat_response" > "$mismatched_response"
	if compare_eq "$mismatched_response" \
		"$test_root/fixtures/parity-tones.wav" \
		"$test_root/fixtures/parity-tones.wav" >/dev/null 2>&1; then
		printf 'self-test: mismatched expected EQ response unexpectedly passed\n' >&2
		return 1
	fi
	compare_noise \
		"$test_root/fixtures/parity-silence.wav" \
		"$test_root/fixtures/parity-silence.wav" >/dev/null
	mono="$test_root/mono.wav"
	sox "$test_root/fixtures/parity-tones.wav" "$mono" remix -
	analyze_tones "$mono" | grep -q $'^31\t-21.01\t-21.01\tn/a\t+0.00$'
	compare_tones "$mono" "$mono" | grep -q '^parity_result=pass$'
	if compare_tones "$mono" \
		"$test_root/fixtures/parity-tones.wav" >/dev/null 2>&1; then
		printf 'self-test: mismatched capture channels unexpectedly passed\n' >&2
		return 1
	fi
	validate_channel_identity \
		"$test_root/fixtures/parity-channel-id-6ch.wav"
	invalid_channels="$test_root/invalid-channels.wav"
	sox "$test_root/fixtures/parity-channel-id-6ch.wav" \
		"$invalid_channels" remix 1 1 3 4 5 6
	if validate_channel_identity "$invalid_channels" >/dev/null 2>&1; then
		printf 'self-test: invalid channel isolation unexpectedly passed\n' >&2
		return 1
	fi

	make_silence 0.75 "$test_root/leading-silence.wav"
	sox \
		"$test_root/leading-silence.wav" \
		"$test_root/fixtures/parity-tones.wav" \
		"$test_root/delayed.wav"
	compare_tones \
		"$test_root/fixtures/parity-tones.wav" \
		"$test_root/delayed.wav" >/dev/null

	before_hash=$(sha256sum "$test_root/fixtures/parity-tones.wav")
	if generate "$test_root/fixtures" >/dev/null 2>&1; then
		printf 'self-test: existing fixture was overwritten\n' >&2
		return 1
	fi
	after_hash=$(sha256sum "$test_root/fixtures/parity-tones.wav")
	[[ $before_hash == "$after_hash" ]] || {
		printf 'self-test: fixture changed after overwrite rejection\n' >&2
		return 1
	}

	sox \
		"$test_root/fixtures/parity-tones.wav" \
		"$test_root/quieter.wav" gain -1
	if mismatch=$(compare_tones \
		"$test_root/fixtures/parity-tones.wav" \
		"$test_root/quieter.wav"); then
		printf 'self-test: 1 dB mismatch unexpectedly passed\n' >&2
		return 1
	fi
	grep -q '^level_delta_1khz_db=-1.00$' <<< "$mismatch"
	grep -q '^parity_result=investigate$' <<< "$mismatch"

	wrong_rate=44100
	[[ $sample_rate == "$wrong_rate" ]] && wrong_rate=48000
	sox "$test_root/fixtures/parity-tones.wav" \
		-r "$wrong_rate" "$test_root/wrong-rate.wav"
	if analyze_tones "$test_root/wrong-rate.wav" >/dev/null 2>&1; then
		printf 'self-test: wrong-rate capture unexpectedly passed\n' >&2
		return 1
	fi

	unsafe_fixture="$test_root/unsafe.wav"
	make_tone 0.25 997 -6 "$unsafe_fixture"
	if validate_fixture_peak "$unsafe_fixture" >/dev/null 2>&1; then
		printf 'self-test: unsafe fixture peak unexpectedly passed\n' >&2
		return 1
	fi

	validate_playback_volume_snapshot Master \
		'Master | playback on | playback level 20 [0..100]' >/dev/null
	if validate_playback_volume_snapshot Master \
		'Master | playback on | playback level 20 [0..99]' >/dev/null 2>&1; then
		printf 'self-test: Master above 20%% unexpectedly passed\n' >&2
		return 1
	fi
	validate_playback_volume_snapshot Front \
		'Front | playback on | playback level 20 [0..100] | playback Front Left=20, Front Right=20' \
		>/dev/null
	if validate_playback_volume_snapshot Front \
		'Front | playback on | playback level 20 [0..100] | playback Front Left=20, Front Right=21' \
		>/dev/null 2>&1; then
		printf 'self-test: unequal Front channel above 20%% unexpectedly passed\n' >&2
		return 1
	fi
	validate_playback_volume_snapshot PCM \
		'PCM | playback level 51 [0..255] | playback Front Left=51, Front Right=51' \
		>/dev/null
	if validate_playback_volume_snapshot PCM \
		'PCM | playback level 52 [0..255] | playback Front Left=52, Front Right=52' \
		>/dev/null 2>&1; then
		printf 'self-test: PCM above 20%% unexpectedly passed\n' >&2
		return 1
	fi
	if validate_playback_volume_snapshot Front \
		'Master | playback level 10 [0..100]' >/dev/null 2>&1; then
		printf 'self-test: mismatched hardware volume control unexpectedly passed\n' >&2
		return 1
	fi
	if validate_playback_volume_snapshot Master \
		'Master | playback level -1 [0..100]' >/dev/null 2>&1; then
		printf 'self-test: out-of-range hardware volume unexpectedly passed\n' >&2
		return 1
	fi
	validate_pipewire_playback_volume_snapshot Master \
		'Master | playback on | playback level 99 [0..99]' >/dev/null
	validate_pipewire_playback_volume_snapshot Front \
		'Front | playback on | playback level 90 [0..99] | playback Front Left=90, Front Right=90' \
		>/dev/null
	validate_pipewire_playback_volume_snapshot PCM \
		'PCM | playback level 255 [0..255] | playback Front Left=255, Front Right=255' \
		>/dev/null
	if validate_pipewire_playback_volume_snapshot Front \
		'Front | playback on | playback level 91 [0..99] | playback Front Left=91, Front Right=91' \
		>/dev/null 2>&1; then
		printf 'self-test: non-zero-dB PipeWire Front stage unexpectedly passed\n' >&2
		return 1
	fi
	validate_gain_snapshot \
		'AE-5: Headphone Gain: Low (16-31  Ohms)' >/dev/null
	if validate_gain_snapshot \
		'AE-5: Headphone Gain: High (150-600  Ohms)' >/dev/null 2>&1; then
		printf 'self-test: High headphone gain unexpectedly passed\n' >&2
		return 1
	fi
	validate_default_output_snapshot \
		'  PipeWire output: AE-5 (node) [default]' >/dev/null
	if validate_default_output_snapshot \
		'  PipeWire output: AE-5 (node) [not default]' >/dev/null 2>&1; then
		printf 'self-test: non-default AE-5 output unexpectedly passed\n' >&2
		return 1
	fi
	validate_pipewire_volume_snapshot 'Volume: 0.20' >/dev/null
	if validate_pipewire_volume_snapshot \
		'Volume: 0.21' >/dev/null 2>&1; then
		printf 'self-test: PipeWire volume above 20%% unexpectedly passed\n' >&2
		return 1
	fi

	printf 'audio parity self-test passed\n'
)

need_tool sox
need_tool soxi

case ${1:-} in
generate)
	[[ $# -eq 2 ]] || {
		usage
		exit 2
	}
	need_tool sha256sum
	generate "$2"
	;;
generate-transitions)
	[[ $# -eq 2 ]] || {
		usage
		exit 2
	}
	need_tool sha256sum
	generate_transitions "$2"
	;;
analyze-tones)
	[[ $# -eq 2 ]] || {
		usage
		exit 2
	}
	analyze_tones "$2"
	;;
compare-tones)
	[[ $# -eq 3 ]] || {
		usage
		exit 2
	}
	compare_tones "$2" "$3"
	;;
compare-eq)
	[[ $# -eq 4 ]] || {
		usage
		exit 2
	}
	compare_eq "$2" "$3" "$4"
	;;
analyze-noise)
	[[ $# -eq 2 ]] || {
		usage
		exit 2
	}
	analyze_noise "$2"
	;;
compare-noise)
	[[ $# -eq 3 ]] || {
		usage
		exit 2
	}
	compare_noise "$2" "$3"
	;;
playback-preflight)
	[[ $# -eq 3 && ($2 == direct || $2 == pipewire) ]] || {
		usage
		exit 2
	}
	playback_preflight "$2" "$3"
	;;
--self-test)
	[[ $# -eq 1 ]] || {
		usage
		exit 2
	}
	need_tool sha256sum
	self_test
	;;
*)
	usage
	exit 2
	;;
esac
