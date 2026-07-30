import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Rectangle {
    id: root

    property var appState
    property bool compact: false
    property bool wide: false
    property var highGainReturnFocusItem: null
    signal reviewRequested
    readonly property color statusColor: appState.statusCode === "ready" ? Theme.success
                                                  : appState.statusCode === "connecting" ? Theme.accent
                                                  : appState.statusCode === "partial" ? Theme.modified
                                                  : Theme.error

    color: Theme.faceplate
    border.color: Theme.separator
    Accessible.role: Accessible.StatusBar
    Accessible.name: qsTr("%1 hardware controls").arg(appState.deviceName)
    Accessible.description: appState.statusDetail

    Timer {
        id: volumeWriteDebounce

        interval: 180
        onTriggered: {
            const requested = Math.round(masterVolumeSlider.value)
            if (masterVolumeSlider.enabled && requested !== root.appState.masterVolume)
                root.appState.requestMasterVolume(requested)
        }
    }

    Connections {
        target: root.appState

        function onMasterVolumeChanged() {
            if (!masterVolumeSlider.pressed && !volumeWriteDebounce.running)
                masterVolumeSlider.value = root.appState.masterVolume
        }

        function onHardwareStateRevisionChanged() {
            volumeWriteDebounce.stop()
            if (!masterVolumeSlider.pressed)
                masterVolumeSlider.value = root.appState.masterVolume
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: root.compact ? Theme.space3 : Theme.space4
        anchors.rightMargin: root.compact ? Theme.space3 : Theme.space4
        anchors.topMargin: Theme.space2
        anchors.bottomMargin: Theme.space2
        spacing: root.compact || !root.wide ? Theme.space2 : Theme.space3

        ColumnLayout {
            Layout.minimumWidth: root.compact ? 150 : 190
            Layout.preferredWidth: root.compact ? 150 : 190
            spacing: Theme.space1

            Label {
                Layout.fillWidth: true
                text: root.appState.deviceName
                color: Theme.textPrimary
                font.pixelSize: Theme.fontLabel
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
                    font.pixelSize: Theme.fontCaption
                    Accessible.description: root.appState.statusDetail
                    ToolTip.visible: statusHover.hovered
                    ToolTip.text: root.appState.statusDetail

                    HoverHandler {
                        id: statusHover
                    }
                }

                Label {
                    visible: !root.compact
                             && root.appState.statusCode === "ready"
                             && root.appState.audioFormatAvailable
                    text: "· " + root.appState.audioFormat
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontCaption
                }
            }
        }

        Rectangle {
            Layout.preferredWidth: 1
            Layout.fillHeight: true
            color: Theme.separator
        }

        ColumnLayout {
            spacing: Theme.space1

            Label {
                text: qsTr("Output")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontCaption
            }

            RowLayout {
                spacing: Theme.space1

                Repeater {
                    model: ["Speakers", "Headphones", "Digital"]

                    delegate: AppButton {
                        id: outputButton

                        required property string modelData

                        objectName: "output-" + modelData.toLowerCase()
                        implicitWidth: root.compact
                                       ? 48
                                       : modelData === "Headphones" ? 112
                                       : modelData === "Speakers" ? 88 : 72
                        implicitHeight: Theme.controlHeight
                        text: root.compact
                              ? modelData === "Headphones" ? qsTr("HP")
                              : modelData === "Speakers" ? qsTr("SPK") : qsTr("DIG")
                              : modelData
                        checked: root.appState.output === modelData
                        checkable: true
                        tooltipText: root.compact ? modelData : ""
                        enabled: root.appState.outputAvailable
                                 && root.appState.outputWriteEnabled
                        blockedReason: root.appState.outputWriteBlockReason
                        Accessible.name: modelData
                        Accessible.description: enabled ? qsTr("Select %1 output").arg(modelData)
                                                        : root.appState.outputWriteBlockReason
                        onClicked: root.appState.selectPreviewOutput(modelData)
                    }
                }
            }
        }

        ColumnLayout {
            visible: !root.compact
            spacing: Theme.space1

            Label {
                text: qsTr("Headphone gain")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontCaption
            }

            RowLayout {
                spacing: Theme.space1

                Repeater {
                    model: ["Low", "Medium", "High"]

                    delegate: AppButton {
                        id: gainButton

                        required property string modelData
                        objectName: "gain-" + modelData.toLowerCase()
                        implicitWidth: root.wide
                                       ? modelData === "Medium" ? 84 : 72
                                       : modelData === "Medium" ? 76 : 66
                        implicitHeight: Theme.controlHeight
                        text: modelData
                        checked: root.appState.headphoneGain === modelData
                        checkable: true
                        enabled: root.appState.headphoneGainAvailable
                                 && root.appState.headphoneGainWriteEnabled
                                 && !root.appState.headphoneGainWriteInFlight
                        blockedReason: root.appState.headphoneGainWriteInFlight
                                       ? qsTr("Wait for the current headphone-gain transaction to finish.")
                                       : root.appState.headphoneGainWriteBlockReason
                        Accessible.name: qsTr("%1 headphone gain").arg(modelData)
                        Accessible.description: enabled
                                                ? modelData === "High"
                                                  ? qsTr("Requires confirmation because High gain can produce a large loudness increase.")
                                                  : qsTr("Briefly pauses the AE-5, applies %1 gain, and verifies hardware readback.").arg(modelData)
                                                : blockedReason
                        onClicked: {
                            if (modelData === "High") {
                                root.highGainReturnFocusItem = gainButton
                                highGainDialog.open()
                            } else {
                                root.appState.requestHeadphoneGain(modelData, false)
                            }
                        }
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
            Layout.minimumWidth: root.compact ? 150 : 220
            spacing: Theme.space1

            Label {
                text: qsTr("Master volume")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontCaption
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Theme.space2

                IconButton {
                    objectName: "master-mute"
                    iconName: root.appState.muted
                              ? "speaker-simple-x"
                              : root.appState.masterVolume === 0
                                ? "speaker-simple-none"
                                : root.appState.masterVolume <= 50
                                  ? "speaker-simple-low" : "speaker-simple-high"
                    accessibleName: root.appState.muted ? qsTr("Unmute") : qsTr("Mute")
                    variant: root.appState.muted ? "danger" : "ghost"
                    enabled: root.appState.muteAvailable
                             && root.appState.muteWriteEnabled
                    blockedReason: root.appState.hardwareWriteBlockReason
                    onClicked: root.appState.requestMuted(!root.appState.muted)
                }

                AppSlider {
                    id: masterVolumeSlider

                    objectName: "master-volume"
                    Layout.fillWidth: true
                    from: 0
                    to: 100
                    enabled: root.appState.volumeAvailable
                             && root.appState.volumeWriteEnabled
                    blockedReason: root.appState.hardwareWriteBlockReason
                    Accessible.name: qsTr("Master volume")
                    Accessible.description: enabled
                                            ? qsTr("%1 percent").arg(Math.round(value))
                                            : root.appState.hardwareWriteBlockReason
                    onValueChanged: {
                        if (enabled && Math.round(value) !== root.appState.masterVolume)
                            volumeWriteDebounce.restart()
                    }
                    Component.onCompleted: value = root.appState.masterVolume
                }

                Label {
                    Layout.preferredWidth: 38
                    text: root.appState.volumeAvailable ? root.appState.masterVolume + "%" : "—"
                    color: Theme.textPrimary
                    horizontalAlignment: Text.AlignRight
                    font.pixelSize: Theme.fontLabel
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.minimumWidth: 150
            Layout.preferredWidth: root.compact ? 150 : 190
            Layout.maximumWidth: root.compact ? 150 : 220
            spacing: Theme.space1

            Label {
                visible: !root.compact
                text: qsTr("Current setup")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontCaption
            }

            Label {
                Layout.fillWidth: true
                text: root.compact
                      ? qsTr("Gain %1 · FX %2 · EQ %3")
                        .arg(root.appState.headphoneGain)
                        .arg(root.appState.effectsProfile)
                        .arg(root.appState.eqPreset)
                      : qsTr("Effects: %1 · EQ: %2").arg(root.appState.effectsProfile).arg(root.appState.eqPreset)
                color: Theme.textPrimary
                font.pixelSize: Theme.fontCaption
                elide: Text.ElideRight
            }

            Label {
                visible: !root.compact
                Layout.fillWidth: true
                text: root.appState.profileStateLive
                      ? qsTr("Live state; save in each section")
                      : root.appState.qaMode
                        ? qsTr("QA preview · hardware writes disabled")
                      : root.appState.hardwareBacked && root.appState.connected
                        ? qsTr("Device live · drafts save by section")
                        : qsTr("Device unavailable · profiles read-only")
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMicro
                elide: Text.ElideRight
            }
        }

        AppButton {
            id: reviewButton

            objectName: "unsaved-review"
            visible: root.appState.unsavedCount > 0
            enabled: visible
            variant: "warning"
            text: root.wide
                  ? qsTr("Review · %1 unsaved").arg(root.appState.unsavedCount)
                  : qsTr("Review · %1").arg(root.appState.unsavedCount)
            tooltipText: qsTr("Review unsaved Effects and EQ changes")
            Accessible.name: qsTr("Review %1 unsaved changes").arg(root.appState.unsavedCount)
            Accessible.ignored: !visible
            onClicked: root.reviewRequested()
        }
    }

    Dialog {
        id: highGainDialog

        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(440, root.width - 48)
        modal: true
        closePolicy: Popup.CloseOnEscape
        title: qsTr("Use High headphone gain?")
        standardButtons: Dialog.Ok | Dialog.Cancel
        onOpened: {
            const acceptButton = standardButton(Dialog.Ok)
            if (acceptButton) {
                acceptButton.text = qsTr("Use High gain")
                acceptButton.forceActiveFocus()
            }
        }
        onAccepted: root.appState.requestHeadphoneGain("High", true)
        onClosed: {
            if (root.highGainReturnFocusItem)
                root.highGainReturnFocusItem.forceActiveFocus()
            root.highGainReturnFocusItem = null
        }

        contentItem: ColumnLayout {
            spacing: Theme.space3
            Accessible.role: Accessible.Grouping
            Accessible.name: highGainDialog.title
            Accessible.description: qsTr("High gain is intended for high-impedance headphones and can be much louder than Low gain.")

            Label {
                Layout.fillWidth: true
                text: qsTr("High gain is intended for 150–600 Ω headphones. Physical AE-5 measurements were about 7 dB louder than Low gain, so remove the headphones from your ears before continuing.")
                color: Theme.textPrimary
                wrapMode: Text.WordWrap
            }

            Label {
                Layout.fillWidth: true
                text: qsTr("The current PipeWire volume and mute state will be preserved.")
                color: Theme.textSecondary
                wrapMode: Text.WordWrap
            }
        }
    }
}
