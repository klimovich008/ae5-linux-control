import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "components"
import "pages"

ApplicationWindow {
    id: root

    width: 1280
    height: 800
    minimumWidth: 1024
    minimumHeight: 680
    visible: true
    title: qsTr("AE5 Control")
    color: Theme.background
    palette.window: Theme.background
    palette.windowText: Theme.textPrimary
    palette.base: Theme.surface
    palette.text: Theme.textPrimary
    palette.button: Theme.surfaceRaised
    palette.buttonText: Theme.textPrimary
    palette.alternateBase: Theme.surfaceRaised
    palette.brightText: Theme.textPrimary
    palette.dark: Theme.surfaceSunken
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.background
    palette.light: Theme.surface
    palette.link: Theme.accent
    palette.mid: Theme.separatorStrong
    palette.midlight: Theme.separator
    palette.placeholderText: Theme.textDisabled
    palette.toolTipBase: Theme.surfaceRaised
    palette.toolTipText: Theme.textPrimary

    readonly property bool compact: width < Theme.compactBreakpoint
    readonly property bool wide: width >= Theme.wideBreakpoint
    readonly property bool effectsModified: appState.effectsState === "Modified"
    readonly property bool eqModified: appState.eqState === "Modified"
    property bool closeConfirmed: false
    property var closeReturnFocusItem: null

    function focusFirstUnsavedAction() {
        if (effectsSaveButton.visible)
            effectsSaveButton.forceActiveFocus()
        else if (eqSaveButton.visible)
            eqSaveButton.forceActiveFocus()
        else
            cancelCloseButton.forceActiveFocus()
    }

    function finishCloseIfClean() {
        if (appState.unsavedCount === 0) {
            closeConfirmed = true
            unsavedDialog.close()
            root.close()
        } else {
            Qt.callLater(root.focusFirstUnsavedAction)
        }
    }

    function saveBeforeClose(objectName) {
        const readOnly = objectName === "effects"
                         ? appState.effectsReadOnly : appState.eqReadOnly
        if (readOnly) {
            closeReturnFocusItem = null
            unsavedDialog.close()
            soundPage.reviewObject(objectName, true)
            return
        }

        if (objectName === "effects")
            appState.saveEffectsDraft()
        else
            appState.saveEqDraft()
        finishCloseIfClean()
    }

    function discardAndClose() {
        closeConfirmed = true
        unsavedDialog.close()
        root.close()
    }

    onClosing: function(closeEvent) {
        if (!closeConfirmed && appState.unsavedCount > 0) {
            closeEvent.accepted = false
            if (!unsavedDialog.visible) {
                closeReturnFocusItem = root.activeFocusItem
                unsavedDialog.open()
            }
        }
    }

    AppState {
        id: appState
    }

    Component.onCompleted: Qt.callLater(function() {
        appState.refreshFromDaemon()
    })

    Timer {
        interval: 5000
        repeat: true
        running: true
        onTriggered: appState.refreshFromDaemon()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            NavigationSidebar {
                Layout.preferredWidth: root.compact
                                       ? Theme.sidebarWidthCompact
                                       : root.wide ? Theme.sidebarWidthWide
                                                   : Theme.sidebarWidth
                Layout.fillHeight: true
                compact: root.compact
            }

            SoundPage {
                id: soundPage

                Layout.fillWidth: true
                Layout.fillHeight: true
                appState: appState
                compact: root.compact
            }
        }

        HardwareFaceplate {
            Layout.fillWidth: true
            Layout.preferredHeight: root.compact
                                    ? Theme.faceplateHeightCompact
                                    : Theme.faceplateHeight
            appState: appState
            compact: root.compact
            onReviewRequested: soundPage.reviewUnsaved()
        }
    }

    Shortcut {
        sequence: StandardKey.Save
        onActivated: soundPage.saveFocusedObject(false)
    }

    Shortcut {
        sequence: "Ctrl+Shift+S"
        onActivated: soundPage.saveFocusedObject(true)
    }

    Dialog {
        id: unsavedDialog

        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(520, root.width - 48)
        modal: true
        closePolicy: Popup.CloseOnEscape
        title: qsTr("Save changes before closing?")

        onOpened: Qt.callLater(root.focusFirstUnsavedAction)
        onClosed: {
            if (!root.closeConfirmed && root.closeReturnFocusItem)
                root.closeReturnFocusItem.forceActiveFocus()
            root.closeReturnFocusItem = null
        }

        contentItem: ColumnLayout {
            spacing: 14
            Accessible.role: Accessible.Grouping
            Accessible.name: unsavedDialog.title
            Accessible.description: qsTr("Effects profiles and EQ presets are saved independently.")

            Label {
                Layout.fillWidth: true
                text: qsTr("Choose each modified object you want to save. Discard closes without changing saved profiles or live audio.")
                color: Theme.textSecondary
                wrapMode: Text.WordWrap
            }

            RowLayout {
                visible: root.effectsModified
                Layout.fillWidth: true
                spacing: 12
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Modified Effects profile")
                Accessible.description: appState.effectsDetail

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Label {
                        text: qsTr("Effects · %1").arg(appState.effectsProfile)
                        color: Theme.textPrimary
                        font.weight: Font.DemiBold
                    }

                    Label {
                        Layout.fillWidth: true
                        text: appState.effectsDetail
                        color: Theme.textSecondary
                        wrapMode: Text.WordWrap
                    }
                }

                AppButton {
                    id: effectsSaveButton

                    Layout.minimumHeight: 40
                    variant: "primary"
                    text: appState.effectsReadOnly ? qsTr("Review Save as…") : qsTr("Save Effects")
                    Accessible.name: appState.effectsReadOnly
                                     ? qsTr("Review Effects profile and save as a new object")
                                     : qsTr("Save Effects profile %1").arg(appState.effectsProfile)
                    onClicked: root.saveBeforeClose("effects")
                }
            }

            RowLayout {
                visible: root.eqModified
                Layout.fillWidth: true
                spacing: 12
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Modified EQ preset")
                Accessible.description: appState.eqDetail

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Label {
                        text: qsTr("Equalizer · %1").arg(appState.eqPreset)
                        color: Theme.textPrimary
                        font.weight: Font.DemiBold
                    }

                    Label {
                        Layout.fillWidth: true
                        text: appState.eqDetail
                        color: Theme.textSecondary
                        wrapMode: Text.WordWrap
                    }
                }

                AppButton {
                    id: eqSaveButton

                    Layout.minimumHeight: 40
                    variant: "primary"
                    text: appState.eqReadOnly ? qsTr("Review Save as…") : qsTr("Save EQ")
                    Accessible.name: appState.eqReadOnly
                                     ? qsTr("Review EQ preset and save as a new object")
                                     : qsTr("Save EQ preset %1").arg(appState.eqPreset)
                    onClicked: root.saveBeforeClose("eq")
                }
            }
        }

        footer: DialogButtonBox {
            AppButton {
                text: qsTr("Discard and close")
                variant: "danger"
                DialogButtonBox.buttonRole: DialogButtonBox.DestructiveRole
                Accessible.description: qsTr("Discard local drafts and close without changing saved profiles or live audio")
                onClicked: root.discardAndClose()
            }

            AppButton {
                id: cancelCloseButton

                text: qsTr("Cancel")
                DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
                Accessible.description: qsTr("Return to AE5 Control without closing")
                onClicked: unsavedDialog.reject()
            }
        }
    }
}
