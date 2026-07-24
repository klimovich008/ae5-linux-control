#!/usr/bin/env bash
set -euo pipefail

[[ $# == 1 ]] || {
	printf 'usage: %s LINUX_SOURCE\n' "${0##*/}" >&2
	exit 2
}

readonly repository_root=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
readonly patch_file="$repository_root/kernel/ca0132-wedge-angle-default.patch"
readonly linux_source=$1

[[ -f $linux_source/sound/hda/codecs/ca0132.c ]] || {
	printf 'error: not a Linux source tree: %s\n' "$linux_source" >&2
	exit 1
}
[[ -f $linux_source/scripts/checkpatch.pl ]] || {
	printf 'error: upstream checkpatch.pl is unavailable: %s\n' "$linux_source" >&2
	exit 1
}
git -C "$linux_source" rev-parse --is-inside-work-tree >/dev/null 2>&1 || {
	printf 'error: Linux source is not a Git worktree: %s\n' "$linux_source" >&2
	exit 1
}
git -C "$linux_source" diff --quiet HEAD -- sound/hda/codecs/ca0132.c || {
	printf 'error: ca0132.c has local changes: %s\n' "$linux_source" >&2
	exit 1
}

printf 'linux_head=%s\n' "$(git -C "$linux_source" rev-parse HEAD)"
(
	cd -- "$linux_source"
	git apply --check "$patch_file"
	perl scripts/checkpatch.pl --no-tree --strict "$patch_file"
)
