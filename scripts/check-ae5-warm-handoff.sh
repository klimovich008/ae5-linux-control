#!/usr/bin/env bash
set -euo pipefail

readonly pstore_guid=cfc8fc79-be2e-4ddc-97f0-9f98bfe298a0

usage() {
	cat >&2 <<EOF
usage:
  $0 --prepare EXPECTED_CURRENT_RELEASE
  AE5_WARM_HANDOFF_CONFIRMED=1 $0 --check EXPECTED_PREVIOUS_RELEASE
  $0 --self-test

Run --prepare in the shutdown candidate immediately before the warm handoff.
Set the acknowledgement for --check only when motherboard power remained on.
EOF
	exit 2
}

fail() {
	printf 'error: %s\n' "$1" >&2
	exit 1
}

marker_count() {
	local marker=$1 file=$2

	awk -v marker="$marker" '
		index($0, marker) {
			count++
		}
		END {
			print count + 0
		}
	' "$file"
}

pstore_file_count() {
	local root=$1

	find "$root" -maxdepth 1 -type f \
		-name "dump-type0-*-$pstore_guid" -printf x |
		wc -c
}

decode_pstore() {
	local root=$1

	python3 - "$root" "$pstore_guid" <<'PY'
import pathlib
import re
import sys
import zlib

root = pathlib.Path(sys.argv[1])
guid = sys.argv[2]
pattern = re.compile(
    rf"^dump-type0-(\d+)-(-?\d+)-(\d+)-([CD])-{re.escape(guid)}$"
)
records = []
for path in root.iterdir():
    match = pattern.fullmatch(path.name)
    if not match:
        continue
    part, count, timestamp, kind = match.groups()
    data = path.read_bytes()
    if len(data) <= 4:
        raise SystemExit(f"EFI pstore record is truncated: {path.name}")
    payload = data[4:]
    if kind == "C":
        try:
            payload = zlib.decompress(payload, -zlib.MAX_WBITS)
        except zlib.error as error:
            raise SystemExit(
                f"cannot decompress EFI pstore record {path.name}: {error}"
            ) from error
    records.append((int(part), int(count), int(timestamp), payload))

if not records:
    raise SystemExit("no EFI pstore shutdown record found")
if len({(count, timestamp) for _, count, timestamp, _ in records}) != 1:
    raise SystemExit("EFI pstore contains more than one dump")
records.sort()
if [part for part, _, _, _ in records] != list(range(1, len(records) + 1)):
    raise SystemExit("EFI pstore dump parts are incomplete or ambiguous")
for _, _, _, payload in records:
    sys.stdout.buffer.write(payload)
PY
}

check_preparation() {
	local expected_release=$1 current_release=$2 taint_file=$3
	local disable_file=$4 always_dump_file=$5 backend_file=$6
	local efivar_root=$7 backend disable always_dump taint records

	[[ $current_release == "$expected_release" ]] ||
		fail "running kernel is $current_release, expected $expected_release"
	for file in "$taint_file" "$disable_file" "$always_dump_file" "$backend_file"; do
		[[ -r $file ]] || fail "required kernel state is unreadable: $file"
	done
	read -r taint < "$taint_file"
	[[ $taint == 0 ]] || fail "kernel taint is $taint, expected 0"
	read -r disable < "$disable_file"
	[[ $disable == N ]] || fail 'EFI pstore is disabled in the candidate boot'
	read -r always_dump < "$always_dump_file"
	[[ $always_dump == Y ]] ||
		fail 'normal-shutdown kernel log dumping is disabled'
	read -r backend < "$backend_file"
	[[ $backend == efi_pstore ]] ||
		fail "pstore backend is ${backend:-unavailable}, expected efi_pstore"
	[[ -d $efivar_root ]] || fail 'EFI variable filesystem is unavailable'
	records=$(pstore_file_count "$efivar_root")
	((records == 0)) ||
		fail "EFI pstore already contains $records record parts"

	printf 'candidate_kernel=%s\n' "$current_release"
	printf 'kernel_taint=%s\n' "$taint"
	printf 'pstore_backend=%s\n' "$backend"
	printf 'normal_shutdown_dump=enabled\n'
	printf 'existing_pstore_records=0\n'
	printf 'shutdown_evidence_preparation=pass\n'
}

check_handoff() {
	local previous_release=$1 current_release=$2
	local previous_log=$3 shutdown_log=$4 current_log=$5 taint_file=$6
	local previous_downloads previous_resets current_downloads taint
	local power_failures reset_failures shutdown_dumps

	[[ $previous_release =~ ^[A-Za-z0-9][A-Za-z0-9._+-]*$ &&
		${#previous_release} -le 128 ]] ||
		fail 'previous release contains unsafe characters'
	[[ $current_release =~ ^[A-Za-z0-9][A-Za-z0-9._+-]*$ &&
		${#current_release} -le 128 ]] ||
		fail 'current release contains unsafe characters'
	for file in "$previous_log" "$shutdown_log" "$current_log"; do
		[[ -s $file ]] || fail "required evidence is empty: $file"
	done
	[[ -r $taint_file ]] || fail 'kernel taint state is unreadable'

	[[ $(marker_count "Linux version $previous_release " "$previous_log") == 1 ]] ||
		fail "previous boot did not identify exactly one $previous_release kernel"
	[[ $(marker_count "Linux version $current_release " "$current_log") == 1 ]] ||
		fail "current journal does not identify running kernel $current_release"

	previous_downloads=$(
		marker_count 'ca0132 DSP downloaded and running' "$previous_log"
	)
	((previous_downloads >= 1)) ||
		fail 'previous boot did not initialize the CA0132 DSP'
	shutdown_dumps=$(marker_count 'Shutdown#' "$shutdown_log")
	((shutdown_dumps >= 1)) ||
		fail 'EFI pstore record is not a normal-shutdown dump'
	previous_resets=$(
		marker_count 'AE-5 DSP reset at shutdown' "$shutdown_log"
	)
	((previous_resets == 1)) ||
		fail "shutdown dump recorded $previous_resets successful DSP resets, expected 1"
	power_failures=$(
		marker_count 'failed to power up AE-5 at shutdown' "$shutdown_log"
	)
	reset_failures=$(
		marker_count 'failed to reset AE-5 DSP at shutdown' "$shutdown_log"
	)
	((power_failures == 0 && reset_failures == 0)) ||
		fail 'shutdown dump recorded an AE-5 DSP-reset failure'

	current_downloads=$(
		marker_count 'ca0132 DSP downloaded and running' "$current_log"
	)
	((current_downloads == 1)) ||
		fail "current boot recorded $current_downloads CA0132 DSP downloads, expected 1"
	read -r taint < "$taint_file"
	[[ $taint == 0 ]] || fail "current kernel taint is $taint, expected 0"

	printf 'previous_kernel=%s\n' "$previous_release"
	printf 'previous_dsp_downloads=%s\n' "$previous_downloads"
	printf 'shutdown_evidence=efi-pstore\n'
	printf 'previous_shutdown_resets=%s\n' "$previous_resets"
	printf 'previous_shutdown_failures=0\n'
	printf 'current_kernel=%s\n' "$current_release"
	printf 'current_dsp_downloads=%s\n' "$current_downloads"
	printf 'current_kernel_taint=%s\n' "$taint"
	printf 'linux_handoff_evidence=pass\n'
}

self_test() (
	local root previous_log shutdown_log current_log taint_file result
	local previous_release=7.1.4-ae5-shutdown
	local current_release=7.1.4-ae5-stable
	local efivars disable_file always_dump_file backend_file

	root=$(mktemp -d "${TMPDIR:-/tmp}/ae5-warm-handoff-test.XXXXXX")
	trap 'find "$root" -depth -delete' EXIT
	previous_log=$root/previous.log
	shutdown_log=$root/shutdown.log
	current_log=$root/current.log
	taint_file=$root/tainted
	efivars=$root/efivars
	disable_file=$root/pstore_disable
	always_dump_file=$root/always_kmsg_dump
	backend_file=$root/backend
	mkdir "$efivars"

	printf 'Linux version %s (test)\n' "$previous_release" > "$previous_log"
	printf 'ca0132 DSP downloaded and running\n' >> "$previous_log"
	printf 'Shutdown#1 Part1\n' > "$shutdown_log"
	printf 'AE-5 DSP reset at shutdown\n' >> "$shutdown_log"
	printf 'Linux version %s (test)\n' "$current_release" > "$current_log"
	printf 'ca0132 DSP downloaded and running\n' >> "$current_log"
	printf '0\n' > "$taint_file"
	printf 'N\n' > "$disable_file"
	printf 'Y\n' > "$always_dump_file"
	printf 'efi_pstore\n' > "$backend_file"

	result=$(
		check_preparation "$previous_release" "$previous_release" \
			"$taint_file" "$disable_file" "$always_dump_file" \
			"$backend_file" "$efivars"
	)
	grep -Fq 'shutdown_evidence_preparation=pass' <<< "$result" ||
		fail 'self-test did not accept valid pstore preparation'
	result=$(
		check_handoff "$previous_release" "$current_release" \
			"$previous_log" "$shutdown_log" "$current_log" "$taint_file"
	)
	grep -Fq 'linux_handoff_evidence=pass' <<< "$result" ||
		fail 'self-test did not accept valid handoff evidence'

	python3 - "$efivars" "$pstore_guid" "$shutdown_log" <<'PY'
import pathlib
import sys
import zlib

root, guid, source = pathlib.Path(sys.argv[1]), sys.argv[2], pathlib.Path(sys.argv[3])
compressor = zlib.compressobj(wbits=-zlib.MAX_WBITS)
payload = compressor.compress(source.read_bytes()) + compressor.flush()
path = root / f"dump-type0-1-1-1780000000-C-{guid}"
path.write_bytes((7).to_bytes(4, "little") + payload)
PY
	[[ $(decode_pstore "$efivars") == "$(<"$shutdown_log")" ]] ||
		fail 'self-test did not decode compressed EFI pstore evidence'
	if (
		check_preparation "$previous_release" "$previous_release" \
			"$taint_file" "$disable_file" "$always_dump_file" \
			"$backend_file" "$efivars"
	) >/dev/null 2>&1; then
		fail 'self-test accepted stale EFI pstore records'
	fi

	printf 'Linux version %s (test)\n' "$previous_release" > "$previous_log"
	if (
		check_handoff "$previous_release" "$current_release" \
			"$previous_log" "$shutdown_log" "$current_log" "$taint_file"
	) >/dev/null 2>&1; then
		fail 'self-test accepted a missing previous DSP initialization'
	fi
	printf 'ca0132 DSP downloaded and running\n' >> "$previous_log"
	printf 'failed to reset AE-5 DSP at shutdown: -5\n' >> "$shutdown_log"
	if (
		check_handoff "$previous_release" "$current_release" \
			"$previous_log" "$shutdown_log" "$current_log" "$taint_file"
	) >/dev/null 2>&1; then
		fail 'self-test accepted a shutdown-reset failure'
	fi

	printf 'AE-5 warm-handoff self-test passed\n'
)

[[ $# -ge 1 ]] || usage
case $1 in
--self-test)
	[[ $# -eq 1 ]] || usage
	self_test
	exit
	;;
--prepare | --check)
	mode=${1#--}
	shift
	;;
-h | --help)
	usage
	;;
*)
	usage
	;;
esac
[[ $# -eq 1 ]] || usage

for tool in awk chmod cp date find grep journalctl mkdir mktemp python3 tee \
	uname wc; do
	command -v "$tool" >/dev/null 2>&1 ||
		fail "required tool is unavailable: $tool"
done

expected_release=$1
current_release=$(uname -r)
taint_file=/proc/sys/kernel/tainted
efivar_root=${AE5_EFIVAR_ROOT:-/sys/firmware/efi/efivars}

if [[ $mode == prepare ]]; then
	check_preparation "$expected_release" "$current_release" "$taint_file" \
		/sys/module/efi_pstore/parameters/pstore_disable \
		/sys/module/printk/parameters/always_kmsg_dump \
		/sys/module/pstore/parameters/backend "$efivar_root"
	exit
fi

[[ ${AE5_WARM_HANDOFF_CONFIRMED:-0} == 1 ]] ||
	fail 'set AE5_WARM_HANDOFF_CONFIRMED=1 only after a handoff without power removal'

cache_root=${XDG_CACHE_HOME:-"$HOME/.cache"}
cache_directory=$cache_root/ae5-control
mkdir -p -- "$cache_directory"
evidence_root=$(mktemp -d \
	"$cache_directory/warm-handoff-$(date +%Y%m%d-%H%M%S).XXXXXX")
chmod 0700 "$evidence_root"
previous_log=$evidence_root/previous-kernel.log
shutdown_log=$evidence_root/shutdown-pstore.log
current_log=$evidence_root/current-kernel.log
result_file=$evidence_root/result.txt

journalctl -k -b -1 --no-pager -o cat > "$previous_log" ||
	fail 'unable to read the previous boot kernel journal'
journalctl -k -b 0 --no-pager -o cat > "$current_log" ||
	fail 'unable to read the current boot kernel journal'
decode_pstore "$efivar_root" > "$shutdown_log"
while IFS= read -r file; do
	cp -- "$file" "$evidence_root/${file##*/}"
done < <(
	find "$efivar_root" -maxdepth 1 -type f \
		-name "dump-type0-*-$pstore_guid" -print
)

printf 'evidence=%s\n' "$evidence_root"
{
	printf 'operator_warm_handoff_confirmed=yes\n'
	printf 'pstore_record_parts=%s\n' "$(pstore_file_count "$efivar_root")"
	check_handoff "$expected_release" "$current_release" \
		"$previous_log" "$shutdown_log" "$current_log" "$taint_file"
	printf 'warm_handoff_result=pass\n'
} | tee "$result_file"
