#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
profile=$repo_root/packaging/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf
path=$repo_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf
rule=$repo_root/packaging/wireplumber/90-ae5-control.conf
input_paths='sound-blaster-ae5-input-microphone sound-blaster-ae5-input-front-microphone sound-blaster-ae5-input-line-in'
output_paths='analog-output analog-output-lineout analog-output-speaker sound-blaster-ae5-output-headphones analog-output-headphones-2'

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

front=$(awk '
	$0 == "[Element Front]" { found = 1; next }
	found && /^\[/ { exit }
	found && /^(switch|volume) = / { print }
' "$path")
[[ $front == $'switch = mute\nvolume = zero' ]]

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

printf 'AE-5 ACP profile: stable managed route order and shared Front DAC validated\n'
