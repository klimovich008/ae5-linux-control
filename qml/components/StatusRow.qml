import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Item {
    id: root

    property string title
    property string detail
    property string value
    property string statusText
    property string statusKind: "ready"
    property bool showSeparator: true
    default property alias trailingContent: trailing.data

    implicitHeight: Math.max(58, rowLayout.implicitHeight + Theme.space3 * 2)
    Accessible.role: Accessible.StaticText
    Accessible.name: root.title
    Accessible.description: root.detail.length > 0
                            ? root.detail + " " + root.value
                            : root.value

    RowLayout {
        id: rowLayout

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.space3

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Theme.space1

            Label {
                Layout.fillWidth: true
                text: root.title
                color: Theme.textPrimary
                font.pixelSize: Theme.fontBody
            }

            Label {
                visible: root.detail.length > 0
                Layout.fillWidth: true
                text: root.detail
                color: Theme.textSecondary
                font.pixelSize: Theme.fontCaption
                wrapMode: Text.Wrap
            }
        }

        Label {
            visible: root.value.length > 0
            Layout.maximumWidth: 260
            text: root.value
            color: Theme.textPrimary
            font.pixelSize: Theme.fontLabel
            font.weight: Font.DemiBold
            horizontalAlignment: Text.AlignRight
            elide: Text.ElideRight
        }

        StateBadge {
            visible: root.statusText.length > 0
            stateKind: root.statusKind
            stateText: root.statusText
        }

        RowLayout {
            id: trailing

            visible: children.length > 0
            spacing: Theme.space2
        }
    }

    Rectangle {
        visible: root.showSeparator
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: 1
        color: Theme.separator
    }
}
