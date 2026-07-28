#!/usr/bin/env bash
# Build the development-only PipeWire client used by the transition harness.
set -euo pipefail

script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root=$(dirname -- "$script_root")
source_file=$project_root/tools/pipewire-format-renegotiate.c
output=${1:-"$project_root/target/diagnostics/ae5-pw-renegotiate"}
compiler=${CC:-cc}

command -v "$compiler" >/dev/null 2>&1 || {
	printf 'error: C compiler is unavailable: %s\n' "$compiler" >&2
	exit 1
}
command -v pkg-config >/dev/null 2>&1 || {
	printf 'error: pkg-config is unavailable\n' >&2
	exit 1
}
pkg-config --exists libpipewire-0.3 || {
	printf '%s\n' \
		'error: PipeWire development headers are unavailable' \
		'Fedora: sudo dnf install pipewire-devel' \
		'Debian/Ubuntu: sudo apt install libpipewire-0.3-dev' >&2
	exit 1
}

read -r -a cflags <<< "$(pkg-config --cflags libpipewire-0.3)"
read -r -a libraries <<< "$(pkg-config --libs libpipewire-0.3)"
install -d -m0755 "$(dirname -- "$output")"
temporary=$(mktemp "$(dirname -- "$output")/.ae5-pw-renegotiate.XXXXXX")
cleanup() {
	rm -f -- "$temporary"
}
trap cleanup EXIT

"$compiler" -std=gnu11 -O2 -Wall -Wextra -Werror \
	"${cflags[@]}" "$source_file" -o "$temporary" "${libraries[@]}"
chmod 0755 "$temporary"
mv -f -- "$temporary" "$output"
trap - EXIT
printf 'built %s\n' "$output"
