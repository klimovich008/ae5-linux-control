#!/usr/bin/env bash
set -uo pipefail

usage() {
	printf 'usage: %s [--self-test | --summary [required-boots] | --suspend-summary [required-cycles] | --before-suspend CYCLE | --after-resume CYCLE | [label] [--append]]\n' "$0" >&2
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

required_alsa_controls_ready() {
	local card_index=$1

	amixer -c "$card_index" info >/dev/null 2>&1 &&
		amixer -c "$card_index" sget 'Output Select' >/dev/null 2>&1 &&
		amixer -c "$card_index" sget 'Front' >/dev/null 2>&1 &&
		amixer -c "$card_index" sget 'HP/Speaker Auto Detect' >/dev/null 2>&1 &&
		amixer -c "$card_index" \
			cget "iface=CARD,name='Headphone Jack'" >/dev/null 2>&1
}

read_alsa_route_controls() {
	local card_index=$1

	amixer -c "$card_index" sget 'Output Select' &&
		amixer -c "$card_index" sget 'Front' &&
		amixer -c "$card_index" sget 'HP/Speaker Auto Detect' &&
		amixer -c "$card_index" \
			cget "iface=CARD,name='Headphone Jack'"
}

wait_for_alsa_controls() {
	local card_index=$1 attempt snapshot

	for ((attempt = 1; attempt <= 50; attempt++)); do
		if required_alsa_controls_ready "$card_index"; then
			if snapshot=$(read_alsa_route_controls "$card_index" 2>/dev/null); then
				printf '%s\n' "$snapshot"
				return 0
			fi
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
			  "sink_max_volume_ratio=\(([.volume[].value] | max) / 65536)",
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

collect_playback_safety() {
	local card_index=$1 control snapshot metrics gain
	local -a controls=(Master Front Surround Center LFE PCM)

	for control in "${controls[@]}"; do
		if ! snapshot=$(amixer -c "$card_index" sget "$control" 2>/dev/null); then
			printf 'playback_volume=%s;percent=unknown;safe=no\n' "$control"
			continue
		fi
		if metrics=$(awk '
			/^[[:space:]]*Limits: Playback -?[0-9]+ - -?[0-9]+$/ {
				minimum = $3
				maximum = $5
			}
			/: Playback -?[0-9]+/ {
				value = $0
				sub(/^.*: Playback /, "", value)
				sub(/ .*/, "", value)
				if (!found || value > highest)
					highest = value
				found = 1
			}
			END {
				if (!found || maximum <= minimum)
					exit 1
				printf "%.1f;%s\n",
				    (highest - minimum) * 100 / (maximum - minimum),
				    (highest - minimum) * 100 <= (maximum - minimum) * 20 ?
				    "yes" : "no"
			}
		' <<< "$snapshot"); then
			printf 'playback_volume=%s;percent=%s;safe=%s\n' \
				"$control" "${metrics%;*}" "${metrics##*;}"
		else
			printf 'playback_volume=%s;percent=unknown;safe=no\n' "$control"
		fi
	done

	if snapshot=$(amixer -c "$card_index" sget 'AE-5: Headphone Gain' 2>/dev/null); then
		gain=$(sed -n "s/^[[:space:]]*Item0: '\\(.*\\)'$/\\1/p" <<< "$snapshot")
	fi
	printf 'headphone_gain=%s\n' "${gain:-unknown}"
}

collect_mixer_fingerprints() {
	local card_index=$1 fingerprint

	if ! command -v sha256sum >/dev/null 2>&1; then
		printf 'raw_mixer_sha256=unavailable\n'
		printf 'simple_mixer_sha256=unavailable\n'
		return
	fi

	if fingerprint=$(LC_ALL=C amixer -c "$card_index" contents 2>/dev/null |
		sha256sum); then
		printf 'raw_mixer_sha256=%s\n' "${fingerprint%% *}"
	else
		printf 'raw_mixer_sha256=unavailable\n'
	fi
	if fingerprint=$(LC_ALL=C amixer -c "$card_index" scontents 2>/dev/null |
		sha256sum); then
		printf 'simple_mixer_sha256=%s\n' "${fingerprint%% *}"
	else
		printf 'simple_mixer_sha256=unavailable\n'
	fi
}

collect_pcm_state() {
	local card_index=$1 status state open_count=0
	local -a statuses=()

	shopt -s nullglob
	statuses=("/proc/asound/card$card_index"/pcm*/sub*/status)
	for status in "${statuses[@]}"; do
		if ! read -r state < "$status"; then
			state=unreadable
		fi
		[[ $state == closed ]] || open_count=$((open_count + 1))
		printf 'pcm_status=%s;%s\n' "${status#"/proc/asound/card$card_index/"}" "$state"
	done
	printf 'pcm_status_files=%d\n' "${#statuses[@]}"
	printf 'pcm_open_count=%d\n' "$open_count"
}

collect_kernel_audio_warning_fingerprint() {
	local warnings fingerprint count

	if ! command -v journalctl >/dev/null 2>&1 ||
		! command -v sha256sum >/dev/null 2>&1; then
		printf 'kernel_audio_warning_count=unavailable\n'
		printf 'kernel_audio_warning_sha256=unavailable\n'
		return
	fi

	if ! warnings=$(journalctl -k -b -p warning..alert \
		--output=short-monotonic --no-pager 2>/dev/null |
		awk 'tolower($0) ~ /ca0132|snd|sound|hda|29:00[.]0|1102:0012/'); then
		printf 'kernel_audio_warning_count=unavailable\n'
		printf 'kernel_audio_warning_sha256=unavailable\n'
		return
	fi
	count=$(awk 'NF { count++ } END { print count + 0 }' <<< "$warnings")
	fingerprint=$(printf '%s' "$warnings" | sha256sum)
	printf 'kernel_audio_warning_count=%s\n' "$count"
	printf 'kernel_audio_warning_sha256=%s\n' "${fingerprint%% *}"
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
	local label=$1 card_index boot_id alsa_controls

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
		if alsa_controls=$(wait_for_alsa_controls "$card_index"); then
			printf 'alsa_control_ready=yes\n'
			printf '%s\n' "$alsa_controls"
		else
			printf '[required ALSA controls did not become readable within 5 seconds]\n'
		fi
	else
		printf '[amixer unavailable]\n'
	fi

	section 'Playback safety'
	if command -v amixer >/dev/null 2>&1; then
		collect_playback_safety "$card_index"
		collect_mixer_fingerprints "$card_index"
	else
		printf '[amixer unavailable]\n'
	fi

	section 'PCM state'
	collect_pcm_state "$card_index"

	section 'Codec routing pins'
	collect_codec_pins "$card_index"

	section 'Service timing and state'
	collect_service_state

	section 'PipeWire route'
	collect_pipewire "$card_index"

	section 'Kernel audio warnings'
	collect_kernel_audio_warning_fingerprint
}

summarize_history() {
	local input=$1 required=$2 mode=$3

	awk -v required="$required" -v mode="$mode" '
		function reset_record(  item) {
			label = boot = kernel = ready = output = front_left = front_right = ""
			auto_detect = jack = pipewire = active_port = active_profile = ""
			raw_hash = simple_hash = pcm_files = pcm_open = gain = sink_volume = ""
			warning_count = warning_hash = cycle = phase = ""
			node = ""
			expect_jack = expect_pipewire = 0
			for (item in pin)
				delete pin[item]
			for (item in safety)
				delete safety[item]
		}

		function reject(message) {
			record_ok = 0
			if (record_reason != "")
				record_reason = record_reason ", "
			record_reason = record_reason message
		}

		function finish_record(  key, item, prefix) {
			cycle = phase = ""
			if (mode == "boot") {
				if (label != "before-pipewire" && label != "after-pipewire")
					return
				phase = label
			} else {
				prefix = "before-suspend-"
				if (index(label, prefix) == 1) {
					phase = "before-suspend"
					cycle = substr(label, length(prefix) + 1)
				}
				prefix = "after-resume-"
				if (index(label, prefix) == 1) {
					phase = "after-resume"
					cycle = substr(label, length(prefix) + 1)
				}
				if (phase == "" || cycle == "")
					return
			}

			record_ok = 1
			record_reason = ""
			if (boot == "") {
				boot = "[missing boot id " ++missing_boot "]"
				reject("missing boot id")
			}

			if (kernel == "")
				reject("missing kernel")
			if (ready != "yes")
				reject("ALSA control not ready")
			if (output != "Headphone")
				reject("Output Select is not Headphone")
			if (front_left != "on" || front_right != "on")
				reject("Front is not unmuted")
			if (auto_detect != "off")
				reject("auto-detect is not off")
			if (jack != "on")
				reject("headphone jack is not present")
			if (pin["0x0b"] != "0x00" || pin["0x0f"] != "0x00" ||
			    pin["0x10"] != "0x00" || pin["0x11"] != "0x40")
				reject("codec pins do not select only 0x11")

			if (mode == "boot" && phase == "before-pipewire") {
				if (pipewire != "inactive")
					reject("PipeWire was active during the early probe")
			} else {
				if (pipewire != "active")
					reject("PipeWire was not active")
				if (active_port != "sound-blaster-ae5-output-headphones;output-headphones")
					reject("card-specific headphone port is not active")
				if (active_profile != "output:analog-stereo+input:analog-stereo")
					reject("unexpected PipeWire profile")
			}

			if (mode == "suspend") {
				if (raw_hash == "" || raw_hash == "unavailable")
					reject("raw mixer fingerprint is unavailable")
				if (simple_hash == "" || simple_hash == "unavailable")
					reject("simple mixer fingerprint is unavailable")
				if (pcm_files !~ /^[1-9][0-9]*$/)
					reject("PCM status files are unavailable")
				if (pcm_open != "0")
					reject("a PCM substream is open")
				for (item in required_safety)
					if (safety[item] != "yes")
						reject(item " exceeds the 20% safety ceiling")
				if (gain !~ /^Low [(]/)
					reject("headphone gain is not Low")
				if (sink_volume == "" || sink_volume + 0 > 0.20)
					reject("PipeWire volume exceeds the 20% safety ceiling")
				if (warning_count == "" || warning_count == "unavailable" ||
				    warning_hash == "" || warning_hash == "unavailable")
					reject("kernel audio warning fingerprint is unavailable")
			}

			key = mode == "boot" ? boot SUBSEP phase : cycle SUBSEP phase
			if (present[key]) {
				duplicates[mode == "boot" ? boot : cycle] = 1
				return
			}
			present[key] = 1
			valid[key] = record_ok
			reason[key] = record_reason
			boots[key] = boot
			record_kernels[key] = kernel
			raw_hashes[key] = raw_hash
			simple_hashes[key] = simple_hash
			warning_counts[key] = warning_count
			warning_hashes[key] = warning_hash
			sequences[key] = ++record_sequence

			if (mode == "boot") {
				kernels[boot] = kernel
				if (!(boot in boot_seen)) {
					boot_seen[boot] = 1
					boot_order[++boot_count] = boot
				}
			} else if (!(cycle in cycle_seen)) {
				cycle_seen[cycle] = 1
				cycle_order[++cycle_count] = cycle
			}
		}

		/^# AE-5 routing state$/ {
			finish_record()
			reset_record()
			next
		}
		/^label=/ {
			label = substr($0, 7)
			next
		}
		/^kernel=/ {
			kernel = substr($0, 8)
			next
		}
		/^boot_id=/ {
			boot = substr($0, 9)
			next
		}
		/^alsa_control_ready=/ {
			ready = substr($0, 20)
			next
		}
		/^playback_volume=/ {
			split($0, fields, ";")
			item = substr(fields[1], 17)
			safety[item] = substr(fields[3], 6)
			next
		}
		/^headphone_gain=/ {
			gain = substr($0, 16)
			next
		}
		/^raw_mixer_sha256=/ {
			raw_hash = substr($0, 18)
			next
		}
		/^simple_mixer_sha256=/ {
			simple_hash = substr($0, 21)
			next
		}
		/^pcm_status_files=/ {
			pcm_files = substr($0, 18)
			next
		}
		/^pcm_open_count=/ {
			pcm_open = substr($0, 16)
			next
		}
		/^kernel_audio_warning_count=/ {
			warning_count = substr($0, 28)
			next
		}
		/^kernel_audio_warning_sha256=/ {
			warning_hash = substr($0, 29)
			next
		}
		/Item0: '\''Headphone'\''/ {
			output = "Headphone"
			next
		}
		/Front Left: Playback/ {
			front_left = ($0 ~ /\[on\]$/) ? "on" : "off"
			next
		}
		/Front Right: Playback/ {
			front_right = ($0 ~ /\[on\]$/) ? "on" : "off"
			next
		}
		/Mono: Playback/ {
			auto_detect = ($0 ~ /\[off\]$/) ? "off" : "on"
			next
		}
		/name='\''Headphone Jack'\''/ {
			expect_jack = 1
			next
		}
		expect_jack && /: values=/ {
			jack = ($0 ~ /values=on/) ? "on" : "off"
			expect_jack = 0
			next
		}
		/^Node 0x/ {
			node = $2
			next
		}
		/^[[:space:]]+Pin-ctls:/ {
			value = $2
			sub(/:$/, "", value)
			pin[node] = value
			next
		}
		/^Id=pipewire.service$/ {
			expect_pipewire = 1
			next
		}
		expect_pipewire && /^ActiveState=/ {
			pipewire = substr($0, 13)
			expect_pipewire = 0
			next
		}
		/^sink_active_port=/ {
			active_port = substr($0, 18)
			next
		}
		/^sink_max_volume_ratio=/ {
			sink_volume = substr($0, 23)
			next
		}
		/^pipewire_active_profile=/ {
			active_profile = substr($0, 25)
			next
		}

		BEGIN {
			split("Master Front Surround Center LFE PCM", safety_names)
			for (item in safety_names)
				required_safety[safety_names[item]] = 1
			reset_record()
		}

		END {
			finish_record()
			if (mode == "suspend") {
				consecutive = completed = passing = 0
				for (cycle_index = 1; cycle_index <= cycle_count; cycle_index++) {
					cycle = cycle_order[cycle_index]
					before = cycle SUBSEP "before-suspend"
					after = cycle SUBSEP "after-resume"
					if (!present[before] || !present[after]) {
						printf "INCOMPLETE\t%s\tmissing %s record\n",
						    cycle, !present[before] ? "before-suspend" : "after-resume"
						consecutive = 0
						continue
					}

					completed++
					pair_reason = ""
					if (duplicates[cycle])
						pair_reason = "duplicate lifecycle record"
					if (sequences[before] >= sequences[after])
						pair_reason = add_reason(pair_reason, "after-resume is not after before-suspend")
					if (boots[before] != boots[after])
						pair_reason = add_reason(pair_reason, "boot ID changed across suspend")
					if (record_kernels[before] != record_kernels[after])
						pair_reason = add_reason(pair_reason, "kernel changed across suspend")
					if (raw_hashes[before] != raw_hashes[after])
						pair_reason = add_reason(pair_reason, "raw mixer state changed")
					if (simple_hashes[before] != simple_hashes[after])
						pair_reason = add_reason(pair_reason, "simple mixer state changed")
					if (warning_counts[before] != warning_counts[after] ||
					    warning_hashes[before] != warning_hashes[after])
						pair_reason = add_reason(pair_reason, "new kernel audio warning")

					if (valid[before] && valid[after] && pair_reason == "") {
						passing++
						consecutive++
						printf "PASS\t%s\t%s\t%s\n", cycle,
						    boots[before], record_kernels[before]
					} else {
						consecutive = 0
						detail = pair_reason
						if (!valid[before])
							detail = add_reason(detail, "before: " reason[before])
						if (!valid[after])
							detail = add_reason(detail, "after: " reason[after])
						printf "FAIL\t%s\t%s\n", cycle, detail
					}
				}

				printf "completed_cycles=%d\npassing_cycles=%d\n", completed, passing
				printf "consecutive_valid=%d\nrequired_cycles=%d\n", consecutive, required
				exit consecutive >= required ? 0 : 1
			}

			consecutive = completed = passing = 0
			for (boot_index = 1; boot_index <= boot_count; boot_index++) {
				boot = boot_order[boot_index]
				before = boot SUBSEP "before-pipewire"
				after = boot SUBSEP "after-pipewire"
				if (!present[before] || !present[after]) {
					printf "INCOMPLETE\t%s\t%s\tmissing %s probe\n",
					    boot, kernels[boot],
					    !present[before] ? "before-pipewire" : "after-pipewire"
					consecutive = 0
					continue
				}

				completed++
				if (valid[before] && valid[after]) {
					passing++
					consecutive++
					printf "PASS\t%s\t%s\n", boot, kernels[boot]
				} else {
					consecutive = 0
					detail = ""
					if (!valid[before])
						detail = "before: " reason[before]
					if (!valid[after]) {
						if (detail != "")
							detail = detail "; "
						detail = detail "after: " reason[after]
					}
					printf "FAIL\t%s\t%s\t%s\n", boot, kernels[boot], detail
				}
			}

			printf "completed_boots=%d\npassing_boots=%d\n", completed, passing
			printf "consecutive_valid=%d\nrequired_boots=%d\n", consecutive, required
			exit consecutive >= required ? 0 : 1
		}

		function add_reason(existing, addition) {
			return existing == "" ? addition : existing ", " addition
		}
	' "$input"
}

summarize_routing_history() {
	summarize_history "$1" "$2" boot
}

summarize_suspend_history() {
	summarize_history "$1" "$2" suspend
}

validate_suspend_snapshot() {
	local label=$1 snapshot=$2

	if [[ $label == before-suspend-* ]]; then
		{
			printf '%s\n' "$snapshot"
			sed 's/^label=before-suspend-/label=after-resume-/' <<< "$snapshot"
		} | summarize_suspend_history /dev/stdin 1
	else
		{
			sed 's/^label=after-resume-/label=before-suspend-/' <<< "$snapshot"
			printf '%s\n' "$snapshot"
		} | summarize_suspend_history /dev/stdin 1
	fi
}

emit_test_record() {
	local label=$1 boot_id=$2 front_state=$3
	local raw_hash=${4:-raw-hash} simple_hash=${5:-simple-hash}
	local pcm_open=${6:-0} safe=${7:-yes}
	local warning_hash=${8:-warning-hash}
	local pipewire_state=inactive
	local control

	if [[ $label != before-pipewire ]]; then
		pipewire_state=active
	fi

	printf '# AE-5 routing state\n'
	printf 'label=%s\nkernel=test-kernel\nboot_id=%s\n' "$label" "$boot_id"
	printf 'alsa_control_ready=yes\n'
	printf "  Item0: 'Headphone'\n"
	printf '  Front Left: Playback 90 [on]\n'
	printf '  Front Right: Playback 90 [%s]\n' "$front_state"
	printf '  Mono: Playback [off]\n'
	printf "numid=63,iface=CARD,name='Headphone Jack'\n"
	printf '  : values=on\n'
	printf 'Node 0x0b [Pin Complex]\n  Pin-ctls: 0x00:\n'
	printf 'Node 0x0f [Pin Complex]\n  Pin-ctls: 0x00:\n'
	printf 'Node 0x10 [Pin Complex]\n  Pin-ctls: 0x00:\n'
	printf 'Node 0x11 [Pin Complex]\n  Pin-ctls: 0x40: OUT VREF_HIZ\n'
	for control in Master Front Surround Center LFE PCM; do
		printf 'playback_volume=%s;percent=20.0;safe=%s\n' "$control" "$safe"
	done
	printf 'headphone_gain=Low (16-31  Ohms)\n'
	printf 'raw_mixer_sha256=%s\n' "$raw_hash"
	printf 'simple_mixer_sha256=%s\n' "$simple_hash"
	printf 'pcm_status_files=4\npcm_open_count=%s\n' "$pcm_open"
	printf 'kernel_audio_warning_count=0\n'
	printf 'kernel_audio_warning_sha256=%s\n' "$warning_hash"
	printf 'Id=pipewire.service\nActiveState=%s\n' "$pipewire_state"
	if [[ $pipewire_state == active ]]; then
		printf 'sink_active_port=sound-blaster-ae5-output-headphones;output-headphones\n'
		printf 'sink_max_volume_ratio=0.20\n'
		printf 'pipewire_active_profile=output:analog-stereo+input:analog-stereo\n'
	fi
}

run_self_test() (
	local calls=0
	local output
	local -a commands=()

	sleep() {
		:
	}
	amixer() {
		commands+=("$*")
		return 0
	}

	if ! required_alsa_controls_ready 7 ||
		(( ${#commands[@]} != 5 )) ||
		[[ ${commands[2]} != '-c 7 sget Front' ]]; then
		printf 'self-test failed: required ALSA control set\n' >&2
		return 1
	fi

	commands=()
	amixer() {
		commands+=("$*")
		[[ $* != '-c 7 sget Front' ]]
	}
	if required_alsa_controls_ready 7 || (( ${#commands[@]} != 3 )); then
		printf 'self-test failed: partial ALSA control set was accepted\n' >&2
		return 1
	fi

	required_alsa_controls_ready() {
		calls=$((calls + 1))
		(( calls >= 3 ))
	}
	read_alsa_route_controls() {
		printf 'complete snapshot\n'
	}

	if ! wait_for_alsa_controls 0 >/dev/null || (( calls != 3 )); then
		printf 'self-test failed: ALSA readiness retry\n' >&2
		return 1
	fi

	calls=0
	required_alsa_controls_ready() {
		calls=$((calls + 1))
		return 1
	}

	if wait_for_alsa_controls 0 || (( calls != 50 )); then
		printf 'self-test failed: ALSA readiness timeout\n' >&2
		return 1
	fi

	output=$(
		{
			emit_test_record before-pipewire boot-1 on
			emit_test_record after-pipewire boot-1 on
			emit_test_record before-pipewire boot-2 on
			emit_test_record after-pipewire boot-2 on
		} | summarize_routing_history /dev/stdin 2
	)
	grep -q '^consecutive_valid=2$' <<< "$output" || {
		printf 'self-test failed: valid routing history\n' >&2
		return 1
	}

	if output=$(
		{
			emit_test_record before-pipewire boot-1 on
			emit_test_record after-pipewire boot-1 on
			emit_test_record before-pipewire boot-2 on
			emit_test_record after-pipewire boot-2 off
		} | summarize_routing_history /dev/stdin 2
	); then
		printf 'self-test failed: invalid routing history accepted\n' >&2
		return 1
	fi
	grep -q '^consecutive_valid=0$' <<< "$output" || {
		printf 'self-test failed: routing failure did not reset progress\n' >&2
		return 1
	}

	output=$(
		{
			emit_test_record before-suspend-campaign-01 boot-1 on
			emit_test_record after-resume-campaign-01 boot-1 on
			emit_test_record before-suspend-campaign-02 boot-1 on
			emit_test_record after-resume-campaign-02 boot-1 on
		} | summarize_suspend_history /dev/stdin 2
	)
	grep -q '^consecutive_valid=2$' <<< "$output" || {
		printf 'self-test failed: valid suspend history\n' >&2
		return 1
	}

	if output=$(
		{
			emit_test_record before-suspend-campaign-01 boot-1 on
			emit_test_record after-resume-campaign-01 boot-1 on changed-hash
		} | summarize_suspend_history /dev/stdin 1
	); then
		printf 'self-test failed: changed mixer state was accepted\n' >&2
		return 1
	fi
	grep -q 'raw mixer state changed' <<< "$output" || {
		printf 'self-test failed: changed mixer state was not diagnosed\n' >&2
		return 1
	}

	if output=$(
		{
			emit_test_record before-suspend-campaign-01 boot-1 on
			emit_test_record after-resume-campaign-01 boot-1 on \
				raw-hash simple-hash 0 no
		} | summarize_suspend_history /dev/stdin 1
	); then
		printf 'self-test failed: unsafe suspend state was accepted\n' >&2
		return 1
	fi
	grep -q 'exceeds the 20% safety ceiling' <<< "$output" || {
		printf 'self-test failed: unsafe suspend state was not diagnosed\n' >&2
		return 1
	}

	if output=$(
		{
			emit_test_record before-suspend-campaign-01 boot-1 on
			emit_test_record after-resume-campaign-01 boot-2 on \
				raw-hash simple-hash 1 yes new-warning-hash
		} | summarize_suspend_history /dev/stdin 1
	); then
		printf 'self-test failed: invalid suspend lifecycle was accepted\n' >&2
		return 1
	fi
	for diagnosis in \
		'boot ID changed across suspend' \
		'new kernel audio warning' \
		'a PCM substream is open'; do
		grep -q "$diagnosis" <<< "$output" || {
			printf 'self-test failed: missing suspend diagnosis: %s\n' \
				"$diagnosis" >&2
			return 1
		}
	done

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

if [[ ${1:-} == --summary ]]; then
	[[ $# -le 2 ]] || {
		usage
		exit 2
	}
	required_boots=${2:-10}
	[[ $required_boots =~ ^[1-9][0-9]*$ ]] || {
		printf 'required-boots must be a positive integer\n' >&2
		exit 2
	}
	state_root=${XDG_STATE_HOME:-"$HOME/.local/state"}
	input=${AE5_ROUTING_LOG:-"$state_root/ae5-control/routing-boot.log"}
	[[ -r $input ]] || {
		printf 'routing history is not readable: %s\n' "$input" >&2
		exit 1
	}
	summarize_routing_history "$input" "$required_boots"
	exit
fi

if [[ ${1:-} == --suspend-summary ]]; then
	[[ $# -le 2 ]] || {
		usage
		exit 2
	}
	required_cycles=${2:-20}
	[[ $required_cycles =~ ^[1-9][0-9]*$ ]] || {
		printf 'required-cycles must be a positive integer\n' >&2
		exit 2
	}
	state_root=${XDG_STATE_HOME:-"$HOME/.local/state"}
	input=${AE5_ROUTING_LOG:-"$state_root/ae5-control/routing-boot.log"}
	[[ -r $input ]] || {
		printf 'routing history is not readable: %s\n' "$input" >&2
		exit 1
	}
	summarize_suspend_history "$input" "$required_cycles"
	exit
fi

if [[ ${1:-} == --before-suspend || ${1:-} == --after-resume ]]; then
	phase=$1
	[[ $# -eq 2 ]] || {
		usage
		exit 2
	}
	cycle=$2
	[[ $cycle =~ ^[A-Za-z0-9][A-Za-z0-9._-]*$ && ${#cycle} -le 64 ]] || {
		printf 'cycle must be 1-64 letters, digits, dots, underscores, or dashes\n' >&2
		exit 2
	}
	label=${1#--}-$cycle
	umask 077
	state_root=${XDG_STATE_HOME:-"$HOME/.local/state"}
	output=${AE5_ROUTING_LOG:-"$state_root/ae5-control/routing-boot.log"}
	snapshot=$(collect "$label")
	collect_status=$?

	if [[ $phase == --before-suspend ]]; then
		if ((collect_status != 0)) ||
			! validation=$(validate_suspend_snapshot "$label" "$snapshot"); then
			printf '%s\n' "${validation:-suspend snapshot collection failed}" >&2
			printf 'pre-suspend snapshot rejected; it was not appended and no suspend was initiated\n' >&2
			exit 1
		fi
	fi

	mkdir -p -- "$(dirname -- "$output")"
	printf '%s\n' "$snapshot" >> "$output"
	printf 'routing state appended to %s\n' "$output" >&2

	if ((collect_status != 0)) ||
		! validation=$(validate_suspend_snapshot "$label" "$snapshot"); then
		printf '%s\n' "${validation:-suspend snapshot collection failed}" >&2
		exit 1
	fi
	if [[ $phase == --before-suspend ]]; then
		printf 'pre-suspend snapshot passed; this command did not suspend the system\n' >&2
	fi
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
