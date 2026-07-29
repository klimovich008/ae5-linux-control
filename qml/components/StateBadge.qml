import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Item {
    id: root

    property string stateText
    property string stateKind: ""
    property int maximumBadgeWidth: 160
    readonly property string normalizedKind: stateKind.length > 0
                                             ? stateKind.toLowerCase().trim().replace(/\s+/g, "-")
                                             : stateText.toLowerCase().trim().replace(/\s+/g, "-")
    readonly property color stateColor: {
        switch (normalizedKind) {
        case "modified":
        case "bypassed":
        case "partial":
            return Theme.modified
        case "saved":
        case "current":
        case "active":
        case "ready":
            return Theme.success
        case "applying":
        case "loading":
        case "connecting":
            return Theme.accent
        case "error":
        case "unavailable":
        case "write-failed":
        case "not-applied":
            return Theme.error
        default:
            return Theme.textSecondary
        }
    }

    implicitWidth: Math.min(maximumBadgeWidth, badgeRow.implicitWidth)
    implicitHeight: 24
    clip: badgeRow.implicitWidth > width
    Accessible.role: Accessible.StaticText
    Accessible.name: stateText

    RowLayout {
        id: badgeRow

        anchors.fill: parent
        spacing: Theme.space2

        Rectangle {
            Layout.preferredWidth: 8
            Layout.preferredHeight: 8
            radius: 4
            color: root.stateColor
        }

        Label {
            Layout.fillWidth: true
            text: root.stateText
            color: root.stateColor
            font.pixelSize: Theme.fontCaption
            font.weight: Font.DemiBold
            elide: Text.ElideRight
            ToolTip.visible: truncated && stateHover.hovered
            ToolTip.text: text

            HoverHandler {
                id: stateHover
            }
        }
    }
}
