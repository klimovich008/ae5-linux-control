import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Rectangle {
    id: root

    property var appState
    property bool compact: false
    readonly property color statusColor: appState.statusCode === "ready" ? Theme.success
                                                  : appState.statusCode === "connecting" ? Theme.accent
                                                  : appState.statusCode === "partial" ? Theme.modified
                                                  : Theme.error

    color: "#06131F"
    border.color: Theme.separator

    Timer {
        id: volumeWriteDebounce

        interval: 180
        onTriggered: {
            const requested = Math.round(masterVolumeSlider.value)
            if (masterVolumeSlider.enabled && requested !== root.appState.masterVolume)
                root.appState.requestMasterVolume(requested)
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: root.compact ? 14 : 20
        anchors.rightMargin: root.compact ? 14 : 20
        anchors.topMargin: 10
        anchors.bottomMargin: 10
        spacing: root.compact ? 12 : 20

        ColumnLayout {
            Layout.preferredWidth: root.compact ? 140 : 190
            spacing: 2

            Label {
                Layout.fillWidth: true
                text: root.appState.deviceName
                color: Theme.textPrimary
                font.pixelSize: 13
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }

            RowLayout {
                spacing: 6

                Rectangle {
                    Layout.preferredWidth: 7
                    Layout.preferredHeight: 7
                    radius: 4
                    color: root.statusColor
                }

                Label {
                    text: root.appState.deviceStatus
                    color: root.statusColor
                    font.pixelSize: 11
                    ToolTip.visible: statusArea.containsMouse
                    ToolTip.text: root.appState.statusDetail

                    MouseArea {
                        id: statusArea
                        anchors.fill: parent
                        hoverEnabled: true
                        acceptedButtons: Qt.NoButton
                    }
                }

                Label {
                    visible: !root.compact && root.appState.audioFormatAvailable
                    text: "· " + root.appState.audioFormat
                    color: Theme.textSecondary
                    font.pixelSize: 11
                }
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.fillHeight: true
            color: Theme.separator
        }

        ColumnLayout {
            spacing: 4

            Label {
                text: qsTr("OUTPUT")
                color: Theme.textSecondary
                font.pixelSize: 10
                font.letterSpacing: 0.8
            }

            RowLayout {
                spacing: 2

                Repeater {
                    model: root.compact ? ["Speakers", "Headphones"] : ["Speakers", "Headphones", "Digital"]

                    delegate: Button {
                        id: outputButton

                        required property string modelData

                        text: root.compact ? (modelData === "Headphones" ? qsTr("HP") : qsTr("SPK")) : modelData
                        checked: root.appState.output === modelData
                        checkable: true
                        enabled: root.appState.outputAvailable
                                 && root.appState.outputWriteEnabled
                        Accessible.name: modelData
                        Accessible.description: enabled ? qsTr("Select %1 output").arg(modelData)
                                                        : root.appState.outputWriteBlockReason
                        ToolTip.visible: hovered && !enabled
                        ToolTip.text: Accessible.description
                        onClicked: root.appState.selectPreviewOutput(modelData)

                        background: Rectangle {
                            radius: Theme.radiusSmall
                            color: outputButton.checked ? Qt.rgba(0, 0.78, 0.9, 0.16) : Theme.surface
                            border.color: outputButton.checked ? Theme.accent : Theme.separator
                        }

                        contentItem: Label {
                            text: outputButton.text
                            color: outputButton.checked ? Theme.textPrimary : Theme.textSecondary
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                            font.pixelSize: 11
                        }
                    }
                }
            }
        }

        ColumnLayout {
            visible: !root.compact
            spacing: 4

            Label {
                text: qsTr("GAIN")
                color: Theme.textSecondary
                font.pixelSize: 10
                font.letterSpacing: 0.8
            }

            RowLayout {
                spacing: 2

                Repeater {
                    model: ["Low", "Medium", "High"]

                    delegate: Button {
                        required property string modelData
                        text: modelData
                        checked: root.appState.headphoneGain === modelData
                        checkable: true
                        enabled: root.appState.headphoneGainAvailable
                                 && root.appState.headphoneGainWriteEnabled
                        Accessible.name: qsTr("%1 headphone gain").arg(modelData)
                        Accessible.description: modelData === "High"
                                                ? qsTr("High gain requires a deliberate safety confirmation.")
                                                : root.appState.hardwareWriteBlockReason
                        ToolTip.visible: hovered
                        ToolTip.text: Accessible.description
                    }
                }
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.fillHeight: true
            color: Theme.separator
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.minimumWidth: 170
            spacing: 3

            Label {
                text: qsTr("MASTER VOLUME")
                color: Theme.textSecondary
                font.pixelSize: 10
                font.letterSpacing: 0.8
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                ToolButton {
                    display: AbstractButton.IconOnly
                    icon.name: root.appState.muted ? "audio-volume-muted-symbolic"
                                                       : "audio-volume-medium-symbolic"
                    icon.color: root.appState.muted ? Theme.error : Theme.textPrimary
                    enabled: root.appState.muteAvailable
                             && root.appState.muteWriteEnabled
                    Accessible.name: root.appState.muted ? qsTr("Unmute") : qsTr("Mute")
                    Accessible.description: enabled ? Accessible.name
                                                    : root.appState.hardwareWriteBlockReason
                    ToolTip.visible: hovered && !enabled
                    ToolTip.text: Accessible.description
                    onClicked: root.appState.requestMuted(!root.appState.muted)
                }

                Slider {
                    id: masterVolumeSlider

                    Layout.fillWidth: true
                    from: 0
                    to: 100
                    value: root.appState.masterVolume
                    enabled: root.appState.volumeAvailable
                             && root.appState.volumeWriteEnabled
                    focusPolicy: Qt.StrongFocus
                    Accessible.name: qsTr("Master volume")
                    Accessible.description: enabled
                                            ? qsTr("%1 percent").arg(Math.round(value))
                                            : root.appState.hardwareWriteBlockReason
                    onValueChanged: {
                        if (enabled && Math.round(value) !== root.appState.masterVolume)
                            volumeWriteDebounce.restart()
                    }
                }

                Label {
                    Layout.preferredWidth: 38
                text: root.appState.volumeAvailable ? root.appState.masterVolume + "%" : "—"
                    color: Theme.textPrimary
                    horizontalAlignment: Text.AlignRight
                    font.pixelSize: 13
                    font.family: "monospace"
                }
            }
        }

        ColumnLayout {
            Layout.preferredWidth: root.compact ? 150 : 180
            spacing: 2

            Label {
                visible: !root.compact
                text: qsTr("CURRENT SETUP")
                color: Theme.textSecondary
                font.pixelSize: 10
                font.letterSpacing: 0.8
            }

            Label {
                Layout.fillWidth: true
                text: root.compact
                      ? qsTr("%1 · %2").arg(root.appState.effectsProfile).arg(root.appState.eqPreset)
                      : qsTr("Effects: %1 · EQ: %2").arg(root.appState.effectsProfile).arg(root.appState.eqPreset)
                color: Theme.textPrimary
                font.pixelSize: 11
                elide: Text.ElideRight
            }

            Label {
                visible: !root.compact
                text: root.appState.profileStateLive
                      ? qsTr("Live state; save in each section")
                      : root.appState.hardwareBacked && root.appState.connected
                        ? qsTr("Device live · profiles preview")
                        : qsTr("Device unavailable · profiles preview")
                color: Theme.textSecondary
                font.pixelSize: 10
            }
        }

        Button {
            id: reviewButton

            visible: root.appState.unsavedCount > 0
            enabled: visible
            text: root.compact ? qsTr("%1 unsaved").arg(root.appState.unsavedCount)
                               : qsTr("%1 unsaved\nReview").arg(root.appState.unsavedCount)
            flat: true
            Accessible.name: qsTr("Review %1 unsaved changes").arg(root.appState.unsavedCount)
            Accessible.ignored: !visible

            contentItem: Label {
                text: reviewButton.text
                color: Theme.modified
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                font.pixelSize: 11
                font.weight: Font.DemiBold
                Accessible.ignored: !reviewButton.visible
            }
        }
    }
}
