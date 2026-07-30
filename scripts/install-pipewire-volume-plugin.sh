#!/usr/bin/env bash
set -euo pipefail

usage() {
	printf 'usage: %s PLUGIN | --uninstall\n' "$0" >&2
	exit 2
}

[[ $# -eq 1 ]] || usage

script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root=$(dirname -- "$script_root")
plugin_root=/usr/local/lib64/ae5-control/spa-0.2
plugin_target=$plugin_root/audioconvert/libspa-audioconvert.so
dropin_root=${XDG_CONFIG_HOME:-"$HOME/.config"}/systemd/user/pipewire.service.d
dropin_target=$dropin_root/50-ae5-spa-plugin.conf
dropin_source=$project_root/packaging/systemd/user/50-ae5-spa-plugin.conf

if [[ $1 == --uninstall ]]; then
	sudo rm -f -- "$plugin_target"
	rm -f -- "$dropin_target"
	systemctl --user daemon-reload
	printf 'AE-5 PipeWire volume overlay removed; restart PipeWire when audio is idle\n'
	exit
fi

plugin=$(realpath -- "$1")
[[ -f $plugin && -x $plugin ]] || {
	printf 'error: plugin is not a readable executable file: %s\n' "$plugin" >&2
	exit 1
}
[[ -f $dropin_source ]] || {
	printf 'error: PipeWire service drop-in is missing: %s\n' "$dropin_source" >&2
	exit 1
}
[[ $(pkg-config --variable=plugindir libspa-0.2) == /usr/lib64/spa-0.2 ]] || {
	printf 'error: this installer currently supports Fedora-compatible lib64 layouts only\n' >&2
	exit 1
}

sudo install -Dm0755 "$plugin" "$plugin_target"
install -Dm0644 "$dropin_source" "$dropin_target"
systemctl --user daemon-reload
printf 'AE-5 PipeWire volume overlay installed; restart PipeWire when audio is idle\n'
