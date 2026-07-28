#!/usr/bin/env bash
set -eEuo pipefail

readonly pstore_guid=cfc8fc79-be2e-4ddc-97f0-9f98bfe298a0
readonly pstore_arguments='efi_pstore.pstore_disable=0 printk.always_kmsg_dump=Y'

usage() {
	cat >&2 <<EOF
usage:
  $0 KERNEL_RPM EXPECTED_RELEASE
  sudo $0 --install KERNEL_RPM EXPECTED_RELEASE
  $0 --self-test

Without --install, verify the candidate without changing the system.
--install installs it side by side, restores the current stock saved entry,
adds shutdown-evidence arguments to the candidate entry only, and selects the
candidate for the next boot only. It never reboots.
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

grub_value() {
	local key=$1

	awk -F= -v key="$key" '
		$1 == key {
			count++
			value = substr($0, length(key) + 2)
		}
		END {
			if (count > 1)
				exit 1
			if (count == 1)
				printf "%s", value
		}
	'
}

pstore_file_count() {
	local root=$1

	find "$root" -maxdepth 1 -type f \
		-name "dump-type0-*-$pstore_guid" -printf x |
		wc -c
}

bls_has_option() {
	local file=$1 option=$2

	awk -v option="$option" '
		$1 == "options" {
			for (field = 2; field <= NF; field++) {
				if ($field == option)
					found = 1
			}
		}
		END {
			exit !found
		}
	' "$file"
}

restore_boot_selection() {
	local status=$?

	trap - ERR
	if [[ ${installation_started:-no} == yes ]]; then
		grub2-set-default "$stock_saved" ||
			printf 'error: failed to restore saved entry %s\n' \
				"$stock_saved" >&2
		grub2-editenv - unset next_entry ||
			printf 'error: failed to clear next_entry\n' >&2
	fi
	printf 'error: candidate installation did not complete safely\n' >&2
	return "$status"
}

abort_after_install() {
	local message=$1

	grub2-set-default "$stock_saved" ||
		printf 'error: failed to restore saved entry %s\n' \
			"$stock_saved" >&2
	grub2-editenv - unset next_entry ||
		printf 'error: failed to clear next_entry\n' >&2
	fail "$message"
}

install_candidate() {
	local rpm_path=$1 release=$2 package_nevra=$3
	local boot_free_kib current_env current_saved current_next
	local running_release secure_boot stock_kernel stock_release
	local candidate_entry candidate_id candidate_kernel
	local pstore_records stock_saved installation_started=no
	local -a candidate_entries

	[[ $EUID -eq 0 || ${self_test_mode:-no} == yes ]] ||
		fail '--install must be run as root'
	grep -Eq \
		"^[[:space:]]*GRUB_DEFAULT[[:space:]]*=[[:space:]]*(saved|\"saved\"|'saved')[[:space:]]*$" \
		"$grub_defaults" ||
		fail 'GRUB_DEFAULT must be saved'
	secure_boot=$(mokutil --sb-state 2>&1 || true)
	grep -Eqi \
		'SecureBoot disabled|This system does not support Secure Boot|This system doesn.t support Secure Boot' \
		<<< "$secure_boot" ||
		fail 'Secure Boot must be disabled for this locally signed kernel'

	current_env=$(grub2-editenv - list)
	stock_saved=$(grub_value saved_entry <<< "$current_env") ||
		fail 'unable to read one saved_entry from grubenv'
	current_next=$(grub_value next_entry <<< "$current_env") ||
		fail 'unable to read next_entry from grubenv'
	[[ -n $stock_saved ]] || fail 'saved_entry is empty'
	[[ -z $current_next ]] ||
		fail "another one-shot boot is already pending: $current_next"
	[[ -f $bls_dir/$stock_saved.conf ]] ||
		fail "saved BLS entry is missing: $stock_saved"

	stock_kernel=$(grubby --default-kernel)
	stock_release=${stock_kernel##*/vmlinuz-}
	[[ -n $stock_release && $stock_release != "$stock_kernel" ]] ||
		fail "unable to identify the saved kernel: $stock_kernel"
	[[ $stock_release != "$release" ]] ||
		fail 'the candidate is already the saved kernel'
	[[ -d $modules_dir/$stock_release ]] ||
		fail "saved kernel module tree is missing: $stock_release"
	grep -qxF "version $stock_release" "$bls_dir/$stock_saved.conf" ||
		fail 'saved BLS entry does not match grubby default kernel'

	running_release=$(uname -r)
	[[ $running_release != "$release" ]] ||
		fail 'refusing to install the currently running release'
	if rpm -q "$package_nevra" >/dev/null 2>&1; then
		fail "candidate package is already installed: $package_nevra"
	fi
	[[ -d $efi_var_root ]] || fail 'EFI variable filesystem is unavailable'
	pstore_records=$(pstore_file_count "$efi_var_root")
	((pstore_records == 0)) ||
		fail "EFI pstore already contains $pstore_records record parts"

	boot_free_kib=$(df -Pk "$boot_dir" |
		awk 'NR == 2 && $4 ~ /^[0-9]+$/ { print $4 }')
	[[ -n $boot_free_kib ]] || fail 'unable to measure free /boot space'
	((boot_free_kib >= 524288)) ||
		fail "/boot has ${boot_free_kib} KiB free; at least 524288 KiB is required"

	rpm -i --test "$rpm_path"

	installation_started=yes
	trap restore_boot_selection ERR
	rpm -i "$rpm_path"
	grub2-set-default "$stock_saved"

	shopt -s nullglob
	candidate_entries=("$bls_dir"/*-"$release".conf)
	shopt -u nullglob
	(( ${#candidate_entries[@]} == 1 )) ||
		abort_after_install \
			"expected one candidate BLS entry, found ${#candidate_entries[@]}"
	candidate_entry=${candidate_entries[0]}
	grep -qxF "version $release" "$candidate_entry" ||
		abort_after_install 'candidate BLS entry has the wrong version'
	candidate_kernel=$boot_dir/vmlinuz-$release
	[[ -f $candidate_kernel ]] ||
		abort_after_install "candidate kernel image is missing: $candidate_kernel"
	grubby --update-kernel="$candidate_kernel" --args="$pstore_arguments"
	bls_has_option "$candidate_entry" 'efi_pstore.pstore_disable=0' ||
		abort_after_install 'candidate BLS entry did not enable EFI pstore'
	bls_has_option "$candidate_entry" 'printk.always_kmsg_dump=Y' ||
		abort_after_install \
			'candidate BLS entry did not enable normal-shutdown log dumping'
	candidate_id=${candidate_entry##*/}
	candidate_id=${candidate_id%.conf}

	current_env=$(grub2-editenv - list)
	current_saved=$(grub_value saved_entry <<< "$current_env") ||
		abort_after_install 'unable to verify the restored saved_entry'
	[[ $current_saved == "$stock_saved" ]] ||
		abort_after_install \
			"saved_entry changed from $stock_saved to $current_saved"

	grub2-reboot "$candidate_id"
	current_env=$(grub2-editenv - list)
	current_saved=$(grub_value saved_entry <<< "$current_env") ||
		abort_after_install 'unable to verify saved_entry after scheduling'
	current_next=$(grub_value next_entry <<< "$current_env") ||
		abort_after_install 'unable to verify next_entry after scheduling'
	[[ $current_saved == "$stock_saved" ]] ||
		abort_after_install \
			"saved_entry changed from $stock_saved to $current_saved"
	[[ $current_next == "$candidate_id" ]] ||
		abort_after_install \
			"next_entry is $current_next instead of $candidate_id"

	trap - ERR
	printf 'package=%s\n' "$package_nevra"
	printf 'release=%s\n' "$release"
	printf 'stock_release=%s\n' "$stock_release"
	printf 'stock_saved_entry=%s\n' "$stock_saved"
	printf 'candidate_next_entry=%s\n' "$candidate_id"
	printf 'candidate_kernel_arguments=%s\n' "$pstore_arguments"
	printf 'existing_pstore_records=0\n'
	printf 'install_performed=yes\n'
	printf 'reboot_performed=no\n'
}

self_test() {
	local test_saved=machine-stock test_next=
	local test_release=7.1.4-ae5-test
	local output output_file

	self_test_mode=yes
	self_test_root=$(
		mktemp -d "${TMPDIR:-/tmp}/ae5-kernel-install-test.XXXXXX"
	)
	trap 'find "$self_test_root" -depth -delete' EXIT
	boot_dir=$self_test_root/boot
	bls_dir=$boot_dir/loader/entries
	modules_dir=$self_test_root/lib/modules
	grub_defaults=$self_test_root/default-grub
	efi_var_root=$self_test_root/efivars
	mkdir -p -- "$bls_dir" "$modules_dir/stock-kernel" "$efi_var_root"
	printf "GRUB_DEFAULT='saved'\n" > "$grub_defaults"
	printf 'version stock-kernel\n' > "$bls_dir/$test_saved.conf"

	mokutil() {
		printf "This system doesn't support Secure Boot\n" >&2
		return 1
	}
	grub2-editenv() {
		if [[ $2 == list ]]; then
			printf 'saved_entry=%s\nnext_entry=%s\n' \
				"$test_saved" "$test_next"
		elif [[ $2 == unset && $3 == next_entry ]]; then
			test_next=
		else
			return 1
		fi
	}
	grub2-set-default() {
		test_saved=$1
	}
	grub2-reboot() {
		test_next=$1
	}
	grubby() {
		case $1 in
		--default-kernel)
			printf '/boot/vmlinuz-stock-kernel\n'
			;;
		--update-kernel=*)
			[[ $1 == "--update-kernel=$boot_dir/vmlinuz-$test_release" ]]
			[[ ${2:-} == "--args=$pstore_arguments" ]]
			printf 'version %s\noptions quiet %s\n' \
				"$test_release" "$pstore_arguments" \
				> "$bls_dir/machine-$test_release.conf"
			;;
		*)
			return 1
			;;
		esac
	}
	uname() {
		[[ $1 == -r ]] || return 1
		printf 'stock-kernel\n'
	}
	df() {
		printf 'Filesystem 1024-blocks Used Available Capacity Mounted on\n'
		printf 'test 1048576 1 1048575 1%% %s\n' "$boot_dir"
	}
	rpm() {
		case ${1:-} in
		-q)
			return 1
			;;
		-i)
			if [[ ${2:-} == --test ]]; then
				return
			fi
			printf 'version %s\n' "$test_release" \
				> "$bls_dir/machine-$test_release.conf"
			: > "$boot_dir/vmlinuz-$test_release"
			test_saved=machine-"$test_release"
			;;
		*)
			return 1
			;;
		esac
	}

	output_file=$self_test_root/output
	install_candidate /tmp/candidate.rpm "$test_release" \
		kernel-test.x86_64 > "$output_file"
	output=$(< "$output_file")
	[[ $test_saved == machine-stock ]] ||
		fail 'self-test did not restore the stock saved entry'
	[[ $test_next == machine-"$test_release" ]] ||
		fail 'self-test did not schedule the candidate once'
	grep -Fq 'install_performed=yes' <<< "$output" ||
		fail 'self-test did not report installation'
	bls_has_option "$bls_dir/machine-$test_release.conf" \
		'efi_pstore.pstore_disable=0' ||
		fail 'self-test did not enable EFI pstore for the candidate'
	bls_has_option "$bls_dir/machine-$test_release.conf" \
		'printk.always_kmsg_dump=Y' ||
		fail 'self-test did not enable normal-shutdown log dumping'
	if bls_has_option "$bls_dir/$test_saved.conf" \
		'efi_pstore.pstore_disable=0'; then
		fail 'self-test changed the stock BLS entry'
	fi

	test_next=
	printf 'stale\n' \
		> "$efi_var_root/dump-type0-1-1-1780000000-D-$pstore_guid"
	if (install_candidate /tmp/candidate.rpm "$test_release" \
		kernel-test.x86_64) >/dev/null 2>&1; then
		fail 'self-test accepted stale EFI pstore records'
	fi
	find "$efi_var_root" -type f -delete

	test_next=already-pending
	if (install_candidate /tmp/candidate.rpm "$test_release" \
		kernel-test.x86_64) >/dev/null 2>&1; then
		fail 'self-test accepted an existing one-shot override'
	fi

	find "$self_test_root" -depth -delete
	trap - EXIT
	printf 'kernel test installer self-test passed\n'
}

self_test_mode=no
mode=check
case ${1:-} in
--install)
	mode=install
	shift
	;;
--self-test)
	[[ $# -eq 1 ]] || {
		usage
		exit 2
	}
	self_test
	exit
	;;
--help|-h)
	usage
	exit
	;;
esac

[[ $# -eq 2 ]] || {
	usage
	exit 2
}

for tool in awk readlink; do
	need_tool "$tool"
done

repo_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)
rpm_path=$(readlink -e -- "$1") || fail "RPM does not exist: $1"
expected_release=$2
[[ $expected_release =~ ^[A-Za-z0-9][A-Za-z0-9._+-]*$ ]] ||
	fail "invalid expected release: $expected_release"

verification=$(
	"$repo_root/scripts/check-host-kernel-rpm.sh" \
		"$rpm_path" "$expected_release"
)
printf '%s\n' "$verification"
release=$(awk -F= '$1 == "release" { print $2 }' <<< "$verification")
package_nevra=$(awk -F= '$1 == "package" { print $2 }' <<< "$verification")
[[ $release == "$expected_release" && -n $package_nevra ]] ||
	fail 'verifier output is incomplete'

if [[ $mode == check ]]; then
	printf 'next_step=sudo %q --install %q %q\n' \
		"$0" "$rpm_path" "$release"
	exit
fi

for tool in df find grep grub2-editenv grub2-reboot grub2-set-default grubby \
	mokutil rpm uname wc; do
	need_tool "$tool"
done
boot_dir=/boot
bls_dir=$boot_dir/loader/entries
modules_dir=/lib/modules
grub_defaults=/etc/default/grub
efi_var_root=/sys/firmware/efi/efivars
install_candidate "$rpm_path" "$release" "$package_nevra"
