#!/usr/bin/env bash
set -euo pipefail

need_tool() {
	command -v "$1" >/dev/null 2>&1 || {
		printf 'error: required tool is unavailable: %s\n' "$1" >&2
		exit 1
	}
}

need_tool appstreamcli
need_tool cargo
need_tool desktop-file-validate
need_tool g++
need_tool git
need_tool pkg-config
need_tool rpmbuild
need_tool tar

script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root=$(dirname -- "$script_root")
package_name=ae5-control
version=$(awk -F ' *= *' '
	$1 == "version" {
		gsub(/"/, "", $2)
		print $2
		exit
	}
' "$project_root/Cargo.toml")
[[ -n $version ]] || {
	printf 'error: unable to read package version from Cargo.toml\n' >&2
	exit 1
}

spec_version=$(awk '$1 == "Version:" { print $2; exit }' \
	"$project_root/packaging/ae5-control.spec")
[[ $version == "$spec_version" ]] || {
	printf 'error: Cargo version %s does not match RPM version %s\n' \
		"$version" "$spec_version" >&2
	exit 1
}

output_root=${AE5_RPM_OUTPUT_ROOT:-"$project_root/dist"}
if [[ -d $output_root ]] &&
	find "$output_root" -maxdepth 1 -type f \
		-name "$package_name*-$version-*.rpm" -print -quit |
	grep -q .; then
	printf 'error: refusing to overwrite existing RPMs for version %s\n' \
		"$version" >&2
	exit 1
fi

pkg-config --atleast-version=4.10 gtk4 || {
	printf 'error: GTK 4.10 or newer development files are required\n' >&2
	exit 1
}
pkg-config --exists alsa || {
	printf 'error: ALSA development files are required\n' >&2
	exit 1
}
pkg-config --exists \
	Qt6Core Qt6Gui Qt6Qml Qt6Quick Qt6QuickControls2 Qt6QuickShapes || {
	printf 'error: Qt 6 base and declarative development files are required\n' >&2
	exit 1
}

temporary_root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-rpm-build.XXXXXX")
trap 'rm -rf -- "$temporary_root"' EXIT
source_name="$package_name-$version"
source_root="$temporary_root/source/$source_name"
rpm_root="$temporary_root/rpmbuild"
mkdir -p -- "$source_root" "$rpm_root"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}

git -C "$project_root" ls-files \
	--cached --others --exclude-standard -z |
	tar --null -C "$project_root" -T - -cf - |
	tar -C "$source_root" -xf -

(
	cd -- "$source_root"
	mkdir -p .cargo
	cargo vendor --locked vendor > .cargo/config.toml
)

source_date_epoch=$(git -C "$project_root" log -1 --format=%ct)
tar \
	--sort=name \
	--mtime="@$source_date_epoch" \
	--owner=0 \
	--group=0 \
	--numeric-owner \
	-C "$temporary_root/source" \
	-czf "$rpm_root/SOURCES/$source_name.tar.gz" \
	"$source_name"
install -m0644 "$project_root/packaging/ae5-control.spec" \
	"$rpm_root/SPECS/ae5-control.spec"

rpmbuild -ba --nodeps \
	--define "_topdir $rpm_root" \
	"$rpm_root/SPECS/ae5-control.spec"

mapfile -t artifacts < <(
	find "$rpm_root/RPMS" "$rpm_root/SRPMS" \
		-type f -name '*.rpm' -print | sort
)
(( ${#artifacts[@]} > 0 )) || {
	printf 'error: rpmbuild produced no RPM artifacts\n' >&2
	exit 1
}

mkdir -p -- "$output_root"
for artifact in "${artifacts[@]}"; do
	target="$output_root/${artifact##*/}"
	[[ ! -e $target ]] || {
		printf 'error: refusing to overwrite %s\n' "$target" >&2
		exit 1
	}
done
for artifact in "${artifacts[@]}"; do
	install -m0644 "$artifact" "$output_root/${artifact##*/}"
	printf 'wrote %s\n' "$output_root/${artifact##*/}"
done
