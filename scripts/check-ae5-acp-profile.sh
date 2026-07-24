#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
profile=$repo_root/packaging/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf
path=$repo_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf
rule=$repo_root/packaging/wireplumber/90-ae5-control.conf
fixed_path=sound-blaster-ae5-output-headphones

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
	[[ " $paths " == *" $fixed_path "* ]]
	[[ " $paths " != *" analog-output-headphones "* ]]
done

front=$(awk '
	$0 == "[Element Front]" { found = 1; next }
	found && /^\[/ { exit }
	found && /^(switch|volume) = / { print }
' "$path")
[[ $front == $'switch = mute\nvolume = zero' ]]

grep -Fq 'device.vendor.id = "0x1102"' "$rule"
grep -Fq 'device.product.id = "0x0012"' "$rule"
grep -Fq 'device.profile-set = "sound-blaster-ae5.conf"' "$rule"

printf 'AE-5 ACP profile: shared Front DAC remains enabled for headphones\n'
