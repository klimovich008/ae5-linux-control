#!/usr/bin/env bash
set -euo pipefail

usage() {
	printf 'usage: %s BINARY_RPM\n' "$0" >&2
}

fail() {
	printf 'error: %s\n' "$1" >&2
	exit 1
}

[[ $# -eq 1 ]] || {
	usage
	exit 2
}
(( EUID == 0 )) || fail 'RPM lifecycle check must run as root'
[[ -e /run/.containerenv || -e /.dockerenv ]] ||
	fail 'RPM lifecycle check is restricted to a disposable container'

for tool in dnf realpath rpm sha256sum; do
	command -v "$tool" >/dev/null 2>&1 ||
		fail "required tool is unavailable: $tool"
done

rpm_path=$(realpath -- "$1")
[[ -f $rpm_path ]] || fail "RPM does not exist: $rpm_path"
[[ $(rpm -qp --qf '%{NAME}' "$rpm_path") == ae5-control ]] ||
	fail 'RPM package name is not ae5-control'
if rpm -q ae5-control >/dev/null 2>&1; then
	fail 'ae5-control is already installed'
fi

profile_state=/root/.config/ae5-control/profiles/package-lifecycle-sentinel
alsa_state=/var/lib/alsa/asound.state
mkdir -p -- "${profile_state%/*}" "${alsa_state%/*}"
printf 'profile state must survive package removal\n' > "$profile_state"
printf 'ALSA state must survive package removal\n' > "$alsa_state"
state_before=$(sha256sum "$profile_state" "$alsa_state")

dnf --quiet --assumeyes --setopt=install_weak_deps=False install "$rpm_path"
rpm -V ae5-control

for command in ae5-control ae5ctl ae5-collect-report; do
	command -v "$command" >/dev/null 2>&1 ||
		fail "installed command is unavailable: $command"
done
ae5ctl help | grep -Fq 'sbcommand-import-active'
ae5ctl help | grep -Fq 'lighting-restore'
ae5ctl help | grep -Fq 'features [verified|substituted|deferred|unsupported]'
ae5ctl features unsupported | grep -Fq 'Device · Super X-Fi'
ae5-collect-report --self-test

if [[ ! -e /dev/snd ]] && ae5ctl status >/dev/null 2>&1; then
	fail 'hardware status unexpectedly succeeded without ALSA device nodes'
fi

mapfile -t owned_files < <(
	while read -r path; do
		if [[ -f $path || -L $path ]]; then
			printf '%s\n' "$path"
		fi
	done < <(rpm -ql ae5-control)
)
(( ${#owned_files[@]} > 0 )) || fail 'installed package owns no files'

dnf --quiet --assumeyes remove ae5-control
if rpm -q ae5-control >/dev/null 2>&1; then
	fail 'ae5-control remains installed'
fi
for path in "${owned_files[@]}"; do
	if [[ -e $path || -L $path ]]; then
		fail "package-owned file remains after removal: $path"
	fi
done

state_after=$(sha256sum "$profile_state" "$alsa_state")
[[ $state_before == "$state_after" ]] ||
	fail 'package removal changed user profile or ALSA state'

printf 'RPM lifecycle check passed: %d owned files removed; profile and ALSA state preserved\n' \
	"${#owned_files[@]}"
