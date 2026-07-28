#!/usr/bin/env bash
set -euo pipefail

script_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
matrix=$(dirname -- "$script_root")/feature-parity.tsv

awk -F '\t' '
NR == 1 {
	expected = "area\tfeature\tstatus\tlinux_mechanism\tcurrent_evidence\tremaining_gate\tsource"
	if ($0 != expected) {
		print "invalid feature-parity.tsv header" > "/dev/stderr"
		failed = 1
	}
	next
}
{
	if (NF != 7) {
		print "feature-parity.tsv line " NR " has " NF " fields; expected 7" > "/dev/stderr"
		failed = 1
	}
	if ($3 != "verified" && $3 != "intentionally substituted" &&
		$3 != "deferred" && $3 != "unsupported") {
		print "feature-parity.tsv line " NR " has invalid status: " $3 > "/dev/stderr"
		failed = 1
	}
	key = $1 SUBSEP $2
	if (seen[key]++) {
		print "feature-parity.tsv line " NR " duplicates a feature" > "/dev/stderr"
		failed = 1
	}
}
END {
	if (NR < 2) {
		print "feature-parity.tsv has no feature rows" > "/dev/stderr"
		failed = 1
	}
	if (failed)
		exit 1
	print "feature parity matrix: " NR - 1 " rows validated"
}
' "$matrix"
