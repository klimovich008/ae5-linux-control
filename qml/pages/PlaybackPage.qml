import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "../components"

PageScaffold {
    id: root

    signal navigateRequested(string page)

    pageTitle: qsTr("Playback")
    pageDescription: qsTr("Inspect the active analog path, format and speaker capabilities. Output, gain and volume remain in the persistent hardware footer.")
    onRetryRequested: root.appState.refreshFromDaemon()

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Active playback path")
        detail: qsTr("One authoritative global control strip prevents route and volume controls from being duplicated across pages.")
        statusText: root.appState.outputAvailable ? qsTr("Ready") : qsTr("Unavailable")
        statusKind: root.appState.outputAvailable ? "ready" : "unavailable"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Current output")
            detail: qsTr("Switch Speakers, Headphones or Digital from the footer.")
            value: root.appState.output
            statusText: root.appState.outputAvailable ? qsTr("Active") : qsTr("Unknown")
            statusKind: root.appState.outputAvailable ? "ready" : "unavailable"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Headphone gain")
            detail: root.appState.output === "Headphones"
                    ? qsTr("Gain is reported by the AE-5 hardware state.")
                    : qsTr("Available when the headphone output is selected.")
            value: root.appState.headphoneGain
            statusText: root.appState.headphoneGainAvailable ? qsTr("Detected") : qsTr("Unavailable")
            statusKind: root.appState.headphoneGainAvailable ? "ready" : "unavailable"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Transport format")
            detail: qsTr("PipeWire negotiates stream format independently from Creative's Windows quality label.")
            value: root.appState.audioFormat
            statusText: root.appState.audioFormatAvailable ? qsTr("Current") : qsTr("Unknown")
            statusKind: root.appState.audioFormatAvailable ? "ready" : "partial"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Sample rate policy")
            detail: qsTr("Automatic follows PipeWire. A fixed 48 or 96 kHz change briefly mutes and reopens only the AE-5, then verifies the negotiated S16 transport before restoring mute.")
            statusText: root.appState.sampleRateWriteInFlight
                        ? qsTr("Applying")
                        : root.appState.sampleRatePolicyAvailable
                          ? qsTr("Current") : qsTr("Unavailable")
            statusKind: root.appState.sampleRateWriteInFlight
                        ? "applying"
                        : root.appState.sampleRatePolicyAvailable
                          ? "ready" : "unavailable"
            showSeparator: false

            AppComboBox {
                id: sampleRatePicker

                function syncPolicy() {
                    const index = model.indexOf(root.appState.sampleRatePolicy)
                    if (index >= 0 && currentIndex !== index)
                        currentIndex = index
                }

                objectName: "sample-rate-policy"
                Layout.preferredWidth: 164
                model: ["Automatic", "48 kHz", "96 kHz"]
                enabled: root.appState.sampleRateWriteEnabled
                         && !root.appState.sampleRateWriteInFlight
                blockedReason: root.appState.sampleRateWriteInFlight
                               ? qsTr("Wait for the current sample-rate transition to finish.")
                               : root.appState.sampleRateWriteBlockReason
                Accessible.name: qsTr("AE-5 sample rate policy")
                Accessible.description: enabled
                                        ? qsTr("Select Automatic, 48 kHz, or 96 kHz. The AE-5 may be briefly muted.")
                                        : blockedReason
                Component.onCompleted: syncPolicy()
                onActivated: root.appState.requestSampleRatePolicy(currentText)

                Connections {
                    target: root.appState

                    function onSampleRatePolicyChanged() {
                        sampleRatePicker.syncPolicy()
                    }
                }
            }
        }
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Speaker configuration")
        detail: qsTr("The Rust backend already validates these controls; the typed QML write contract remains intentionally gated.")
        statusText: qsTr("Partial")
        statusKind: "partial"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Speaker layout")
            detail: qsTr("2.0, 2.1, 4.0, 4.1 and 5.1 are supported by ALSA and PipeWire.")
            value: qsTr("2.0 stereo")
            statusText: qsTr("Read only")
            statusKind: "partial"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Bass redirection")
            detail: qsTr("Available only for compatible LFE layouts and still awaiting physical acceptance.")
            value: qsTr("Off")
            statusText: qsTr("Deferred")
            statusKind: "partial"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("PCM roll-off filter")
            detail: qsTr("Slow Roll Off, Minimum Phase and Fast Roll Off exist in the driver; response verification is still open.")
            value: qsTr("Driver control")
            statusText: qsTr("Deferred")
            statusKind: "partial"
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Direct Mode")
            detail: root.appState.directModeAvailable
                    ? root.appState.hardwareWriteBlockReason
                    : qsTr("The production driver does not expose a verified safe Direct Mode transition.")
            value: root.appState.directMode ? qsTr("Active") : qsTr("Off")
            statusText: root.appState.directModeAvailable ? qsTr("Guarded") : qsTr("Unsupported")
            statusKind: root.appState.directModeAvailable ? "partial" : "unavailable"
            showSeparator: false
        }
    }

    RowLayout {
        Layout.fillWidth: true

        Label {
            Layout.fillWidth: true
            text: qsTr("Sound effects and Direct Mode conflicts are explained on the Sound page.")
            color: Theme.textSecondary
            font.pixelSize: Theme.fontCaption
            wrapMode: Text.Wrap
        }

        AppButton {
            text: qsTr("Open Sound")
            onClicked: root.navigateRequested("sound")
        }
    }
}
