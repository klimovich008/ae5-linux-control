#!/usr/bin/env bash
set -euo pipefail

readonly sample_rate=${AE5_SAMPLE_RATE:-48000}
readonly bit_depth=24
readonly channels=2
readonly sync_threshold=${AE5_SYNC_THRESHOLD:-1%}
readonly maximum_fixture_peak_db=-14
readonly -a frequencies=(31 62 125 250 500 1000 2000 4000 8000 16000)

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
  audio-parity.sh analyze-tones CAPTURE.wav
  audio-parity.sh compare-tones WINDOWS.wav LINUX.wav
  audio-parity.sh analyze-noise CAPTURE.wav
  audio-parity.sh compare-noise WINDOWS.wav LINUX.wav
  audio-parity.sh --self-test

Set AE5_SYNC_THRESHOLD to override the default 1% marker threshold.
Set AE5_SAMPLE_RATE to 44100, 48000, or 96000; the default is 48000.
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
	local duration=$1 output=$2

	sox -n -r "$sample_rate" -b "$bit_depth" -c "$channels" \
		"$output" trim 0 "$duration"
}

make_tone() {
	local duration=$1 frequency=$2 level=$3 output=$4

	sox -n -r "$sample_rate" -b "$bit_depth" -c "$channels" \
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

generate() (
	local output_root=$1 temporary_root
	local -a outputs=(
		"$output_root/parity-tones.wav"
		"$output_root/parity-sweep.wav"
		"$output_root/parity-level-steps.wav"
		"$output_root/parity-silence.wav"
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
	for output in "${outputs[@]:0:4}"; do
		validate_fixture_peak "$output"
	done

	(
		cd -- "$output_root"
		sha256sum \
			parity-tones.wav \
			parity-sweep.wav \
			parity-level-steps.wav \
			parity-silence.wav > SHA256SUMS
	)

	printf 'generated %s Hz, 24-bit stereo fixtures in %s\n' \
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
	[[ $channel_count == "$channels" ]] || {
		printf 'error: expected %s channels, got %s: %s\n' \
			"$channels" "$channel_count" "$input" >&2
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
	printf 'frequency_hz\trms_dbfs\tleft_dbfs\tright_dbfs\trelative_to_1khz_db\n'
	awk -F '\t' -v reference="$reference" '{
		printf "%s\t%.2f\t%.2f\t%.2f\t%+.2f\n",
			$1, $2, $3, $4, $2 - reference
	}' <<< "$levels"
}

compare_tones() (
	local windows=$1 linux=$2 temporary_root
	local windows_levels linux_levels windows_reference linux_reference

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

noise_level() {
	local input=$1 stats

	validate_capture "$input" || return
	stats=$(sox "$input" -n stats 2>&1) || return
	awk '
		$1 == "RMS" && $2 == "lev" && $3 == "dB" {
			printf "%s\t%s\t%s\n", $4, $5, $6
		}
	' <<< "$stats"
}

analyze_noise() {
	local input=$1 levels

	levels=$(noise_level "$input") || return
	printf 'capture=%s\n' "$input"
	printf 'rms_dbfs\tleft_dbfs\tright_dbfs\n'
	printf '%s\n' "$levels"
}

compare_noise() {
	local windows=$1 linux=$2 windows_level linux_level

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
	local test_root before_hash after_hash mismatch unsafe_fixture wrong_rate

	test_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-audio-parity-test.XXXXXX")
	trap 'rm -rf -- "$test_root"' EXIT

	generate "$test_root/fixtures" >/dev/null
	(
		cd -- "$test_root/fixtures"
		sha256sum -c SHA256SUMS >/dev/null
	)
	analyze_tones "$test_root/fixtures/parity-tones.wav" >/dev/null
	compare_tones \
		"$test_root/fixtures/parity-tones.wav" \
		"$test_root/fixtures/parity-tones.wav" >/dev/null
	compare_noise \
		"$test_root/fixtures/parity-silence.wav" \
		"$test_root/fixtures/parity-silence.wav" >/dev/null

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
