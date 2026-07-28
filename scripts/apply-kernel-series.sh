#!/usr/bin/env bash
set -euo pipefail

usage() {
	cat >&2 <<EOF
usage: $0 [--check|--apply] SOURCE_TREE [SERIES_FILE]

  --check  Test the complete series in an isolated temporary tree (default).
  --apply  Apply the complete series to SOURCE_TREE transactionally.
EOF
}

fail() {
	printf 'error: %s\n' "$1" >&2
	exit 1
}

need_tool() {
	command -v "$1" >/dev/null 2>&1 ||
		fail "required tool is unavailable: $1"
}

mode=check
case ${1:-} in
--check)
	shift
	;;
--apply)
	mode=apply
	shift
	;;
--help|-h)
	usage
	exit 0
	;;
esac

[[ $# -ge 1 && $# -le 2 ]] || {
	usage
	exit 2
}

for tool in awk cmp cp git mktemp readlink sha256sum sort; do
	need_tool "$tool"
done

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
source_tree=$(readlink -e -- "$1") ||
	fail "source tree does not exist: $1"
series_file=$(readlink -e -- "${2:-"$repo_root/kernel/series"}") ||
	fail "series file does not exist: ${2:-"$repo_root/kernel/series"}"

[[ -d $source_tree ]] || fail "source tree is not a directory: $source_tree"
for required in Makefile sound/hda/codecs/ca0132.c \
	sound/hda/codecs/Kconfig sound/hda/codecs/Makefile; do
	[[ -f $source_tree/$required ]] ||
		fail "not a Linux source tree; missing $required"
done

patches=()
while IFS= read -r entry || [[ -n $entry ]]; do
	entry=${entry%%#*}
	entry=${entry#"${entry%%[![:space:]]*}"}
	entry=${entry%"${entry##*[![:space:]]}"}
	[[ -n $entry ]] || continue
	[[ $entry != /* && $entry != *..* ]] ||
		fail "series entry must be a repository-relative path: $entry"
	[[ $entry != *[[:space:]]* ]] ||
		fail "series entry may not contain whitespace: $entry"
	patch_path=$repo_root/$entry
	[[ -f $patch_path ]] || fail "series patch does not exist: $entry"
	patches+=("$patch_path")
done < "$series_file"
(( ${#patches[@]} > 0 )) || fail "series contains no patches: $series_file"

mapfile -t touched_paths < <(
	for patch_path in "${patches[@]}"; do
		awk '
			$1 == "---" && $2 ~ /^a\// { print substr($2, 3) }
			$1 == "+++" && $2 ~ /^b\// { print substr($2, 3) }
		' "$patch_path"
	done | sort -u
)
(( ${#touched_paths[@]} > 0 )) ||
	fail "series does not name any source paths"

for relative_path in "${touched_paths[@]}"; do
	[[ $relative_path != /* && $relative_path != *..* ]] ||
		fail "patch path escapes the source tree: $relative_path"
done

applied=()
rollback() {
	local index
	local rollback_failed=0

	for ((index=${#applied[@]} - 1; index >= 0; index--)); do
		git -C "$target_tree" apply --reverse "${applied[$index]}" ||
			rollback_failed=1
	done
	applied=()
	return "$rollback_failed"
}

apply_series() {
	local patch_path

	for patch_path in "${patches[@]}"; do
		if ! git -C "$target_tree" apply --check --whitespace=error-all \
			"$patch_path"; then
			printf 'error: incompatible patch: %s\n' \
				"${patch_path#"$repo_root"/}" >&2
			rollback ||
				printf 'error: rollback was incomplete in %s\n' \
					"$target_tree" >&2
			return 1
		fi
		if ! git -C "$target_tree" apply --whitespace=error-all \
			"$patch_path"; then
			printf 'error: failed to apply checked patch: %s\n' \
				"${patch_path#"$repo_root"/}" >&2
			rollback ||
				printf 'error: rollback was incomplete in %s\n' \
					"$target_tree" >&2
			return 1
		fi
		applied+=("$patch_path")
	done
}

series_sha256=$(sha256sum "$series_file" | awk '{ print $1 }')
patchset_sha256=$(
	for patch_path in "${patches[@]}"; do
		patch_sha256=$(sha256sum "$patch_path" | awk '{ print $1 }')
		printf '%s  %s\n' "$patch_sha256" \
			"${patch_path#"$repo_root"/}"
	done | sha256sum | awk '{ print $1 }'
)
source_commit=not-a-git-worktree
if git -C "$source_tree" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
	source_commit=$(git -C "$source_tree" rev-parse HEAD)
fi

if [[ $mode == apply ]]; then
	if [[ $source_commit != not-a-git-worktree ]] &&
		[[ -n $(git -C "$source_tree" status --porcelain \
			--untracked-files=all) ]]; then
		fail "refusing to patch a dirty Git worktree: $source_tree"
	fi

	target_tree=$source_tree
	apply_series || exit 1
	if [[ $source_commit != not-a-git-worktree ]]; then
		if ! git -C "$target_tree" diff --check; then
			rollback ||
				printf 'error: rollback was incomplete in %s\n' \
					"$target_tree" >&2
			fail "patched tree contains whitespace errors"
		fi
	fi

	printf 'mode=apply\n'
	printf 'source=%s\n' "$source_tree"
	printf 'base=%s\n' "$source_commit"
	printf 'series_sha256=%s\n' "$series_sha256"
	printf 'patchset_sha256=%s\n' "$patchset_sha256"
	printf 'patch_count=%s\n' "${#patches[@]}"
	printf 'result=applied\n'
	printf 'install_performed=no\n'
	exit 0
fi

temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-kernel-series.XXXXXX")
target_tree=$temporary_root/source
mkdir -p -- "$target_tree"
cleanup() {
	rm -rf -- "$temporary_root"
}
trap cleanup EXIT

for relative_path in "${touched_paths[@]}"; do
	if [[ -e $source_tree/$relative_path ]]; then
		(
			cd -- "$source_tree"
			cp -a --parents -- "$relative_path" "$target_tree"
		)
	fi
done

apply_series || exit 1
rollback || fail "reverse application failed in the isolated check tree"

for relative_path in "${touched_paths[@]}"; do
	if [[ -e $source_tree/$relative_path ]]; then
		[[ -e $target_tree/$relative_path ]] ||
			fail "round trip removed source path: $relative_path"
		cmp -s -- "$source_tree/$relative_path" \
			"$target_tree/$relative_path" ||
			fail "round trip changed source path: $relative_path"
	else
		[[ ! -e $target_tree/$relative_path ]] ||
			fail "round trip left a new path behind: $relative_path"
	fi
done

printf 'mode=check\n'
printf 'source=%s\n' "$source_tree"
printf 'base=%s\n' "$source_commit"
printf 'series_sha256=%s\n' "$series_sha256"
printf 'patchset_sha256=%s\n' "$patchset_sha256"
printf 'patch_count=%s\n' "${#patches[@]}"
printf 'result=compatible\n'
printf 'source_modified=no\n'
