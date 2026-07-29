#!/usr/bin/env bash
# Recover the AE-5 from the CA0132 idle DSP self-oscillation without a
# reboot, by unbinding and rebinding the card's PCI device so the driver
# re-downloads the DSP image.
#
# Background: with global OutFX enabled the DSP can latch into a state
# where it emits a continuous ~61-65 Hz harmonic stack with no stream
# playing. The fault survives an effect-parameter reset, a PipeWire
# restart, and an OutFX toggle. Only a DSP re-download clears it; on a
# fresh DSP the same configuration measures exact digital silence.
#
# Scope is the audited card only: PCI 1102:0012, subsystem 1102:0051.
# Other sound cards are never touched.
#
# Requires root for the PCI unbind/rebind. Output stays hard-muted for
# the whole operation so a transient cannot reach the headphones.
set -euo pipefail

PCI_ID="1102:0012"
SUBSYSTEM="1102:0051"
DRIVER="snd_hda_intel"
AE5CTL="${AE5CTL:-ae5ctl}"
PROFILE="${AE5_REINIT_PROFILE:-}"
ASSUME_YES="${AE5_REINIT_YES:-0}"

die() { printf 'error: %s\n' "$*" >&2; exit 1; }
note() { printf '%s\n' "$*"; }

find_slot() {
    local slot
    while read -r slot _; do
        [ -n "$slot" ] || continue
        local sub
        sub="$(lspci -s "$slot" -vmm 2>/dev/null \
            | awk -F'\t' '/^SVendor:|^SDevice:/{print $2}' | tr -d ' ')"
        printf '0000:%s\n' "$slot"
        return 0
    done < <(lspci -d "$PCI_ID" 2>/dev/null)
    return 1
}

verify_target() {
    # Refuse to act unless the subsystem matches the audited AE-5.
    local slot="$1" ids
    ids="$(lspci -n -s "${slot#0000:}" 2>/dev/null | awk '{print $3}')"
    [ "$ids" = "$PCI_ID" ] \
        || die "PCI $slot reports $ids, expected $PCI_ID"
    local sub
    sub="$(lspci -n -v -s "${slot#0000:}" 2>/dev/null \
        | awk '/Subsystem:/{print $NF}')"
    if [ -n "$sub" ] && [ "$sub" != "$SUBSYSTEM" ]; then
        die "PCI $slot subsystem $sub, expected $SUBSYSTEM"
    fi
}

wuh_rms() {
    local dev tmp
    dev="$(arecord -l 2>/dev/null | awk '
        tolower($0) ~ /what u hear/ {
            if (match($0, /^card ([0-9]+).*device ([0-9]+)/, m))
                { print m[1] "," m[2]; exit } }')"
    [ -n "${dev:-}" ] || { echo "nan"; return; }
    tmp="$(mktemp -t ae5-reinit-XXXXXX.wav)"
    if arecord -D "hw:${dev}" -f S32_LE -c 2 -r 48000 -d 2 "$tmp" >/dev/null 2>&1; then
        sox "$tmp" -n stats 2>&1 | awk '/RMS lev dB/{print $4; f=1} END{if(!f) print "nan"}'
    else
        echo "nan"
    fi
    rm -f "$tmp"
}

hard_mute() {
    "$AE5CTL" set-playback-switch Master off >/dev/null 2>&1 || true
    "$AE5CTL" set-playback-switch Front off  >/dev/null 2>&1 || true
    command -v wpctl >/dev/null 2>&1 && wpctl set-mute @DEFAULT_AUDIO_SINK@ 1 || true
}

find_card() {
    local card path
    for path in /sys/class/sound/card[0-9]*; do
        [ -r "$path/device/vendor" ] || continue
        [ "$(cat "$path/device/vendor")" = "0x1102" ] || continue
        [ "$(cat "$path/device/device")" = "0x0012" ] || continue
        [ "$(cat "$path/device/subsystem_vendor")" = "0x1102" ] || continue
        [ "$(cat "$path/device/subsystem_device")" = "0x0051" ] || continue
        card="${path##*/card}"
        printf '%s\n' "$card"
        return 0
    done
    return 1
}

authorize_rebind() {
    ELEVATE=()
    [ "$(id -u)" -eq 0 ] && return

    if command -v pkexec >/dev/null 2>&1; then
        note "requesting desktop authorization for the later PCI rebind"
        pkexec /usr/bin/true ||
            die "desktop authorization for the PCI rebind failed"
        ELEVATE=(pkexec)
        return
    fi

    command -v sudo >/dev/null 2>&1 ||
        die "root, pkexec, or sudo is required for the PCI rebind"
    sudo -v || die "sudo authorization for the PCI rebind failed"
    ELEVATE=(sudo)
}

main() {
    command -v lspci >/dev/null 2>&1 || die "lspci is required"

    local slot
    slot="$(find_slot)" || die "no PCI device matching $PCI_ID found"
    verify_target "$slot"
    note "target: $slot ($PCI_ID / $SUBSYSTEM)"

    local before
    before="$(wuh_rms)"
    note "before: What U Hear RMS ${before} dBFS"

    if [ "$ASSUME_YES" != "1" ]; then
        note ""
        note "This will briefly remove and re-add the AE-5, stopping the local"
        note "PipeWire session while it happens. Headphones should be OFF your"
        note "head. Set AE5_REINIT_YES=1 to skip this prompt."
        printf 'continue? [y/N] '
        read -r reply
        case "$reply" in y|Y|yes|YES) ;; *) note "aborted"; exit 1 ;; esac
    fi

    local -a ELEVATE
    authorize_rebind

    note "hard-muting output"
    hard_mute
    sleep 1

    note "stopping PipeWire session"
    systemctl --user stop pipewire.socket pipewire-pulse.socket \
        wireplumber.service pipewire.service pipewire-pulse.service 2>/dev/null || true
    sleep 2

    note "unbinding $slot"
    "${ELEVATE[@]}" sh -c "echo $slot > /sys/bus/pci/drivers/$DRIVER/unbind" \
        || die "unbind failed"
    sleep 3
    note "rebinding $slot"
    "${ELEVATE[@]}" sh -c "echo $slot > /sys/bus/pci/drivers/$DRIVER/bind" \
        || die "rebind failed — a reboot will restore the card"
    # The card returns with Master and Front ON at their 0 dB points. Re-mute
    # immediately: everything below (log check, PipeWire restart, profile
    # reapply) would otherwise run with a live analog output, which is
    # audible to anyone wearing the headphones.
    for _ in 1 2 3 4 5 6 7 8 9 10; do
        find_card >/dev/null && break
        sleep 0.5
    done
    find_card >/dev/null || die "AE-5 did not return after rebind"
    hard_mute
    sleep 3

    if journalctl -k -n 60 --no-pager 2>/dev/null \
            | grep -q 'ca0132 DSP downloaded and running'; then
        note "DSP re-download confirmed in kernel log"
    else
        note "warning: no fresh 'DSP downloaded' line found; check dmesg"
    fi

    note "restarting PipeWire session"
    systemctl --user start pipewire.socket pipewire-pulse.socket 2>/dev/null || true
    systemctl --user start wireplumber.service pipewire.service \
        pipewire-pulse.service 2>/dev/null || true
    sleep 6

    hard_mute
    if [ -n "$PROFILE" ]; then
        note "reapplying profile $PROFILE"
        "$AE5CTL" profile-apply "$PROFILE" 2>&1 | tail -1 || true
    fi
    "$AE5CTL" route-repair >/dev/null 2>&1 || true
    command -v wpctl >/dev/null 2>&1 && {
        wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%
        wpctl set-mute @DEFAULT_AUDIO_SINK@ 1
    } || true

    sleep 2
    local after
    after="$(wuh_rms)"
    note "after : What U Hear RMS ${after} dBFS"
    note ""
    "$AE5CTL" route-status 2>&1 | grep -E 'health' || true
    note "output left hard-muted at 5%; unmute when ready"
}

case "${1:-}" in
    -h|--help)
        cat <<'EOF'
usage: dsp-reinit.sh

Clears the CA0132 idle DSP self-oscillation by rebinding the AE-5 PCI
device so the driver re-downloads the DSP. Scoped to PCI 1102:0012 /
subsystem 1102:0051; refuses to touch anything else. Requests desktop
authorization (or sudo as a fallback) before it stops PipeWire.

Environment:
  AE5_REINIT_PROFILE  profile JSON to reapply afterwards
  AE5_REINIT_YES=1    skip the confirmation prompt
  AE5CTL              path to ae5ctl when not on PATH
EOF
        ;;
    --self-test)
        command -v lspci >/dev/null 2>&1 || die "lspci is required"
        command -v arecord >/dev/null 2>&1 || die "arecord is required"
        s="$(find_slot)" || die "AE-5 not present"
        verify_target "$s"
        echo "dsp-reinit self-test passed (target $s)"
        ;;
    *) main ;;
esac
