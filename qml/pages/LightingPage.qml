import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "../components"

PageScaffold {
    id: root

    readonly property bool previewAvailable: root.appState.qaMode
    readonly property string lightingReason:
        qsTr("The shared Rust lighting backend is verified, but its five-LED state and writes are not yet part of the typed ae5d QML contract.")
    readonly property var previewColors: ["#139CC0", "#139CC0", "#32B1D0",
                                          "#139CC0", "#0E7F9C"]

    pageTitle: qsTr("Lighting")
    pageDescription: qsTr("Configure the five onboard LEDs without letting RGB dominate the audio interface.")
    onRetryRequested: root.appState.refreshFromDaemon()

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Onboard lighting")
        detail: root.lightingReason
        statusText: root.previewAvailable ? qsTr("5 LEDs · QA") : qsTr("Integration pending")
        statusKind: root.previewAvailable ? "ready" : "partial"

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Unified color")
            detail: qsTr("One verified frame is sent to the complete five-LED chain.")
            value: root.previewAvailable ? qsTr("AE5 cyan") : qsTr("Unavailable")
            statusText: qsTr("Read only")
            statusKind: "partial"

            Rectangle {
                Layout.preferredWidth: 34
                Layout.preferredHeight: 34
                radius: Theme.radiusSmall
                color: root.previewAvailable ? root.previewColors[0] : Theme.surfaceSunken
                border.width: 1
                border.color: root.previewAvailable ? Theme.accent : Theme.separatorStrong
                Accessible.role: Accessible.Indicator
                Accessible.name: qsTr("Unified color preview")
            }

            AppButton {
                text: qsTr("Choose color")
                enabled: false
                blockedReason: root.lightingReason
            }
        }

        StatusRow {
            Layout.fillWidth: true
            title: qsTr("Restore at login")
            detail: qsTr("The packaged one-shot restore keeps lighting persistence separate from the GUI.")
            value: qsTr("User service")
            statusText: qsTr("Supported")
            statusKind: "ready"
            showSeparator: false
        }
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Individual LEDs")
        detail: qsTr("Each LED remains independently addressable through the Linux multicolor LED class.")
        statusText: root.previewAvailable ? qsTr("Preview") : qsTr("Unavailable")
        statusKind: root.previewAvailable ? "ready" : "partial"

        GridLayout {
            Layout.fillWidth: true
            columns: root.width >= 780 ? 5 : 3
            columnSpacing: Theme.space3
            rowSpacing: Theme.space3

            Repeater {
                model: 5

                delegate: Rectangle {
                    required property int index

                    Layout.fillWidth: true
                    Layout.preferredHeight: 92
                    radius: Theme.radiusSmall
                    color: Theme.surfaceSunken
                    border.width: 1
                    border.color: Theme.separator
                    Accessible.role: Accessible.Grouping
                    Accessible.name: qsTr("Onboard LED %1").arg(index + 1)

                    ColumnLayout {
                        anchors.centerIn: parent
                        spacing: Theme.space2

                        Rectangle {
                            Layout.alignment: Qt.AlignHCenter
                            Layout.preferredWidth: 34
                            Layout.preferredHeight: 34
                            radius: 17
                            color: root.previewAvailable
                                   ? root.previewColors[index]
                                   : Theme.surface
                            border.width: 2
                            border.color: root.previewAvailable
                                          ? root.previewColors[index]
                                          : Theme.separatorStrong
                        }

                        Label {
                            text: qsTr("LED %1").arg(index + 1)
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontCaption
                        }
                    }
                }
            }
        }
    }
}
