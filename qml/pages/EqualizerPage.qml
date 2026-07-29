import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "../components"

PageScaffold {
    id: root

    readonly property var frequencies: ["31 Hz", "62 Hz", "125 Hz", "250 Hz",
                                         "500 Hz", "1 kHz", "2 kHz", "4 kHz",
                                         "8 kHz", "16 kHz"]

    function saveCurrent(forceSaveAs) {
        detailHeader.saveCurrent(forceSaveAs)
    }

    pageTitle: qsTr("Equalizer")
    pageDescription: qsTr("Edit the selected ten-band preset precisely. Saving the preset and applying it to live audio remain separate actions.")
    onRetryRequested: root.appState.refreshFromDaemon()

    ObjectHeader {
        id: detailHeader

        Layout.fillWidth: true
        objectKey: "eq-detail"
        objectTitle: qsTr("EQ preset")
        selectorLabel: qsTr("Preset")
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

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: Theme.controlHeightLarge + Theme.space2
        radius: Theme.radiusSmall
        color: Theme.surface
        border.color: root.appState.softwareEqState === "error"
                      ? Theme.error
                      : root.appState.softwareEqActive ? Theme.accent : Theme.separator

        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Theme.space3
            anchors.rightMargin: Theme.space3
            spacing: Theme.space2

            Label {
                text: qsTr("Live EQ")
                color: Theme.textPrimary
                font.pixelSize: Theme.fontLabel
                font.weight: Font.DemiBold
            }

            StateBadge {
                stateKind: root.appState.softwareEqState
                stateText: root.appState.softwareEqActive ? qsTr("Active") : qsTr("Inactive")
            }

            Label {
                Layout.fillWidth: true
                text: root.appState.softwareEqDetail
                color: Theme.textSecondary
                font.pixelSize: Theme.fontCaption
                elide: Text.ElideRight
            }

            AppButton {
                visible: root.appState.softwareEqActive
                enabled: visible && root.appState.softwareEqState !== "applying"
                Accessible.ignored: !visible
                text: qsTr("Disable")
                onClicked: root.appState.disableSoftwareEq()
            }

            AppButton {
                enabled: root.appState.eqEnabled
                         && root.appState.eqApplyAvailable
                         && root.appState.softwareEqState !== "applying"
                blockedReason: root.appState.eqApplyBlockReason
                text: qsTr("Apply EQ")
                variant: "primary"
                onClicked: root.appState.applyEqDraft()
            }
        }
    }

    EqualizerGraph {
        Layout.fillWidth: true
        Layout.minimumHeight: 240
        Layout.preferredHeight: root.compact ? 240 : 280
        appState: root.appState
        editingEnabled: root.appState.profileCatalogStatus === "ready"
    }

    SectionPanel {
        Layout.fillWidth: true
        title: qsTr("Band values")
        detail: qsTr("Use the sliders for fine adjustment or keyboard arrows for exact steps.")
        statusText: qsTr("±12 dB")
        statusKind: "ready"

        GridLayout {
            Layout.fillWidth: true
            columns: root.width >= 820 ? 2 : 1
            columnSpacing: Theme.space5
            rowSpacing: Theme.space1

            Repeater {
                model: root.frequencies

                delegate: EqualizerBandRow {
                    required property string modelData
                    required property int index

                    Layout.fillWidth: true
                    appState: root.appState
                    bandIndex: index
                    frequency: modelData
                    editingEnabled: root.appState.profileCatalogStatus === "ready"
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: Theme.space3

            Item {
                Layout.fillWidth: true
            }

            AppButton {
                text: qsTr("Flat")
                enabled: root.appState.profileCatalogStatus === "ready"
                blockedReason: qsTr("The profile library is unavailable.")
                onClicked: {
                    for (let index = 0; index < 10; ++index)
                        root.appState.updateEqBand(index, 0)
                }
            }

            AppButton {
                text: qsTr("Revert preset")
                enabled: root.appState.eqState === "Modified"
                blockedReason: qsTr("The selected preset has no local changes.")
                onClicked: root.appState.revertEqDraft()
            }
        }
    }
}
