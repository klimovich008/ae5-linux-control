import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "../components"

PageScaffold {
    id: root

    signal navigateRequested(string page)

    pageTitle: qsTr("Overview")
    pageDescription: qsTr("A quiet summary of the active AE-5 path, saved sound objects and driver readiness.")
    onRetryRequested: root.appState.refreshFromDaemon()

    GridLayout {
        Layout.fillWidth: true
        columns: root.width >= 840 ? 4 : 2
        columnSpacing: Theme.space3
        rowSpacing: Theme.space3

        MetricCard {
            Layout.fillWidth: true
            title: qsTr("Device")
            value: root.appState.connected ? qsTr("Sound BlasterX AE-5")
                                           : root.appState.deviceName
            detail: root.appState.audioFormatAvailable
                    ? root.appState.audioFormat
                    : root.appState.statusDetail
            stateText: root.appState.deviceStatus
            stateKind: root.appState.statusCode
            actionText: qsTr("Open Device")
            onActivated: root.navigateRequested("device")
        }

        MetricCard {
            Layout.fillWidth: true
            title: qsTr("Active output")
            value: root.appState.outputAvailable ? root.appState.output : qsTr("Unavailable")
            detail: root.appState.output === "Headphones"
                    ? qsTr("%1 gain").arg(root.appState.headphoneGain)
                    : qsTr("Output switching stays in the hardware footer.")
            stateText: root.appState.outputAvailable ? qsTr("Ready") : qsTr("Unavailable")
            stateKind: root.appState.outputAvailable ? "ready" : "unavailable"
            actionText: qsTr("Open Playback")
            onActivated: root.navigateRequested("playback")
        }

        MetricCard {
            Layout.fillWidth: true
            title: qsTr("Sound objects")
            value: root.appState.effectsProfile
            detail: qsTr("EQ preset: %1").arg(root.appState.eqPreset)
            stateText: root.appState.unsavedCount > 0
                       ? qsTr("%1 modified").arg(root.appState.unsavedCount)
                       : qsTr("Saved")
            stateKind: root.appState.unsavedCount > 0 ? "modified" : "ready"
            actionText: qsTr("Open Sound")
            onActivated: root.navigateRequested("sound")
        }

        MetricCard {
            Layout.fillWidth: true
            title: qsTr("Live equalizer")
            value: root.appState.softwareEqActive ? qsTr("Active") : qsTr("Inactive")
            detail: root.appState.softwareEqDetail
            stateText: root.appState.softwareEqState === "error"
                       ? qsTr("Error")
                       : root.appState.softwareEqActive ? qsTr("Live") : qsTr("Quiet")
            stateKind: root.appState.softwareEqState
            actionText: qsTr("Open Equalizer")
            onActivated: root.navigateRequested("equalizer")
        }
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("System readiness")
        detail: qsTr("These checks describe the control path. They do not claim that every physical connector has completed acceptance testing.")
        statusText: root.appState.statusCode === "ready" ? qsTr("Operational")
                                                          : root.appState.deviceStatus
        statusKind: root.appState.statusCode

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("AE-5 discovery")
            detail: qsTr("Exact Creative PCI and ALSA identity")
            value: root.appState.cardIndex >= 0
                   ? qsTr("ALSA card %1").arg(root.appState.cardIndex)
                   : qsTr("Not detected")
            statusText: root.appState.connected ? qsTr("Connected") : qsTr("Unavailable")
            statusKind: root.appState.connected ? "ready" : "unavailable"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("User service")
            detail: qsTr("Typed session D-Bus state and checked writes")
            value: "ae5d"
            statusText: root.appState.daemonAvailable ? qsTr("Available") : qsTr("Offline")
            statusKind: root.appState.daemonAvailable ? "ready" : "unavailable"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("ALSA controls")
            detail: qsTr("Controls discovered for the exact card revision")
            value: root.appState.controlsCount > 0
                   ? qsTr("%1 controls").arg(root.appState.controlsCount)
                   : qsTr("Unavailable")
            statusText: root.appState.controlsCount > 0 ? qsTr("Detected") : qsTr("Unknown")
            statusKind: root.appState.controlsCount > 0 ? "ready" : "partial"
            showSeparator: false
        }
    }
}
