import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "../components"

PageScaffold {
    id: root

    pageTitle: qsTr("Device")
    pageDescription: qsTr("Card identity, driver readiness, firmware symptoms and guarded recovery actions in one user-facing view.")
    onRetryRequested: root.appState.refreshFromDaemon()

    SectionPanel {
        Layout.fillWidth: true
        title: root.appState.deviceName
        detail: root.appState.statusDetail
        statusText: root.appState.deviceStatus
        statusKind: root.appState.statusCode

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("ALSA card")
            detail: qsTr("Exact Creative 1102:0012 / 1102:0051 matching")
            value: root.appState.cardIndex >= 0
                   ? qsTr("Card %1").arg(root.appState.cardIndex)
                   : qsTr("Unavailable")
            statusText: root.appState.connected ? qsTr("Matched") : qsTr("Not detected")
            statusKind: root.appState.connected ? "ready" : "unavailable"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("ALSA controls")
            detail: qsTr("Mixer controls exposed for this card revision")
            value: root.appState.controlsCount > 0
                   ? qsTr("%1 controls").arg(root.appState.controlsCount)
                   : qsTr("Unavailable")
            statusText: root.appState.controlsCount > 0 ? qsTr("Available") : qsTr("Unknown")
            statusKind: root.appState.controlsCount > 0 ? "ready" : "partial"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("User daemon")
            detail: qsTr("Typed D-Bus state, profile persistence and checked writes")
            value: "ae5d"
            statusText: root.appState.daemonAvailable ? qsTr("Running") : qsTr("Unavailable")
            statusKind: root.appState.daemonAvailable ? "ready" : "unavailable"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Current format")
            detail: qsTr("Live PipeWire/ALSA playback state")
            value: root.appState.audioFormat
            statusText: root.appState.audioFormatAvailable ? qsTr("Reported") : qsTr("Unavailable")
            statusKind: root.appState.audioFormatAvailable ? "ready" : "partial"
            showSeparator: false
        }
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Driver capability boundaries")
        detail: qsTr("Safety blocks are product behavior, not hidden errors.")
        statusText: qsTr("Fail closed")
        statusKind: "ready"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Hardware OutFX")
            detail: qsTr("Production builds reject this transition because repeated tests corrupted the DSP until reinitialization or cold boot.")
            value: qsTr("Blocked")
            statusText: qsTr("Safety")
            statusKind: "unavailable"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Direct Mode")
            detail: root.appState.directModeAvailable
                    ? root.appState.hardwareWriteBlockReason
                    : qsTr("Not exposed by the current production driver.")
            value: root.appState.directModeAvailable ? qsTr("Detected") : qsTr("Unavailable")
            statusText: root.appState.directModeAvailable ? qsTr("Guarded") : qsTr("Driver limit")
            statusKind: "partial"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Firmware")
            detail: root.appState.statusCode === "firmware-missing"
                    ? root.appState.statusDetail
                    : qsTr("No missing-firmware condition is reported by the current device state.")
            value: root.appState.statusCode === "firmware-missing"
                   ? qsTr("Missing") : qsTr("No reported fault")
            statusText: root.appState.statusCode === "firmware-missing"
                        ? qsTr("Action required") : qsTr("Quiet")
            statusKind: root.appState.statusCode === "firmware-missing"
                        ? "unavailable" : "ready"
            showSeparator: false
        }
    }

    RowLayout {
        Layout.fillWidth: true

        Label {
            Layout.fillWidth: true
            text: qsTr("Refreshing reads state only; it does not restart or reconfigure the card.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontCaption
        }

        AppButton {
            text: qsTr("Refresh device")
            onClicked: root.appState.refreshFromDaemon()
        }
    }
}
