import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Rectangle {
    id: root

    property string title
    property string value
    property string detail
    property string stateText
    property string stateKind: "ready"
    property string actionText
    signal activated

    implicitHeight: 170
    radius: Theme.radiusMedium
    color: Theme.surface
    border.width: 1
    border.color: Theme.separator
    Accessible.role: Accessible.Grouping
    Accessible.name: root.title
    Accessible.description: root.value + ". " + root.detail

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.space4
        spacing: Theme.space2

        RowLayout {
            Layout.fillWidth: true

            Label {
                Layout.fillWidth: true
                text: root.title
                color: Theme.textSecondary
                font.pixelSize: Theme.fontCaption
            }

            StateBadge {
                visible: root.stateText.length > 0
                stateKind: root.stateKind
                stateText: root.stateText
            }
        }

        Label {
            Layout.fillWidth: true
            text: root.value
            color: Theme.textPrimary
            font.pixelSize: 20
            font.weight: Font.DemiBold
            elide: Text.ElideRight
        }

        Label {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.minimumHeight: 32
            text: root.detail
            color: Theme.textSecondary
            font.pixelSize: Theme.fontCaption
            wrapMode: Text.Wrap
            elide: Text.ElideRight
        }

        AppButton {
            visible: root.actionText.length > 0
            enabled: visible
            Accessible.ignored: !visible
            variant: "ghost"
            text: root.actionText
            onClicked: root.activated()
        }
    }
}
