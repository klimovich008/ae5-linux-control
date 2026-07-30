#!/usr/bin/env bash
set -euo pipefail

binary=${1:-target/release/ae5-control-qml}

if [[ ! -x "$binary" ]]; then
    echo "Qt/QML binary is not executable: $binary" >&2
    exit 2
fi

run_qml_check() {
    QT_QPA_PLATFORM=offscreen \
    QT_QUICK_BACKEND=software \
    QT_ACCESSIBILITY=1 \
    timeout 20s "$binary" "$@" 2>&1
}

if ! focus_output=$(run_qml_check --qa-state=ready --qa-focus-audit); then
    printf '%s\n' "$focus_output" >&2
    echo "QML focus-order audit did not pass." >&2
    exit 1
fi

if ! modified_focus_output=$(run_qml_check --qa-state=both-modified --qa-focus-audit); then
    printf '%s\n' "$modified_focus_output" >&2
    echo "Modified-state QML focus-order audit did not pass." >&2
    exit 1
fi

if ! tray_output=$(
    run_qml_check --qa-state=ready --start-hidden --qa-tray-smoke
); then
    printf '%s\n' "$tray_output" >&2
    echo "QML start-hidden and tray lifecycle smoke did not pass." >&2
    exit 1
fi

if ! tray_unsaved_output=$(
    run_qml_check --qa-state=both-modified --start-hidden \
        --qa-tray-unsaved-smoke
); then
    printf '%s\n' "$tray_unsaved_output" >&2
    echo "QML hidden unsaved Quit lifecycle smoke did not pass." >&2
    exit 1
fi

states=(
    ready
    no-device
    partial
    firmware-missing
    permission-denied
    device-busy
    write-failed
    daemon-unavailable
    direct-mode
    both-modified
)

pages=(
    overview
    sound
    equalizer
    playback
    recording
    mixer
    lighting
    device
    settings
)

for state in "${states[@]}"; do
    if ! state_output=$(run_qml_check "--qa-state=$state" --qa-state-smoke); then
        printf '%s\n' "$state_output" >&2
        echo "QML state smoke failed for $state." >&2
        exit 1
    fi
done

for page in "${pages[@]}"; do
    if ! page_output=$(run_qml_check --qa-state=ready "--qa-page=$page" --qa-state-smoke); then
        printf '%s\n' "$page_output" >&2
        echo "QML page smoke failed for $page." >&2
        exit 1
    fi
done

echo "QML focus-order audit passed."
echo "QML start-hidden and tray lifecycle smoke passed."
echo "QML hidden unsaved Quit lifecycle smoke passed."
echo "QML accessibility and state smoke passed for ${#states[@]} scenarios."
echo "QML page smoke passed for ${#pages[@]} destinations."
