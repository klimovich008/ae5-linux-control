import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Rectangle {
    id: root

    property string title
    property string detail
    property string statusText
    property string statusKind: "ready"
    property bool elevated: false
    default property alias panelContent: panelBody.data

    implicitHeight: panelLayout.implicitHeight + Theme.space4 * 2
    radius: Theme.radiusMedium
    color: elevated ? Theme.surfaceRaised : Theme.surface
    border.width: 1
    border.color: Theme.separator
    Accessible.role: Accessible.Grouping
    Accessible.name: root.title
    Accessible.description: root.detail

    ColumnLayout {
        id: panelLayout

        anchors.fill: parent
        anchors.margins: Theme.space4
        spacing: Theme.space3

        RowLayout {
            Layout.fillWidth: true
            spacing: Theme.space3

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Theme.space1

                Label {
                    Layout.fillWidth: true
                    text: root.title
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontSectionTitle
                    font.weight: Font.DemiBold
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

            StateBadge {
                visible: root.statusText.length > 0
                stateKind: root.statusKind
                stateText: root.statusText
            }
        }

        Rectangle {
            visible: panelBody.children.length > 0
            Layout.fillWidth: true
            Layout.preferredHeight: 1
            color: Theme.separator
        }

        ColumnLayout {
            id: panelBody

            Layout.fillWidth: true
            spacing: 0
        }
    }
}
