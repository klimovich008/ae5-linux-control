import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Item {
    id: root

    property string objectKey
    property string objectTitle
    property string selectorLabel
    property string currentName
    property string stateText
    property string statusDetail
    property bool readOnly: true
    property var options: []
    property var returnFocusItem: null
    readonly property bool modified: stateText === "Modified"
    readonly property bool modalOpen: saveAsDialog.visible

    signal saveRequested
    signal saveAsRequested(string name)
    signal revertRequested
    signal selectionRequested(string name)

    implicitHeight: headerLayout.implicitHeight
    Accessible.role: Accessible.Grouping
    Accessible.name: objectTitle
    Accessible.description: statusDetail

    function openSaveAs() {
        returnFocusItem = root.Window.window ? root.Window.window.activeFocusItem : null
        saveAsName.text = currentName
        saveAsDialog.open()
    }

    function focusEditor() {
        const candidates = [selector, actionsButton, revertButton, saveButton]
        for (let index = 0; index < candidates.length; ++index) {
            if (candidates[index].visible && candidates[index].enabled) {
                candidates[index].forceActiveFocus(Qt.TabFocusReason)
                return true
            }
        }
        return false
    }

    function saveCurrent(forceSaveAs) {
        if (forceSaveAs || readOnly)
            openSaveAs()
        else if (modified)
            saveRequested()
    }

    RowLayout {
        id: headerLayout

        anchors.left: parent.left
        anchors.right: parent.right
        spacing: Theme.space3

        ColumnLayout {
            Layout.fillWidth: true
            Layout.minimumWidth: 180
            Layout.preferredWidth: 180
            spacing: Theme.space1

            Label {
                text: root.objectTitle
                color: Theme.textPrimary
                font.pixelSize: Theme.fontSectionTitle
                font.weight: Font.DemiBold
            }

            Label {
                id: detailLabel

                Layout.fillWidth: true
                text: root.statusDetail
                color: Theme.textSecondary
                font.pixelSize: Theme.fontCaption
                elide: Text.ElideRight
                ToolTip.visible: truncated && detailHover.hovered
                ToolTip.text: text

                HoverHandler {
                    id: detailHover
                }
            }
        }

        RowLayout {
            Layout.preferredWidth: 308
            Layout.minimumWidth: 308
            Layout.maximumWidth: 308
            Layout.alignment: Qt.AlignVCenter
            spacing: Theme.space2

            Label {
                Layout.preferredWidth: 80
                text: root.selectorLabel
                color: Theme.textSecondary
                font.pixelSize: Theme.fontCaption
                horizontalAlignment: Text.AlignRight
            }

            AppComboBox {
                id: selector

                objectName: root.objectKey + "-selector"
                Layout.preferredWidth: 220
                Layout.minimumWidth: 220
                Layout.maximumWidth: 220
                model: root.options
                currentIndex: {
                    for (let index = 0; index < root.options.length; ++index) {
                        if (root.options[index] === root.currentName)
                            return index
                    }
                    return -1
                }
                enabled: root.options.length > 0 && !root.modified
                blockedReason: root.modified
                               ? qsTr("Save or revert this draft before selecting another object.")
                               : ""
                Accessible.name: root.selectorLabel
                Accessible.description: root.modified
                                        ? qsTr("Save or revert this draft before selecting another object.")
                                        : qsTr("Selects an object for editing; live audio is unchanged.")
                onActivated: index => root.selectionRequested(textAt(index))
            }
        }

        StateBadge {
            Layout.alignment: Qt.AlignVCenter
            Layout.minimumWidth: 88
            stateKind: root.stateText.toLowerCase()
            stateText: root.stateText
        }

        RowLayout {
            Layout.alignment: Qt.AlignVCenter
            Layout.preferredWidth: 200
            spacing: Theme.space2

            AppButton {
                id: revertButton

                objectName: root.objectKey + "-revert"
                visible: root.modified
                enabled: visible
                Accessible.ignored: !visible
                text: qsTr("Revert")
                Accessible.name: qsTr("Revert %1 draft").arg(root.objectTitle)
                onClicked: root.revertRequested()
            }

            AppButton {
                id: saveButton

                objectName: root.objectKey + "-save"
                visible: root.modified
                enabled: visible
                Accessible.ignored: !visible
                variant: "primary"
                text: root.readOnly ? qsTr("Save as") : qsTr("Save")
                Accessible.name: root.readOnly
                                 ? qsTr("Save %1 as a new object").arg(root.objectTitle)
                                 : qsTr("Save %1").arg(root.objectTitle)
                onClicked: root.readOnly ? root.openSaveAs() : root.saveRequested()
            }

            Item {
                Layout.fillWidth: true
            }

            IconButton {
                id: actionsButton

                objectName: root.objectKey + "-actions"
                iconName: "dots-three-vertical"
                accessibleName: qsTr("%1 actions").arg(root.objectTitle)
                onClicked: actionsMenu.open()

                Menu {
                    id: actionsMenu

                    MenuItem {
                        text: qsTr("Save as…")
                        enabled: root.options.length > 0
                        hoverEnabled: true
                        onTriggered: root.openSaveAs()

                        HoverHandler {
                            cursorShape: parent.enabled
                                         ? Qt.PointingHandCursor
                                         : Qt.ForbiddenCursor
                        }
                    }

                    MenuItem {
                        text: qsTr("Revert draft")
                        enabled: root.modified
                        hoverEnabled: true
                        onTriggered: root.revertRequested()

                        HoverHandler {
                            cursorShape: parent.enabled
                                         ? Qt.PointingHandCursor
                                         : Qt.ForbiddenCursor
                        }
                    }
                }
            }
        }
    }

    Dialog {
        id: saveAsDialog

        parent: Overlay.overlay
        anchors.centerIn: parent
        width: 360
        modal: true
        title: qsTr("Save %1 as").arg(root.objectTitle)
        standardButtons: Dialog.Save | Dialog.Cancel
        onOpened: {
            saveAsName.forceActiveFocus()
            saveAsName.selectAll()
        }
        onClosed: {
            if (root.returnFocusItem)
                root.returnFocusItem.forceActiveFocus()
            root.returnFocusItem = null
        }
        onAccepted: root.saveAsRequested(saveAsName.text.trim())

        contentItem: ColumnLayout {
            spacing: 8

            Label {
                Layout.fillWidth: true
                text: qsTr("This creates an independent user object for the current output.")
                color: Theme.textSecondary
                wrapMode: Text.WordWrap
            }

            TextField {
                id: saveAsName

                objectName: root.objectKey + "-save-as-name"
                Layout.fillWidth: true
                placeholderText: qsTr("Name")
                Accessible.name: qsTr("%1 name").arg(root.objectTitle)
                onTextChanged: {
                    const button = saveAsDialog.standardButton(Dialog.Save)
                    if (button)
                        button.enabled = text.trim().length > 0
                }
            }
        }
    }
}
