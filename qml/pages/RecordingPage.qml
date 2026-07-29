import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "../components"

PageScaffold {
    id: root

    readonly property bool previewValues: root.appState.qaMode
    readonly property string contractReason:
        qsTr("Capture controls are available in the shared Rust backend, but are not yet exposed by the typed ae5d QML contract.")

    pageTitle: qsTr("Recording")
    pageDescription: qsTr("Inspect input capabilities and understand exactly which capture operations are available through this QML build.")
    onRetryRequested: root.appState.refreshFromDaemon()

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Recording source")
        detail: qsTr("The physical input and desktop default-input route remain separate.")
        statusText: root.previewValues ? qsTr("QA preview") : qsTr("Integration pending")
        statusKind: "partial"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Input source")
            detail: root.contractReason
            value: root.previewValues ? qsTr("Microphone") : qsTr("Unavailable")
            statusText: qsTr("Read only")
            statusKind: "partial"

            AppComboBox {
                Layout.preferredWidth: 180
                model: [qsTr("Microphone"), qsTr("Front Microphone"), qsTr("Line In")]
                currentIndex: 0
                enabled: false
                blockedReason: root.contractReason
                Accessible.name: qsTr("Recording source")
                Accessible.description: root.contractReason
            }
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Capture level")
            detail: root.contractReason
            value: root.previewValues ? "68%" : "—"
            statusText: qsTr("Read only")
            statusKind: "partial"

            AppSlider {
                Layout.preferredWidth: 180
                from: 0
                to: 100
                value: root.previewValues ? 68 : 0
                enabled: false
                blockedReason: root.contractReason
                Accessible.name: qsTr("Capture level")
            }
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Microphone boost")
            detail: qsTr("The driver exposes 0, 10, 20 and 30 dB steps.")
            value: root.previewValues ? "10 dB" : "—"
            statusText: qsTr("Deferred")
            statusKind: "partial"
            showSeparator: false
        }
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Capture processing")
        detail: qsTr("Unavailable features remain visible with their cause; no control silently writes the unsafe hardware DSP.")
        statusText: qsTr("Guarded")
        statusKind: "partial"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Noise reduction")
            detail: qsTr("Hardware InFX writes are not part of the safe QML contract.")
            value: qsTr("Off")
            statusText: qsTr("Not implemented")
            statusKind: "unavailable"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Voice focus")
            detail: qsTr("The CA0132 control exists, but its physical behavior still needs measurement.")
            value: qsTr("Off")
            statusText: qsTr("Deferred")
            statusKind: "partial"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("What U Hear")
            detail: qsTr("The retained driver capture PCM has verified stereo channel identity.")
            value: qsTr("Driver path")
            statusText: qsTr("Supported")
            statusKind: "ready"
            showSeparator: false
        }
    }
}
