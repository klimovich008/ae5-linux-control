#!/usr/bin/env bash
set -euo pipefail

usage() {
	printf 'usage: %s [--from-build|--uninstall]\n' "$0" >&2
	exit 2
}

mode=install
case ${1:-} in
'')
	;;
--from-build)
	mode=from-build
	;;
--uninstall)
	mode=uninstall
	;;
*)
	usage
	;;
esac
[[ $# -le 1 ]] || usage

script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
project_root=$(dirname -- "$script_root")
data_root=${XDG_DATA_HOME:-"$HOME/.local/share"}
config_root=${XDG_CONFIG_HOME:-"$HOME/.config"}
state_root=${XDG_STATE_HOME:-"$HOME/.local/state"}
bin_root=${AE5_USER_BIN_DIR:-"$HOME/.local/bin"}
system_acp_root=${AE5_SYSTEM_ACP_ROOT:-/usr/share/alsa-card-profile}
payload_parent=$data_root/ae5-control
payload_root=$payload_parent/user-install
marker=$payload_root/.ae5-control-user-install
manifest=$state_root/ae5-control/user-install-links.v1
marker_value=ae5-control-user-install-v1
daemon_unit_template=$project_root/packaging/systemd/user/ae5d-user.service.in
daemon_dbus_template=$project_root/packaging/dbus-1/services/io.github.klimovich008.Ae5Control-user.service.in

for root in \
	"$data_root" "$config_root" "$state_root" "$bin_root" \
	"$system_acp_root"; do
	[[ $root == /* && $root != / ]] || {
		printf 'error: installation path must be absolute and narrower than /: %s\n' \
			"$root" >&2
		exit 1
	}
done

payload_sources=(
	"$project_root/target/release/ae5-control"
	"$project_root/target/release/ae5-control-qml"
	"$project_root/target/release/ae5ctl"
	"$project_root/target/release/ae5d"
	"$project_root/scripts/collect-linux-report.sh"
	"$project_root/scripts/install-user.sh"
	"$project_root/packaging/io.github.klimovich008.ae5control.desktop"
	"$project_root/packaging/io.github.klimovich008.ae5control.svg"
	"$project_root/packaging/io.github.klimovich008.ae5control.metainfo.xml"
	"$project_root/packaging/io.github.klimovich008.ae5control-lighting.desktop"
	"$project_root/packaging/wireplumber/90-ae5-control.conf"
	"$project_root/packaging/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf"
	"$project_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf"
	"$project_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-microphone.conf"
	"$project_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-front-microphone.conf"
	"$project_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-line-in.conf"
	"$project_root/LICENSE-APACHE"
	"$project_root/LICENSE-MIT"
)
payload_paths=(
	bin/ae5-control
	bin/ae5-control-qml
	bin/ae5ctl
	bin/ae5d
	bin/ae5-collect-report
	bin/ae5-control-user-install
	share/applications/io.github.klimovich008.ae5control.desktop
	share/icons/hicolor/scalable/apps/io.github.klimovich008.ae5control.svg
	share/metainfo/io.github.klimovich008.ae5control.metainfo.xml
	share/autostart/io.github.klimovich008.ae5control-lighting.desktop
	share/wireplumber/90-ae5-control.conf
	share/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf
	share/alsa-card-profile/mixer/paths/sound-blaster-ae5-output-headphones.conf
	share/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-microphone.conf
	share/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-front-microphone.conf
	share/alsa-card-profile/mixer/paths/sound-blaster-ae5-input-line-in.conf
	share/licenses/LICENSE-APACHE
	share/licenses/LICENSE-MIT
)
payload_modes=(
	0755 0755 0755 0755 0755 0755
	0644 0644 0644 0644 0644 0644 0644 0644 0644 0644
	0644 0644
)
[[ ${#payload_sources[@]} -eq ${#payload_paths[@]} &&
	${#payload_sources[@]} -eq ${#payload_modes[@]} ]] || {
	printf 'error: internal user-install payload lists do not match\n' >&2
	exit 1
}

link_keys=()
link_destinations=()
link_targets=()
link_comparisons=()
link_kinds=()

add_link() {
	link_keys+=("$1")
	link_destinations+=("$2")
	link_targets+=("$3")
	link_comparisons+=("$4")
	link_kinds+=("$5")
}

add_link gui "$bin_root/ae5-control" \
	"$payload_root/bin/ae5-control" "$project_root/target/release/ae5-control" bundle
add_link qml-gui "$bin_root/ae5-control-qml" \
	"$payload_root/bin/ae5-control-qml" "$project_root/target/release/ae5-control-qml" bundle
add_link cli "$bin_root/ae5ctl" \
	"$payload_root/bin/ae5ctl" "$project_root/target/release/ae5ctl" bundle
add_link daemon "$bin_root/ae5d" \
	"$payload_root/bin/ae5d" "$project_root/target/release/ae5d" bundle
add_link report "$bin_root/ae5-collect-report" \
	"$payload_root/bin/ae5-collect-report" "$project_root/scripts/collect-linux-report.sh" bundle
add_link user-installer "$bin_root/ae5-control-user-install" \
	"$payload_root/bin/ae5-control-user-install" "$project_root/scripts/install-user.sh" bundle
add_link desktop \
	"$data_root/applications/io.github.klimovich008.ae5control.desktop" \
	"$payload_root/share/applications/io.github.klimovich008.ae5control.desktop" \
	"$project_root/packaging/io.github.klimovich008.ae5control.desktop" bundle
add_link icon \
	"$data_root/icons/hicolor/scalable/apps/io.github.klimovich008.ae5control.svg" \
	"$payload_root/share/icons/hicolor/scalable/apps/io.github.klimovich008.ae5control.svg" \
	"$project_root/packaging/io.github.klimovich008.ae5control.svg" bundle
add_link metainfo \
	"$data_root/metainfo/io.github.klimovich008.ae5control.metainfo.xml" \
	"$payload_root/share/metainfo/io.github.klimovich008.ae5control.metainfo.xml" \
	"$project_root/packaging/io.github.klimovich008.ae5control.metainfo.xml" bundle
add_link daemon-dbus \
	"$data_root/dbus-1/services/io.github.klimovich008.Ae5Control.service" \
	"$payload_root/share/dbus-1/services/io.github.klimovich008.Ae5Control.service" \
	"$daemon_dbus_template" bundle
add_link daemon-unit \
	"$config_root/systemd/user/ae5d.service" \
	"$payload_root/share/systemd/user/ae5d.service" \
	"$daemon_unit_template" bundle
add_link autostart \
	"$config_root/autostart/io.github.klimovich008.ae5control-lighting.desktop" \
	"$payload_root/share/autostart/io.github.klimovich008.ae5control-lighting.desktop" \
	"$project_root/packaging/io.github.klimovich008.ae5control-lighting.desktop" bundle
add_link wireplumber \
	"$config_root/wireplumber/wireplumber.conf.d/90-ae5-control.conf" \
	"$payload_root/share/wireplumber/90-ae5-control.conf" \
	"$project_root/packaging/wireplumber/90-ae5-control.conf" bundle
add_link acp-profile \
	"$config_root/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf" \
	"$payload_root/share/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf" \
	"$project_root/packaging/alsa-card-profile/mixer/profile-sets/sound-blaster-ae5.conf" bundle
for path in \
	output-headphones input-microphone input-front-microphone input-line-in; do
	add_link "acp-$path" \
		"$config_root/alsa-card-profile/mixer/paths/sound-blaster-ae5-$path.conf" \
		"$payload_root/share/alsa-card-profile/mixer/paths/sound-blaster-ae5-$path.conf" \
		"$project_root/packaging/alsa-card-profile/mixer/paths/sound-blaster-ae5-$path.conf" \
		bundle
done
for path in default.conf 9999-custom.conf; do
	add_link "system-profile-$path" \
		"$config_root/alsa-card-profile/mixer/profile-sets/$path" \
		"$system_acp_root/mixer/profile-sets/$path" \
		"$system_acp_root/mixer/profile-sets/$path" system
done
for path in analog-output-headphones.conf analog-output.conf.common; do
	add_link "system-path-$path" \
		"$config_root/alsa-card-profile/mixer/paths/$path" \
		"$system_acp_root/mixer/paths/$path" \
		"$system_acp_root/mixer/paths/$path" system
done

refresh_desktop_database() {
	if command -v update-desktop-database >/dev/null 2>&1 &&
		[[ -d $data_root/applications ]]; then
		update-desktop-database "$data_root/applications" >/dev/null
	fi
}

standard_user_integration() {
	[[ $data_root == "$HOME/.local/share" &&
		$config_root == "$HOME/.config" ]]
}

reload_user_runtime() {
	standard_user_integration || return 0
	if command -v systemctl >/dev/null 2>&1 &&
		! systemctl --user daemon-reload >/dev/null 2>&1; then
		printf 'warning: the user service manager is unavailable; ae5d activation will be ready after the next login\n' >&2
	elif command -v systemctl >/dev/null 2>&1; then
		systemctl --user try-restart ae5d.service >/dev/null 2>&1 || true
	fi
	if command -v busctl >/dev/null 2>&1 &&
		! busctl --user call \
			org.freedesktop.DBus /org/freedesktop/DBus \
			org.freedesktop.DBus ReloadConfig >/dev/null 2>&1; then
		printf 'warning: the session bus did not reload ae5d activation metadata; activation will be ready after the next login\n' >&2
	fi
}

stop_user_daemon() {
	standard_user_integration || return 0
	command -v systemctl >/dev/null 2>&1 || return 0
	systemctl --user stop ae5d.service >/dev/null 2>&1 || true
}

uninstall() {
	local index destination target kind key owned

	if [[ -e $manifest || -L $manifest ]]; then
		[[ -f $manifest && ! -L $manifest ]] || {
			printf 'error: refusing unexpected install manifest: %s\n' \
				"$manifest" >&2
			exit 1
		}
	fi
	if [[ -e $payload_root || -L $payload_root ]]; then
		[[ -d $payload_root && ! -L $payload_root &&
			-f $marker && ! -L $marker ]] || {
			printf 'error: refusing unexpected payload path: %s\n' \
				"$payload_root" >&2
			exit 1
		}
		[[ $(<"$marker") == "$marker_value" ]] || {
			printf 'error: refusing payload with an unknown marker: %s\n' \
				"$payload_root" >&2
			exit 1
		}
	fi

	stop_user_daemon
	for index in "${!link_keys[@]}"; do
		key=${link_keys[$index]}
		destination=${link_destinations[$index]}
		target=${link_targets[$index]}
		kind=${link_kinds[$index]}
		owned=no
		if [[ -f $manifest ]] && grep -Fxq -- "$key" "$manifest"; then
			owned=yes
		fi
		if [[ -L $destination && $(readlink -- "$destination") == "$target" ]] &&
			[[ $kind == bundle || $owned == yes ]]; then
			rm -f -- "$destination"
		fi
	done

	if [[ -d $payload_root ]]; then
		find "$payload_root" -depth -delete
		rmdir --ignore-fail-on-non-empty -- "$payload_parent" 2>/dev/null || true
	fi
	rm -f -- "$manifest"
	refresh_desktop_database
	reload_user_runtime
	printf 'AE-5 Control user installation removed; profiles and settings were preserved\n'
	printf 'restart WirePlumber when no audio stream is active, or log out and back in\n'
}

if [[ $mode == uninstall ]]; then
	uninstall
	exit
fi

if [[ $mode == install ]]; then
	command -v cargo >/dev/null 2>&1 || {
		printf 'error: cargo is required to build the user installation\n' >&2
		exit 1
	}
	(
		cd -- "$project_root"
		cargo build --locked --release --all-features
	)
fi

for source in "${payload_sources[@]}" \
	"$daemon_unit_template" "$daemon_dbus_template"; do
	[[ -f $source ]] || {
		printf 'error: required installation source is missing: %s\n' "$source" >&2
		exit 1
	}
done
for index in "${!link_keys[@]}"; do
	if [[ ${link_kinds[$index]} == system &&
		! -f ${link_targets[$index]} ]]; then
		printf 'error: required system ACP file is missing: %s\n' \
			"${link_targets[$index]}" >&2
		exit 1
	fi
done
for index in "${!link_keys[@]}"; do
	destination=${link_destinations[$index]}
	target=${link_targets[$index]}
	comparison=${link_comparisons[$index]}
	if [[ ! -e $destination && ! -L $destination ]]; then
		continue
	fi
	if [[ -L $destination && $(readlink -- "$destination") == "$target" ]]; then
		continue
	fi
	if [[ -f $destination && -f $comparison ]] &&
		cmp -s -- "$destination" "$comparison"; then
		continue
	fi
	printf 'error: refusing to replace existing path: %s\n' "$destination" >&2
	exit 1
done
if [[ -e $payload_root || -L $payload_root ]]; then
	[[ -d $payload_root && ! -L $payload_root &&
		-f $marker && ! -L $marker ]] || {
		printf 'error: refusing unexpected payload path: %s\n' "$payload_root" >&2
		exit 1
	}
	[[ $(<"$marker") == "$marker_value" ]] || {
		printf 'error: refusing payload with an unknown marker: %s\n' \
			"$payload_root" >&2
		exit 1
	}
fi
if [[ -e $manifest || -L $manifest ]]; then
	[[ -f $manifest && ! -L $manifest ]] || {
		printf 'error: refusing unexpected install manifest: %s\n' \
			"$manifest" >&2
		exit 1
	}
fi

had_payload=no
if [[ -d $payload_root ]]; then
	had_payload=yes
fi
if [[ -e $payload_parent || -L $payload_parent ]]; then
	[[ -d $payload_parent && ! -L $payload_parent ]] || {
		printf 'error: refusing unexpected payload parent: %s\n' \
			"$payload_parent" >&2
		exit 1
	}
else
	install -d -m0755 "$payload_parent"
fi
staging_root=$(mktemp -d "$payload_parent/.user-install.new.XXXXXX")
old_payload=
cleanup_staging() {
	local status=$?
	trap - EXIT
	if [[ -n ${old_payload:-} && -d $old_payload ]]; then
		if [[ ! -e $payload_root && ! -L $payload_root ]]; then
			mv -- "$old_payload" "$payload_root" || {
				printf 'error: failed to restore previous payload: %s\n' \
					"$old_payload" >&2
			}
		elif [[ -d $payload_root && ! -L $payload_root ]]; then
			find "$old_payload" -depth -delete || true
		fi
	fi
	if [[ -n ${staging_root:-} && -d $staging_root ]]; then
		find "$staging_root" -depth -delete || true
	fi
	exit "$status"
}
trap cleanup_staging EXIT

for index in "${!payload_sources[@]}"; do
	install -Dm"${payload_modes[$index]}" \
		"${payload_sources[$index]}" "$staging_root/${payload_paths[$index]}"
	cmp -s -- "${payload_sources[$index]}" \
		"$staging_root/${payload_paths[$index]}" || {
		printf 'error: staged payload verification failed: %s\n' \
			"${payload_paths[$index]}" >&2
		exit 1
	}
done

render_daemon_metadata() {
	local source=$1
	local destination=$2
	local daemon_path=$payload_root/bin/ae5d
	local escaped_path line

	[[ $daemon_path != *$'\n'* && $daemon_path != *$'\r'* ]] || {
		printf 'error: daemon path contains a line break: %s\n' "$daemon_path" >&2
		exit 1
	}
	escaped_path=${daemon_path//\\/\\\\}
	escaped_path=${escaped_path//\"/\\\"}
	install -d -m0755 "$(dirname -- "$destination")"
	: > "$destination"
	while IFS= read -r line || [[ -n $line ]]; do
		printf '%s\n' "${line//@AE5D_EXEC@/$escaped_path}" >> "$destination"
	done < "$source"
	chmod 0644 "$destination"
}

render_daemon_metadata "$daemon_unit_template" \
	"$staging_root/share/systemd/user/ae5d.service"
render_daemon_metadata "$daemon_dbus_template" \
	"$staging_root/share/dbus-1/services/io.github.klimovich008.Ae5Control.service"
grep -Fxq "ExecStart=\"$payload_root/bin/ae5d\"" \
	"$staging_root/share/systemd/user/ae5d.service"
grep -Fxq "Exec=\"$payload_root/bin/ae5d\"" \
	"$staging_root/share/dbus-1/services/io.github.klimovich008.Ae5Control.service"
printf '%s\n' "$marker_value" > \
	"$staging_root/.ae5-control-user-install"
chmod 0644 "$staging_root/.ae5-control-user-install"

if [[ -d $payload_root ]]; then
	old_payload=$(mktemp -d "$payload_parent/.user-install.old.XXXXXX")
	rmdir -- "$old_payload"
	mv -- "$payload_root" "$old_payload"
fi
if ! mv -- "$staging_root" "$payload_root"; then
	if [[ -n $old_payload && -d $old_payload &&
		! -e $payload_root && ! -L $payload_root ]]; then
		mv -- "$old_payload" "$payload_root"
	fi
	exit 1
fi
staging_root=
if [[ -n $old_payload && -d $old_payload ]]; then
	find "$old_payload" -depth -delete
fi
trap - EXIT

install -d -m0700 "$(dirname -- "$manifest")"
touch "$manifest"
chmod 0600 "$manifest"

for index in "${!link_keys[@]}"; do
	key=${link_keys[$index]}
	destination=${link_destinations[$index]}
	target=${link_targets[$index]}
	comparison=${link_comparisons[$index]}
	if [[ ! -e $destination && ! -L $destination ]]; then
		install -d -m0755 "$(dirname -- "$destination")"
		ln -s -- "$target" "$destination"
		grep -Fxq -- "$key" "$manifest" || printf '%s\n' "$key" >> "$manifest"
	elif [[ -L $destination && $(readlink -- "$destination") == "$target" ]]; then
		:
	elif [[ -f $destination && -f $comparison ]] &&
		cmp -s -- "$destination" "$comparison"; then
		printf 'kept identical existing path: %s\n' "$destination"
	fi
done

refresh_desktop_database
reload_user_runtime
if [[ $had_payload == yes ]]; then
	printf 'AE-5 Control upgraded for %s without root\n' \
		"${USER:-the current user}"
else
	printf 'AE-5 Control installed for %s without root\n' \
		"${USER:-the current user}"
fi
printf 'launch it from the application menu or run %s/ae5-control-qml\n' "$bin_root"
printf 'restart WirePlumber when no audio stream is active, or log out and back in\n'
printf 'onboard lighting still requires the project kernel patch and system udev rule\n'
case :$PATH: in
*:"$bin_root":*)
	;;
*)
	printf 'warning: add %s to PATH for ae5ctl and desktop autostart\n' "$bin_root" >&2
	;;
esac
