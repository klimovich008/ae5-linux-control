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
PATH="$test_home/.local/bin:$PATH" ae5ctl help >/dev/null
HOME=$test_home XDG_CONFIG_HOME=$config_root \
	PATH="$test_home/.local/bin:$PATH" \
	ae5-collect-report --self-test >/dev/null

manifest_before=$(sha256sum "$manifest")
run_installer --from-build >/dev/null
[[ $(sha256sum "$manifest") == "$manifest_before" ]]

install -Dm0600 /dev/null \
	"$config_root/ae5-control/profiles/preserve-me.json"
printf '%s\n' preserve > "$config_root/ae5-control/lighting.json"
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

printf 'rootless user install: payload, integration, idempotence, dependency/conflict refusal, and removal validated\n'
