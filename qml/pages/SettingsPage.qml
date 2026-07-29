import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "../components"

PageScaffold {
    id: root

    pageTitle: qsTr("Settings")
    pageDescription: qsTr("Application appearance, background restoration and guarded maintenance behavior.")
    onRetryRequested: root.appState.refreshFromDaemon()

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Appearance")
        detail: qsTr("Theme changes apply to this session immediately. Persistent preference storage is a later settings slice.")
        statusText: Theme.light ? qsTr("Light") : qsTr("Dark")
        statusKind: "ready"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Color theme")
            detail: qsTr("The same semantic colors and contrast rules are used in both themes.")
            value: Theme.light ? qsTr("Light") : qsTr("Dark")
            statusText: qsTr("Session")
            statusKind: "ready"
            showSeparator: false

            AppButton {
                objectName: "theme-dark"
                text: qsTr("Dark")
                checkable: true
                checked: !Theme.light
                onClicked: Theme.light = false
            }

            AppButton {
                objectName: "theme-light"
                text: qsTr("Light")
                checkable: true
                checked: Theme.light
                onClicked: Theme.light = true
            }
        }
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Background behavior")
        detail: qsTr("Audio state restoration must remain explicit, bounded and independent from opening the GUI.")
        statusText: root.appState.daemonAvailable ? qsTr("Service available") : qsTr("Offline")
        statusKind: root.appState.daemonAvailable ? "ready" : "unavailable"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Device state service")
            detail: qsTr("The ae5d user service supplies state and checked operations over session D-Bus.")
            value: "ae5d"
            statusText: root.appState.daemonAvailable ? qsTr("Available") : qsTr("Unavailable")
            statusKind: root.appState.daemonAvailable ? "ready" : "unavailable"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Lighting restore")
            detail: qsTr("A packaged one-shot user action restores saved onboard LED colors at login.")
            value: qsTr("Rootless")
            statusText: qsTr("Supported")
            statusKind: "ready"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Automatic audio routing")
            detail: qsTr("Not enabled by default; ALSA controls persist and unnecessary route rewrites can interrupt audio.")
            value: qsTr("Off")
            statusText: qsTr("Intentional")
            statusKind: "ready"
            showSeparator: false
        }
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Maintenance")
        detail: qsTr("System packages and linux-firmware own updates. The application never asks to run as root.")
        statusText: qsTr("Distribution managed")
        statusKind: "ready"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Application updates")
            detail: qsTr("Use the installed RPM or reversible user installer.")
            value: qsTr("External")
            statusText: qsTr("Supported")
            statusKind: "ready"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Reset device settings")
            detail: qsTr("The guarded Linux-driver baseline requires a restorable backup and verified rollback before writes.")
            value: qsTr("Unavailable here")
            statusText: qsTr("Guarded")
            statusKind: "partial"

            AppButton {
                text: qsTr("Reset")
                variant: "danger"
                enabled: false
                blockedReason: qsTr("Factory reset is not connected to the typed QML service yet.")
            }
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Refresh application state")
            detail: qsTr("Reconnect to ae5d and read the latest confirmed values.")
            statusText: qsTr("Safe")
            statusKind: "ready"
            showSeparator: false

            AppButton {
                text: qsTr("Refresh")
                onClicked: root.appState.refreshFromDaemon()
            }
        }
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("About")
        detail: qsTr("AE5 Control is an open Linux control center for the Creative Sound BlasterX AE-5.")
        statusText: qsTr("Development build")
        statusKind: "ready"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Architecture")
            detail: qsTr("Qt 6/QML interface · Rust state and profiles · ae5d typed D-Bus service")
            value: qsTr("Native Linux")
            showSeparator: false
        }
    }
}
