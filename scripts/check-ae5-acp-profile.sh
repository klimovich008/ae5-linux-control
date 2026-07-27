#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
profile=$repo_root/packaging/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf
path=$repo_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf
rule=$repo_root/packaging/wireplumber/90-ae5-control.conf
input_paths='sound-blaster-ae5-input-microphone sound-blaster-ae5-input-front-microphone sound-blaster-ae5-input-line-in'
output_paths='analog-output analog-output-lineout analog-output-speaker sound-blaster-ae5-output-headphones analog-output-headphones-2'

device=$(awk '
	$0 == "[Mapping analog-stereo]" { found = 1; next }
	found && /^\[/ { exit }
	found && /^device-strings = / {
		sub(/^device-strings = /, "")
		print
		exit
	}
' "$profile")
[[ $device == 'hw:%f' ]]

for mapping in \
	analog-surround-21 analog-surround-40 analog-surround-41 \
	analog-surround-50 analog-surround-51; do
	device=$(awk -v section="[Mapping $mapping]" '
		$0 == section { found = 1; next }
		found && /^\[/ { exit }
		found && /^device-strings = / {
			sub(/^device-strings = /, "")
			print
			exit
		}
	' "$profile")
	[[ $device == 'hw:%f' ]]
done

for mapping in analog-stereo stereo-fallback mono-fallback; do
	paths=$(awk -v section="[Mapping $mapping]" '
		$0 == section { found = 1; next }
		found && /^\[/ { exit }
		found && /^paths-output = / {
			sub(/^paths-output = /, "")
			print
			exit
		}
	' "$profile")
	if [[ $mapping == mono-fallback ]]; then
		[[ $paths == "$output_paths analog-output-mono" ]]
	else
		[[ $paths == "$output_paths" ]]
	fi
done

for mapping in \
	analog-stereo stereo-fallback mono-fallback \
	analog-surround-21 analog-surround-40 analog-surround-41 \
	analog-surround-50 analog-surround-51 analog-surround-71; do
	paths=$(awk -v section="[Mapping $mapping]" '
		$0 == section { found = 1; next }
		found && /^\[/ { exit }
		found && /^paths-input = / {
			sub(/^paths-input = /, "")
			print
			exit
		}
	' "$profile")
	[[ $paths == "$input_paths" ]]
done

for element in Master Front; do
	settings=$(awk -v section="[Element $element]" '
		$0 == section { found = 1; next }
		found && /^\[/ { exit }
		found && /^(switch|volume) = / { print }
	' "$path")
	[[ $settings == $'switch = on\nvolume = zero' ]]
done

pcm=$(awk '
	$0 == "[Element PCM]" { found = 1; next }
	found && /^\[/ { exit }
	found && /^(switch|volume) = / { print }
' "$path")
[[ $pcm == $'switch = mute\nvolume = zero' ]]

grep -Fqx '[Jack Headphone]' "$path"
grep -Fqx 'required = any' "$path"
output_select=$(awk '
	$0 == "[Element Output Select]" { found = 1; next }
	found && /^\[/ { exit }
	found && /^(required|enumeration) = / { print }
' "$path")
[[ $output_select == $'required = enumeration\nenumeration = select' ]]

[[ $(grep -Ec '^\[Option Output Select:' "$path") -eq 1 ]]
headphone_option=$(awk '
	$0 == "[Option Output Select:Headphone]" { found = 1; next }
	found && /^\[/ { exit }
	found && /^(required|name|priority) = / { print }
' "$path")
[[ $headphone_option == \
	$'required = enumeration\nname = output-headphones\npriority = 10' ]]
if grep -Fq '[Option Output Select:Speakers]' "$path"; then
	printf 'AE-5 headphone path exposes an invalid Speakers route\n' >&2
	exit 1
fi
if grep -Fq '.include analog-output-headphones.conf' "$path"; then
	printf 'AE-5 headphone path inherits ambiguous generic Output Select options\n' >&2
	exit 1
fi

for input in microphone front-microphone line-in; do
	input_path=$repo_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-$input.conf
	grep -Fq '[Element Capture]' "$input_path"
	grep -Fq 'switch = mute' "$input_path"
	grep -Fq 'volume = merge' "$input_path"
	grep -Fq '[Element Input Source]' "$input_path"
	grep -Fq 'enumeration = select' "$input_path"
done
grep -Fq '[Option Input Source:Microphone]' \
	"$repo_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-microphone.conf"
grep -Fq '[Option Input Source:Front Microphone]' \
	"$repo_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-front-microphone.conf"
grep -Fq '[Option Input Source:Line In]' \
	"$repo_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-line-in.conf"

grep -Fq 'device.vendor.id = "0x1102"' "$rule"
grep -Fq 'device.product.id = "0x0012"' "$rule"
grep -Fq 'device.profile-set = "sound-blaster-ae5.conf"' "$rule"
grep -Fq 'api.alsa.soft-mixer = true' "$rule"
grep -Fq 'api.alsa.ignore-dB = true' "$rule"
grep -Fq 'node.name = "~alsa_output.*"' "$rule"
grep -Fq 'device.profile.name = "~analog-.*"' "$rule"
grep -Fq 'alsa.components = "~HDA:11020011,11020051,.*"' "$rule"
grep -Fq 'audio.format = "S16LE"' "$rule"
grep -Fq 'api.alsa.disable-mmap = true' "$rule"
grep -Fq 'api.alsa.period-size = 6016' "$rule"
grep -Fq 'api.alsa.period-num = 4' "$rule"
grep -Fq 'session.suspend-timeout-seconds = 0' "$rule"

printf 'AE-5 ACP profile: persistent raw S16 playback, one exact headphone route, and shared Front DAC validated\n'
