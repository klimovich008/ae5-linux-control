import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Item {
    id: root

    property string objectTitle
    property string selectorLabel
    property string currentName
    property string stateText
    property string statusDetail
    property bool readOnly: true
    property var options: []
    readonly property bool modified: stateText === "Modified"

    signal saveRequested
    signal saveAsRequested(string name)
    signal revertRequested
    signal selectionRequested(string name)

    implicitHeight: headerLayout.implicitHeight

    function openSaveAs() {
        saveAsName.text = currentName
        saveAsDialog.open()
    }

    RowLayout {
        id: headerLayout

        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 12

        ColumnLayout {
            Layout.fillWidth: true
            Layout.minimumWidth: 170
            spacing: 3

            Label {
                text: root.objectTitle
                color: Theme.textPrimary
                font.pixelSize: 18
                font.weight: Font.DemiBold
            }

            Label {
                id: detailLabel

                Layout.fillWidth: true
                text: root.statusDetail
                color: Theme.textSecondary
                font.pixelSize: 12
                elide: Text.ElideRight
                ToolTip.visible: truncated && detailHover.hovered
                ToolTip.text: text

                HoverHandler {
                    id: detailHover
                }
            }
        }

        ColumnLayout {
            Layout.preferredWidth: 190
            spacing: 4

            Label {
                text: root.selectorLabel
                color: Theme.textSecondary
                font.pixelSize: 12
            }

            ComboBox {
                Layout.fillWidth: true
                model: root.options
                currentIndex: {
                    for (let index = 0; index < root.options.length; ++index) {
                        if (root.options[index] === root.currentName)
                            return index
                    }
                    return -1
                }
                enabled: root.options.length > 0 && !root.modified
                Accessible.name: root.selectorLabel
                Accessible.description: root.modified
                                        ? qsTr("Save or revert this draft before selecting another object.")
                                        : qsTr("Selects an object for editing; live audio is unchanged.")
                ToolTip.visible: hovered && !enabled
                ToolTip.text: Accessible.description
                onActivated: index => root.selectionRequested(textAt(index))
            }
        }

        RowLayout {
            Layout.alignment: Qt.AlignBottom
            Layout.preferredWidth: 78
            Layout.preferredHeight: 37
            spacing: 7

            Rectangle {
                Layout.preferredWidth: 7
                Layout.preferredHeight: 7
                radius: 4
                color: root.modified ? Theme.modified
                                     : root.stateText === "Saved" ? Theme.success
                                                                  : Theme.accent
            }

            Label {
                text: root.stateText
                color: root.modified ? Theme.modified
                                     : root.stateText === "Saved" ? Theme.success
                                                                  : Theme.textSecondary
                font.pixelSize: 13
            }
        }

        Button {
            visible: root.modified
            enabled: visible
            Accessible.ignored: !visible
            Layout.alignment: Qt.AlignBottom
            text: qsTr("Revert")
            Accessible.name: qsTr("Revert %1 draft").arg(root.objectTitle)
            onClicked: root.revertRequested()
        }

        Button {
            id: saveButton

            visible: root.modified
            enabled: visible
            Accessible.ignored: !visible
            Layout.alignment: Qt.AlignBottom
            text: root.readOnly ? qsTr("Save as") : qsTr("Save")
            Accessible.name: root.readOnly
                             ? qsTr("Save %1 as a new object").arg(root.objectTitle)
                             : qsTr("Save %1").arg(root.objectTitle)
            onClicked: root.readOnly ? root.openSaveAs() : root.saveRequested()

            background: Rectangle {
                radius: Theme.radiusSmall
                color: saveButton.down ? Qt.darker(Theme.accent, 1.25)
                                       : saveButton.hovered ? Qt.lighter(Theme.accent, 1.1)
                                                            : Theme.accent
            }

            contentItem: Label {
                text: saveButton.text
                color: Theme.background
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
                font.weight: Font.DemiBold
            }
        }

        ToolButton {
            Layout.alignment: Qt.AlignBottom
            display: AbstractButton.IconOnly
            icon.name: "view-more-symbolic"
            icon.color: Theme.textSecondary
            Accessible.name: qsTr("%1 actions").arg(root.objectTitle)
            ToolTip.visible: hovered
            ToolTip.text: Accessible.name
            onClicked: actionsMenu.open()

            Menu {
                id: actionsMenu

                MenuItem {
                    text: qsTr("Save as…")
                    enabled: root.options.length > 0
                    onTriggered: root.openSaveAs()
                }

                MenuItem {
                    text: qsTr("Revert draft")
                    enabled: root.modified
                    onTriggered: root.revertRequested()
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
