#!/usr/bin/env bash
set -euo pipefail

readonly startup_budget_ms=1000
readonly refresh_budget_ms=100
readonly idle_cpu_budget_percent=1
readonly rss_budget_kib=$((100 * 1024))
readonly idle_samples=50
readonly idle_interval_seconds=0.1

binary=${1:-target/release/ae5-control}
[[ -x $binary ]] || {
	printf 'error: executable not found: %s\n' "$binary" >&2
	printf 'build it with: cargo build --locked --release --all-features\n' >&2
	exit 1
}

output=$(mktemp "${TMPDIR:-/tmp}/ae5-gui-performance.XXXXXX")
pid=
cleanup() {
	if [[ -n $pid ]] && kill -0 "$pid" 2>/dev/null; then
		kill "$pid" 2>/dev/null || true
		wait "$pid" 2>/dev/null || true
	fi
	rm -f -- "$output"
}
trap cleanup EXIT

AE5_CONTROL_PERFORMANCE_PROBE=1 "$binary" >"$output" 2>&1 &
pid=$!

for ((attempt = 0; attempt < 500; attempt++)); do
	if grep -q '^probe_ready=1$' "$output"; then
		break
	fi
	if ! kill -0 "$pid" 2>/dev/null; then
		printf 'error: GUI exited before the performance probe was ready\n' >&2
		sed -n '1,160p' "$output" >&2
		exit 1
	fi
	sleep 0.01
done

grep -q '^probe_ready=1$' "$output" || {
	printf 'error: GUI performance probe did not become ready within 5 seconds\n' >&2
	sed -n '1,160p' "$output" >&2
	exit 1
}

startup_ms=$(awk -F = '$1 == "startup_ms" { print $2 }' "$output")
refresh_ms=$(awk -F = '$1 == "control_refresh_ms" { print $2 }' "$output")
[[ $startup_ms =~ ^[0-9]+$ && $refresh_ms =~ ^[0-9]+$ ]] || {
	printf 'error: invalid performance probe output\n' >&2
	sed -n '1,160p' "$output" >&2
	exit 1
}

read_ticks() {
	awk '{ print $14 + $15 }' "/proc/$pid/stat"
}

start_ticks=$(read_ticks)
max_rss_kib=0
for ((sample = 0; sample < idle_samples; sample++)); do
	kill -0 "$pid" 2>/dev/null || {
		printf 'error: GUI exited during the idle sample\n' >&2
		exit 1
	}
	rss_kib=$(awk '$1 == "VmRSS:" { print $2 }' "/proc/$pid/status")
	((rss_kib > max_rss_kib)) && max_rss_kib=$rss_kib
	sleep "$idle_interval_seconds"
done
end_ticks=$(read_ticks)

clock_ticks=$(getconf CLK_TCK)
idle_seconds=$(awk -v samples="$idle_samples" -v interval="$idle_interval_seconds" \
	'BEGIN { print samples * interval }')
idle_cpu_percent=$(awk \
	-v ticks="$((end_ticks - start_ticks))" \
	-v clock_ticks="$clock_ticks" \
	-v seconds="$idle_seconds" \
	'BEGIN { printf "%.2f", (ticks / clock_ticks) * 100 / seconds }')

printf 'startup_ms=%s\n' "$startup_ms"
printf 'control_refresh_ms=%s\n' "$refresh_ms"
printf 'idle_cpu_percent=%s\n' "$idle_cpu_percent"
printf 'max_idle_rss_kib=%s\n' "$max_rss_kib"

result=0
if ((startup_ms < startup_budget_ms)); then
	printf 'startup_result=pass\n'
else
	printf 'startup_result=fail\n'
	result=1
fi
if ((refresh_ms <= refresh_budget_ms)); then
	printf 'control_refresh_result=pass\n'
else
	printf 'control_refresh_result=fail\n'
	result=1
fi
if awk -v actual="$idle_cpu_percent" -v budget="$idle_cpu_budget_percent" \
	'BEGIN { exit !(actual <= budget) }'; then
	printf 'idle_cpu_result=pass\n'
else
	printf 'idle_cpu_result=fail\n'
	result=1
fi
if ((max_rss_kib < rss_budget_kib)); then
	printf 'rss_result=pass\n'
else
	printf 'rss_result=fail\n'
	result=1
fi

exit "$result"
