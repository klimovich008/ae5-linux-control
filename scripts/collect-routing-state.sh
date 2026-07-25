#!/usr/bin/env bash
set -uo pipefail

usage() {
	printf 'usage: %s [--self-test | --summary [required-boots] | [label] [--append]]\n' "$0" >&2
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

	section 'Codec routing pins'
	collect_codec_pins "$card_index"

	section 'Service timing and state'
	collect_service_state

	section 'PipeWire route'
	collect_pipewire "$card_index"
}

summarize_routing_history() {
	local input=$1 required_boots=$2

	awk -v required="$required_boots" '
		function reset_record(  item) {
			label = boot = kernel = ready = output = front_left = front_right = ""
			auto_detect = jack = pipewire = active_port = active_profile = ""
			node = ""
			expect_jack = expect_pipewire = 0
			for (item in pin)
				delete pin[item]
		}

		function reject(message) {
			record_ok = 0
			if (record_reason != "")
				record_reason = record_reason ", "
			record_reason = record_reason message
		}

		function finish_record(  key) {
			if (label != "before-pipewire" && label != "after-pipewire")
				return
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

			if (label == "before-pipewire") {
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

			key = boot SUBSEP label
			present[key] = 1
			valid[key] = record_ok
			reason[key] = record_reason
			kernels[boot] = kernel
			if (!(boot in boot_seen)) {
				boot_seen[boot] = 1
				boot_order[++boot_count] = boot
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
		/^pipewire_active_profile=/ {
			active_profile = substr($0, 25)
			next
		}

		BEGIN {
			reset_record()
		}

		END {
			finish_record()
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
	' "$input"
}

emit_test_record() {
	local label=$1 boot_id=$2 front_state=$3
	local pipewire_state=inactive

	if [[ $label == after-pipewire ]]; then
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
	printf 'Id=pipewire.service\nActiveState=%s\n' "$pipewire_state"
	if [[ $label == after-pipewire ]]; then
		printf 'sink_active_port=sound-blaster-ae5-output-headphones;output-headphones\n'
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
