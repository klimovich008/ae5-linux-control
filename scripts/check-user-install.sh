#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
for binary in ae5-control ae5ctl; do
	[[ -x $repo_root/target/release/$binary ]] || {
		printf 'error: build release binaries before the user-install check\n' >&2
		exit 1
	}
done

test_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-user-install.XXXXXX")
cleanup() {
	find "$test_root" -depth -delete
}
trap cleanup EXIT

test_home=$test_root/home
data_root=$test_root/data
config_root=$test_root/config
state_root=$test_root/state
system_acp=$test_root/system-acp
install -d -m0755 "$test_home" "$system_acp"
for path in \
	mixer/profile-sets/default.conf \
	mixer/profile-sets/9999-custom.conf \
	mixer/paths/analog-output-headphones.conf \
	mixer/paths/analog-output.conf.common; do
	install -Dm0644 /dev/null "$system_acp/$path"
done

run_installer() {
	HOME=$test_home \
	XDG_DATA_HOME=$data_root \
	XDG_CONFIG_HOME=$config_root \
	XDG_STATE_HOME=$state_root \
	AE5_SYSTEM_ACP_ROOT=$system_acp \
	PATH="$test_home/.local/bin:$PATH" \
		bash "$repo_root/scripts/install-user.sh" "$@"
}

run_installer --from-build
payload=$data_root/ae5-control/user-install
manifest=$state_root/ae5-control/user-install-links.v1
[[ -x $payload/bin/ae5-control && -x $payload/bin/ae5ctl ]]
[[ -x $payload/bin/ae5-control-user-install ]]
cmp "$payload/bin/ae5-control" "$repo_root/target/release/ae5-control"
cmp "$payload/bin/ae5ctl" "$repo_root/target/release/ae5ctl"
[[ -L $test_home/.local/bin/ae5-control ]]
[[ -L $data_root/applications/io.github.klimovich008.ae5control.desktop ]]
[[ -L $config_root/wireplumber/wireplumber.conf.d/90-ae5-control.conf ]]
[[ -L $config_root/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf ]]
[[ -L $config_root/alsa-card-profile/mixer/profile-sets/default.conf ]]
desktop-file-validate \
	"$data_root/applications/io.github.klimovich008.ae5control.desktop"
appstreamcli validate --no-net --strict \
	"$data_root/metainfo/io.github.klimovich008.ae5control.metainfo.xml"
PATH="$test_home/.local/bin:$PATH" ae5ctl help | grep -Fq 'route-repair'
PATH="$test_home/.local/bin:$PATH" ae5ctl features unsupported |
	grep -Fq 'Device · Super X-Fi'
profile_root=$config_root/ae5-control/profiles
install -d -m0700 "$profile_root"
printf '%s\n' \
	'{' \
	'  "format_version": 1,' \
	'  "name": "Headphones",' \
	'  "target": "1102:0012/1102:0051",' \
	'  "controls": {' \
	'    "Output Select": {' \
	'      "choice": "Headphone"' \
	'    }' \
	'  }' \
	'}' > "$profile_root/rename-me.json"
rename_output=$(
	HOME=$test_home XDG_CONFIG_HOME=$config_root \
		PATH="$test_home/.local/bin:$PATH" \
		ae5ctl profile-rename rename-me.json '  Late night  '
)
grep -Fq "renamed saved profile to 'Late night' (rename-me.json)" \
	<<<"$rename_output"
grep -Fq '"name": "Late night"' "$profile_root/rename-me.json"
cp -- "$profile_root/rename-me.json" "$config_root/ae5-control/outside.json"
if HOME=$test_home XDG_CONFIG_HOME=$config_root \
	PATH="$test_home/.local/bin:$PATH" \
	ae5ctl profile-rename ../outside.json Escaped \
	>"$test_root/profile-rename-escape.log" 2>&1; then
	printf 'error: profile rename accepted a path outside the library\n' >&2
	exit 1
fi
grep -Fq 'profile is not a JSON file directly inside the profile library' \
	"$test_root/profile-rename-escape.log"
grep -Fq '"name": "Late night"' "$config_root/ae5-control/outside.json"
HOME=$test_home XDG_CONFIG_HOME=$config_root \
	PATH="$test_home/.local/bin:$PATH" \
	ae5-collect-report --self-test >/dev/null

manifest_before=$(sha256sum "$manifest")
run_installer --from-build >/dev/null
[[ $(sha256sum "$manifest") == "$manifest_before" ]]

printf '%s\n' old-cli > "$payload/bin/ae5ctl"
printf '%s\n' old-gui > "$payload/bin/ae5-control"
install -Dm0600 /dev/null \
	"$config_root/ae5-control/profiles/preserve-me.json"
printf '%s\n' preserve > "$config_root/ae5-control/lighting.json"
upgrade_output=$(run_installer --from-build)
grep -Fq 'AE-5 Control upgraded' <<<"$upgrade_output"
cmp "$payload/bin/ae5-control" "$repo_root/target/release/ae5-control"
cmp "$payload/bin/ae5ctl" "$repo_root/target/release/ae5ctl"
[[ $(sha256sum "$manifest") == "$manifest_before" ]]
[[ -f $config_root/ae5-control/profiles/preserve-me.json ]]
grep -Fxq preserve "$config_root/ae5-control/lighting.json"
if find "$data_root/ae5-control" -mindepth 1 -maxdepth 1 \
	-name '.user-install.*' -print -quit | grep -q .; then
	printf 'error: upgrade left a staging or backup payload\n' >&2
	exit 1
fi

payload_before=$(find "$payload" -type f -print0 |
	sort -z |
	xargs -0 sha256sum)
fake_bin=$test_root/fake-bin
install -d -m0755 "$fake_bin"
real_cmp=$(command -v cmp)
{
	printf '#!/usr/bin/env bash\n'
	printf 'for argument in "$@"; do\n'
	printf '\tif [[ $argument == *%q* ]]; then exit 1; fi\n' \
		'/.user-install.new.'
	printf 'done\n'
	printf 'exec %q "$@"\n' "$real_cmp"
} > "$fake_bin/cmp"
chmod 0755 "$fake_bin/cmp"
saved_path=$PATH
PATH=$fake_bin:$PATH
if run_installer --from-build >"$test_root/staged-verify.log" 2>&1; then
	printf 'error: upgrade accepted a failed staged verification\n' >&2
	exit 1
fi
PATH=$saved_path
grep -Fq 'staged payload verification failed' \
	"$test_root/staged-verify.log"
[[ $(find "$payload" -type f -print0 |
	sort -z |
	xargs -0 sha256sum) == "$payload_before" ]]
if find "$data_root/ae5-control" -mindepth 1 -maxdepth 1 \
	-name '.user-install.*' -print -quit | grep -q .; then
	printf 'error: failed upgrade left a staging or backup payload\n' >&2
	exit 1
fi

fake_mv_bin=$test_root/fake-mv-bin
install -d -m0755 "$fake_mv_bin"
real_mv=$(command -v mv)
{
	printf '#!/usr/bin/env bash\n'
	printf 'for argument in "$@"; do\n'
	printf '\tif [[ $argument == *%q* ]]; then exit 1; fi\n' \
		'/.user-install.new.'
	printf 'done\n'
	printf 'exec %q "$@"\n' "$real_mv"
} > "$fake_mv_bin/mv"
chmod 0755 "$fake_mv_bin/mv"
PATH=$fake_mv_bin:$PATH
if run_installer --from-build >"$test_root/swap-failure.log" 2>&1; then
	printf 'error: upgrade accepted a failed payload swap\n' >&2
	exit 1
fi
PATH=$saved_path
[[ $(find "$payload" -type f -print0 |
	sort -z |
	xargs -0 sha256sum) == "$payload_before" ]]
if find "$data_root/ae5-control" -mindepth 1 -maxdepth 1 \
	-name '.user-install.*' -print -quit | grep -q .; then
	printf 'error: swap rollback left a staging or backup payload\n' >&2
	exit 1
fi

rm -f -- "$test_home/.local/bin/ae5ctl"
printf '%s\n' conflict > "$test_home/.local/bin/ae5ctl"
if run_installer --from-build >"$test_root/upgrade-conflict.log" 2>&1; then
	printf 'error: upgrade replaced a conflicting user path\n' >&2
	exit 1
fi
grep -Fq 'refusing to replace existing path' \
	"$test_root/upgrade-conflict.log"
[[ $(find "$payload" -type f -print0 |
	sort -z |
	xargs -0 sha256sum) == "$payload_before" ]]
grep -Fxq conflict "$test_home/.local/bin/ae5ctl"
rm -f -- "$test_home/.local/bin/ae5ctl"
ln -s -- "$payload/bin/ae5ctl" "$test_home/.local/bin/ae5ctl"

printf '%s\n' invalid > "$payload/.ae5-control-user-install"
if HOME=$test_home \
	XDG_DATA_HOME=$data_root \
	XDG_CONFIG_HOME=$config_root \
	XDG_STATE_HOME=$state_root \
	AE5_SYSTEM_ACP_ROOT=$system_acp \
	PATH="$test_home/.local/bin:$PATH" \
	ae5-control-user-install --uninstall \
	>"$test_root/invalid-marker.log" 2>&1; then
	printf 'error: uninstaller accepted an invalid payload marker\n' >&2
	exit 1
fi
grep -Fq 'unknown marker' "$test_root/invalid-marker.log"
[[ -L $test_home/.local/bin/ae5-control && -d $payload ]]
printf '%s\n' ae5-control-user-install-v1 \
	> "$payload/.ae5-control-user-install"
HOME=$test_home \
XDG_DATA_HOME=$data_root \
XDG_CONFIG_HOME=$config_root \
XDG_STATE_HOME=$state_root \
AE5_SYSTEM_ACP_ROOT=$system_acp \
PATH="$test_home/.local/bin:$PATH" \
	ae5-control-user-install --uninstall
[[ ! -e $payload && ! -L $payload ]]
[[ ! -e $test_home/.local/bin/ae5-control &&
	! -L $test_home/.local/bin/ae5-control ]]
[[ ! -e $data_root/applications/io.github.klimovich008.ae5control.desktop &&
	! -L $data_root/applications/io.github.klimovich008.ae5control.desktop ]]
[[ -f $config_root/ae5-control/profiles/preserve-me.json ]]
grep -Fxq preserve "$config_root/ae5-control/lighting.json"

printf '%s\n' conflict > "$test_home/.local/bin/ae5ctl"
if run_installer --from-build >"$test_root/conflict.log" 2>&1; then
	printf 'error: installer replaced a conflicting user path\n' >&2
	exit 1
fi
grep -Fq 'refusing to replace existing path' "$test_root/conflict.log"
grep -Fxq conflict "$test_home/.local/bin/ae5ctl"
[[ ! -e $payload && ! -L $payload ]]

rm -f -- "$test_home/.local/bin/ae5ctl" \
	"$system_acp/mixer/profile-sets/default.conf"
if run_installer --from-build >"$test_root/missing-acp.log" 2>&1; then
	printf 'error: installer accepted a missing system ACP dependency\n' >&2
	exit 1
fi
grep -Fq 'required system ACP file is missing' "$test_root/missing-acp.log"
[[ ! -e $payload && ! -L $payload ]]

printf 'rootless user install: payload, transactional upgrade, integration, idempotence, dependency/conflict refusal, and removal validated\n'
