#!/usr/bin/env bash
set -euo pipefail

unit_root=${XDG_CONFIG_HOME:-"$HOME/.config"}/systemd/user
probe_target=$HOME/.local/libexec/ae5-routing-probe
script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root=$(dirname -- "$script_root")

if [[ ${1:-} == --uninstall ]]; then
	[[ $# -eq 1 ]] || {
		printf 'usage: %s [--uninstall]\n' "$0" >&2
		exit 2
	}
	systemctl --user disable ae5-routing-before.service ae5-routing-after.timer \
		2>/dev/null || true
	rm -f -- \
		"$unit_root/ae5-routing-before.service" \
		"$unit_root/ae5-routing-after.service" \
		"$unit_root/ae5-routing-after.timer" \
		"$probe_target"
	systemctl --user daemon-reload
	printf 'routing probe uninstalled; its private state log was retained\n'
	exit
fi

[[ $# -eq 0 ]] || {
	printf 'usage: %s [--uninstall]\n' "$0" >&2
	exit 2
}

install -Dm755 \
	"$project_root/scripts/collect-routing-state.sh" \
	"$probe_target"
for unit in \
	ae5-routing-before.service \
	ae5-routing-after.service \
	ae5-routing-after.timer; do
	install -Dm644 "$project_root/systemd/user/$unit" "$unit_root/$unit"
done

systemctl --user daemon-reload
systemctl --user enable ae5-routing-before.service ae5-routing-after.timer
printf 'routing probe enabled for the next login/boot; this command did not reboot\n'
