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
                       ? qsTr("Volume, mute and profile saving are live; audio apply remains a safe preview")
                       : root.appState.deviceStatus
                detail: root.appState.statusCode === "ready"
                        ? root.appState.profileCatalogDetail + " "
                          + qsTr("Editing changes a draft; saving writes only that section, while live audio remains unchanged.")
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
                    statusDetail: root.appState.eqDetail
                    readOnly: root.appState.eqReadOnly
                    options: root.appState.eqPresetNames
                    onSelectionRequested: name => root.appState.selectEqPreset(name)
                    onSaveRequested: root.appState.saveEqDraft()
                    onSaveAsRequested: name => root.appState.saveEqDraftAs(name)
                    onRevertRequested: root.appState.revertEqDraft()
                }

                EqualizerGraph {
                    Layout.fillWidth: true
                    Layout.minimumHeight: 180
                    Layout.preferredHeight: root.compact ? 190 : 214
                    appState: root.appState
                    editingEnabled: root.appState.profileCatalogStatus === "ready"
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
                    statusDetail: root.appState.effectsDetail
                    readOnly: root.appState.effectsReadOnly
                    options: root.appState.effectsProfileNames
                    onSelectionRequested: name => root.appState.selectEffectsProfile(name)
                    onSaveRequested: root.appState.saveEffectsDraft()
                    onSaveAsRequested: name => root.appState.saveEffectsDraftAs(name)
                    onRevertRequested: root.appState.revertEffectsDraft()
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
                        controlKey: "surround"
                        title: qsTr("Surround")
                        initialValue: root.appState.surroundLevel
                        initiallyEnabled: root.appState.surroundEnabled
                        available: root.appState.surroundAvailable
                        editingEnabled: root.appState.profileCatalogStatus === "ready"
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        controlKey: "crystalizer"
                        title: qsTr("Crystalizer")
                        initialValue: root.appState.crystalizerLevel
                        initiallyEnabled: root.appState.crystalizerEnabled
                        available: root.appState.crystalizerAvailable
                        editingEnabled: root.appState.profileCatalogStatus === "ready"
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        controlKey: "bass"
                        title: qsTr("Bass")
                        initialValue: root.appState.bassLevel
                        initiallyEnabled: root.appState.bassEnabled
                        available: root.appState.bassAvailable
                        editingEnabled: root.appState.profileCatalogStatus === "ready"
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        controlKey: "smart-volume"
                        title: qsTr("Smart Volume")
                        initialValue: root.appState.smartVolumeLevel
                        initiallyEnabled: root.appState.smartVolumeEnabled
                        available: root.appState.smartVolumeAvailable
                        editingEnabled: root.appState.profileCatalogStatus === "ready"
                        leftPole: qsTr("Night")
                        rightPole: qsTr("Loud")
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        controlKey: "dialog"
                        title: qsTr("Dialog+")
                        initialValue: root.appState.dialogLevel
                        initiallyEnabled: root.appState.dialogEnabled
                        available: root.appState.dialogAvailable
                        editingEnabled: root.appState.profileCatalogStatus === "ready"
                        controlsEnabled: !root.appState.directMode
                    }
                }
            }
        }
    }
}
