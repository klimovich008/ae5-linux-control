import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "../components"

Rectangle {
    id: root

    property var appState
    property bool compact: false

    color: Theme.background

    ScrollView {
        anchors.fill: parent
        clip: true
        contentWidth: availableWidth

        ColumnLayout {
            width: parent.width
            spacing: 10

            Item {
                Layout.preferredHeight: 8
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: root.compact ? 22 : 32
                Layout.rightMargin: root.compact ? 22 : 32
                spacing: 4

                Label {
                    text: qsTr("Sound")
                    color: Theme.textPrimary
                    font.pixelSize: 28
                    font.weight: Font.DemiBold
                }

                Label {
                    text: qsTr("Hardware output, Effects profiles and EQ presets remain separate.")
                    color: Theme.textSecondary
                    font.pixelSize: 13
                }
            }

            CapabilityNotice {
                Layout.fillWidth: true
                Layout.leftMargin: root.compact ? 22 : 32
                Layout.rightMargin: root.compact ? 22 : 32
                statusCode: root.appState.statusCode
                title: root.appState.statusCode === "ready"
                       ? qsTr("Volume and mute are live; profile selection is a safe preview")
                       : root.appState.deviceStatus
                detail: root.appState.statusCode === "ready"
                        ? root.appState.profileCatalogDetail + " "
                          + qsTr("Selecting EQ or Effects changes this view only; applying and saving audio changes are not connected yet.")
                        : root.appState.statusDetail
                onRetryRequested: root.appState.refreshFromDaemon()
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: root.compact ? 22 : 32
                Layout.rightMargin: root.compact ? 22 : 32
                spacing: 10

                ObjectHeader {
                    Layout.fillWidth: true
                    objectTitle: qsTr("Equalizer")
                    selectorLabel: qsTr("EQ preset")
                    currentName: root.appState.eqPreset
                    stateText: root.appState.eqState
                    cleanSubtitle: qsTr("%1 preset loaded for preview; live audio is unchanged.")
                                   .arg(root.appState.eqSource)
                    modifiedSubtitle: qsTr("Applied in preview. Not yet saved to \"%1\".").arg(root.appState.eqPreset)
                    options: root.appState.eqPresetNames
                    onSelectionRequested: name => root.appState.selectEqPreset(name)
                    onSaveRequested: root.appState.saveEqPreview()
                }

                EqualizerGraph {
                    Layout.fillWidth: true
                    Layout.minimumHeight: 180
                    Layout.preferredHeight: root.compact ? 190 : 214
                    appState: root.appState
                    editingEnabled: false
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.leftMargin: root.compact ? 22 : 32
                Layout.rightMargin: root.compact ? 22 : 32
                Layout.preferredHeight: 1
                color: Theme.separator
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: root.compact ? 22 : 32
                Layout.rightMargin: root.compact ? 22 : 32
                Layout.bottomMargin: 20
                spacing: 8

                ObjectHeader {
                    Layout.fillWidth: true
                    objectTitle: qsTr("Effects")
                    selectorLabel: qsTr("Effects profile")
                    currentName: root.appState.effectsProfile
                    stateText: root.appState.effectsState
                    cleanSubtitle: qsTr("%1 profile loaded for preview; live audio is unchanged.")
                                   .arg(root.appState.effectsSource)
                    modifiedSubtitle: qsTr("Applied in preview. Not yet saved to \"%1\".").arg(root.appState.effectsProfile)
                    options: root.appState.effectsProfileNames
                    onSelectionRequested: name => root.appState.selectEffectsProfile(name)
                    onSaveRequested: root.appState.saveEffectsPreview()
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: directModeRow.implicitHeight + 12
                    radius: Theme.radiusSmall
                    color: root.appState.directMode ? Qt.rgba(0.61, 0.45, 0.96, 0.12)
                                                        : Theme.surface
                    border.color: root.appState.directMode ? Theme.focus : Theme.separator

                    RowLayout {
                        id: directModeRow

                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.leftMargin: 12
                        anchors.rightMargin: 12
                        spacing: 12

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: 2

                            Label {
                                text: qsTr("Direct Mode")
                                color: Theme.textPrimary
                                font.pixelSize: 14
                            }

                            Label {
                                text: root.appState.directMode
                                      ? qsTr("EQ and enhancements are visible but bypassed.")
                                      : qsTr("Bypasses EQ and enhancements when enabled.")
                                color: Theme.textSecondary
                                font.pixelSize: 11
                            }
                        }

                        Switch {
                            checked: root.appState.directMode
                            enabled: root.appState.directModeAvailable
                                     && root.appState.directModeWriteEnabled
                            Accessible.name: qsTr("Direct Mode")
                            Accessible.description: enabled
                                                    ? qsTr("Toggle Direct Mode")
                                                    : root.appState.directModeAvailable
                                                      ? root.appState.hardwareWriteBlockReason
                                                      : qsTr("Direct Mode is not exposed by the current driver.")
                            ToolTip.visible: hovered && !enabled
                            ToolTip.text: Accessible.description
                            onToggled: root.appState.setPreviewDirectMode(checked)
                        }
                    }
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: root.width >= 1300 ? 2 : 1
                    columnSpacing: 28
                    rowSpacing: 0

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        title: qsTr("Surround")
                        initialValue: root.appState.surroundLevel
                        initiallyEnabled: root.appState.surroundEnabled
                        available: root.appState.surroundAvailable
                        editingEnabled: false
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        title: qsTr("Crystalizer")
                        initialValue: root.appState.crystalizerLevel
                        initiallyEnabled: root.appState.crystalizerEnabled
                        available: root.appState.crystalizerAvailable
                        editingEnabled: false
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        title: qsTr("Bass")
                        initialValue: root.appState.bassLevel
                        initiallyEnabled: root.appState.bassEnabled
                        available: root.appState.bassAvailable
                        editingEnabled: false
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        title: qsTr("Smart Volume")
                        initialValue: root.appState.smartVolumeLevel
                        initiallyEnabled: root.appState.smartVolumeEnabled
                        available: root.appState.smartVolumeAvailable
                        editingEnabled: false
                        leftPole: qsTr("Night")
                        rightPole: qsTr("Loud")
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        title: qsTr("Dialog+")
                        initialValue: root.appState.dialogLevel
                        initiallyEnabled: root.appState.dialogEnabled
                        available: root.appState.dialogAvailable
                        editingEnabled: false
                        controlsEnabled: !root.appState.directMode
                    }
                }
            }
        }
    }
}
