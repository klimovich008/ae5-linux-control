import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "../components"

PageScaffold {
    id: root

    readonly property string channelReason:
        qsTr("This channel is available in the shared Rust mixer backend but is not yet exposed by ae5d.")

    pageTitle: qsTr("Mixer")
    pageDescription: qsTr("Review playback, capture and desktop routing layers without stacking a second master-volume control over the hardware footer.")
    onRetryRequested: root.appState.refreshFromDaemon()

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Playback channels")
        detail: qsTr("Master volume remains interactive only in the persistent footer. Additional ALSA channels are shown honestly as pending integration.")
        statusText: qsTr("Partial")
        statusKind: "partial"

        Repeater {
            model: [
                { name: qsTr("Master"), value: root.appState.masterVolume,
                  available: root.appState.volumeAvailable, detail: qsTr("AE-5-only Windows-tapered PipeWire volume") },
                { name: qsTr("PCM"), value: root.appState.qaMode ? 80 : 0,
                  available: root.appState.qaMode, detail: root.channelReason },
                { name: qsTr("Front"), value: root.appState.qaMode ? 19 : 0,
                  available: root.appState.qaMode, detail: root.channelReason },
                { name: qsTr("Surround"), value: root.appState.qaMode ? 0 : 0,
                  available: root.appState.qaMode, detail: root.channelReason }
            ]

            delegate: StatusRow {
                required property var modelData
                required property int index

                Layout.fillWidth: true
                title: modelData.name
                detail: modelData.detail
                value: modelData.available ? modelData.value + "%" : "—"
                statusText: modelData.name === qsTr("Master")
                            ? qsTr("Footer control")
                            : modelData.available ? qsTr("QA preview") : qsTr("Not exposed")
                statusKind: modelData.name === qsTr("Master") ? "ready" : "partial"
                showSeparator: index < 3

                AppSlider {
                    Layout.preferredWidth: 180
                    from: 0
                    to: 100
                    value: modelData.value
                    enabled: false
                    blockedReason: modelData.name === qsTr("Master")
                                   ? qsTr("Use the persistent footer to change master volume.")
                                   : root.channelReason
                    Accessible.name: qsTr("%1 level").arg(modelData.name)
                }
            }
        }
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Recording channels")
        detail: qsTr("Capture gain and loopback monitoring need a typed daemon contract before this page can write them.")
        statusText: qsTr("Read only")
        statusKind: "partial"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Capture")
            detail: root.channelReason
            value: root.appState.qaMode ? "68%" : "—"
            statusText: qsTr("Not exposed")
            statusKind: "partial"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("What U Hear")
            detail: qsTr("Driver capture is verified; stream-level monitoring and volume remain deferred.")
            value: qsTr("Capture PCM")
            statusText: qsTr("Supported")
            statusKind: "ready"
            showSeparator: false
        }
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Desktop routing")
        detail: qsTr("PipeWire default nodes are independent from the AE-5 hardware mixer.")
        statusText: root.appState.outputAvailable ? qsTr("Playback known") : qsTr("Unavailable")
        statusKind: root.appState.outputAvailable ? "ready" : "unavailable"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Playback output")
            detail: qsTr("Current AE-5 route reported by ae5d")
            value: root.appState.output
            statusText: qsTr("Current")
            statusKind: "ready"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Recording input")
            detail: root.channelReason
            value: qsTr("Not exposed")
            statusText: qsTr("Pending")
            statusKind: "partial"
            showSeparator: false
        }
    }
}
