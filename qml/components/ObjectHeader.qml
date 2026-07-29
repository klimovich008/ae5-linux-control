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
    property string cleanSubtitle
    property string modifiedSubtitle
    property var options: []
    readonly property bool modified: stateText === "Modified"

    signal saveRequested
    signal selectionRequested(string name)

    implicitHeight: headerLayout.implicitHeight

    RowLayout {
        id: headerLayout

        anchors.left: parent.left
        anchors.right: parent.right
        spacing: 16

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3

            Label {
                text: root.objectTitle
                color: Theme.textPrimary
                font.pixelSize: 18
                font.weight: Font.DemiBold
            }

            Label {
                Layout.fillWidth: true
                text: root.modified ? root.modifiedSubtitle : root.cleanSubtitle
                color: Theme.textSecondary
                font.pixelSize: 12
                elide: Text.ElideRight
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
                enabled: root.options.length > 0
                Accessible.name: root.selectorLabel
                Accessible.description: qsTr("Selects an object for preview; it does not change live audio.")
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
            id: saveButton

            visible: root.modified
            enabled: visible
            Layout.alignment: Qt.AlignBottom
            text: qsTr("Save")
            Accessible.name: qsTr("Save %1").arg(root.objectTitle)
            Accessible.ignored: !visible
            onClicked: root.saveRequested()

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
        }
    }
}
