#!/usr/bin/env bash
# Runtime debug visibility for the CA0132 driver, no rebuild required.
#
# The kernel ships 121 dynamic-debug call sites in snd_hda_codec_ca0132 —
# every dspio command, DSP transfer and setup step can narrate itself into the
# kernel log. They are off by default, which is why the DSP faults chased in
# this project left no kernel evidence: the driver had plenty to say and
# nobody had switched it on.
#
# Requires root for /sys/kernel/debug; everything here is read-mostly and
# touches no audio state.
set -euo pipefail

CONTROL=/sys/kernel/debug/dynamic_debug/control
MODULE=snd_hda_codec_ca0132
EVIDENCE_ROOT="${AE5_EVIDENCE_ROOT:-$HOME/.local/state/ae5-incidents}"

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
note() { printf '%s\n' "$*"; }

as_root() {
    if [ "$(id -u)" -eq 0 ]; then "$@"; else sudo "$@"; fi
}

require_control() {
    as_root test -f "$CONTROL" \
        || die "dynamic debug unavailable (CONFIG_DYNAMIC_DEBUG or debugfs missing)"
}

enabled_count() {
    as_root grep -c "\[$MODULE\].* =p" "$CONTROL" 2>/dev/null || echo 0
}

snapshot() {
    local dir
    dir="$EVIDENCE_ROOT/$(date +%Y%m%d-%H%M%S)"
    mkdir -p "$dir"
    note "collecting into $dir"

    journalctl -k -b --no-pager 2>/dev/null | tail -300 > "$dir/kernel-tail.log" || true
    amixer -c 0 contents > "$dir/mixer-contents.txt" 2>&1 || true
    cat /proc/asound/card0/codec#* > "$dir/codec-dump.txt" 2>/dev/null || true
    for f in /proc/asound/card0/pcm*p/sub*/status /proc/asound/card0/pcm*p/sub*/hw_params; do
        [ -f "$f" ] && printf '== %s\n%s\n' "$f" "$(cat "$f")" >> "$dir/pcm-state.txt"
    done
    command -v pw-dump >/dev/null 2>&1 && pw-dump > "$dir/pw-dump.json" 2>/dev/null || true
    command -v wpctl >/dev/null 2>&1 && wpctl status > "$dir/wpctl-status.txt" 2>&1 || true
    printf 'kernel %s taint %s\ndyndbg sites enabled: %s\n' \
        "$(uname -r)" "$(cat /proc/sys/kernel/tainted)" "$(enabled_count)" \
        > "$dir/system.txt"

    note "snapshot complete:"
    ls -l "$dir"
}

case "${1:-}" in
    on)
        require_control
        as_root sh -c "echo 'module $MODULE +p' > $CONTROL"
        note "ca0132 debug ON ($(enabled_count) sites); watch with: journalctl -kf | grep -i ca0132"
        ;;
    off)
        require_control
        as_root sh -c "echo 'module $MODULE -p' > $CONTROL"
        note "ca0132 debug OFF"
        ;;
    status)
        require_control
        note "enabled sites: $(enabled_count) of $(as_root grep -c "\[$MODULE\]" "$CONTROL")"
        ;;
    watch)
        note "streaming kernel log for ca0132/hda (Ctrl-C to stop)"
        journalctl -kf --no-pager | grep --line-buffered -iE 'ca0132|hda'
        ;;
    snapshot)
        snapshot
        ;;
    --self-test)
        command -v journalctl >/dev/null 2>&1 || die "journalctl is required"
        command -v amixer >/dev/null 2>&1 || die "amixer is required"
        [ -d "$(dirname "$EVIDENCE_ROOT")" ] || die "no parent for evidence root"
        echo "ca0132-debug self-test passed"
        ;;
    *)
        cat <<'EOF'
usage: ca0132-debug.sh on|off|status|watch|snapshot

  on        enable the driver's dynamic-debug messages (dspio commands,
            DSP transfers, chip IO) in the kernel log
  off       disable them again
  status    how many call sites are currently enabled
  watch     follow the kernel log filtered to ca0132/hda
  snapshot  collect kernel tail, full mixer readback, codec dump, PCM and
            PipeWire state into ~/.local/state/ae5-incidents/<timestamp>/

Turn debug ON before reproducing a DSP fault, reproduce, then `snapshot`.
EOF
        ;;
esac
