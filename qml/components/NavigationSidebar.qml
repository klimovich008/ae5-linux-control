import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Rectangle {
    id: root

    property bool compact: false
    property string currentKey: "sound"
    signal pageRequested(string key)

    // Phosphor icons share a 256 px canvas, but their drawn bounds vary enough
    // to look inconsistent when every icon is given the same source size.
    // Normalize the visible glyphs while keeping one fixed alignment slot.
    function opticalIconSize(name) {
        switch (name) {
        case "squares-four":
            return 26
        case "speaker-high":
        case "microphone":
            return 21
        case "faders-horizontal":
        case "circuitry":
            return 25
        case "lightbulb":
            return 22
        default:
            return 23
        }
    }

    color: Theme.sidebar

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: 22
        anchors.bottomMargin: 16
        spacing: 2

        Label {
            Layout.leftMargin: root.compact ? 0 : 18
            Layout.alignment: root.compact ? Qt.AlignHCenter : Qt.AlignLeft
            Layout.bottomMargin: 20
            text: root.compact ? "AE5" : "AE5 CONTROL"
            color: Theme.textPrimary
            font.pixelSize: root.compact ? 14 : 15
            font.bold: true
            font.letterSpacing: 1.2
        }

        Repeater {
            model: [
                { key: "overview", label: qsTr("Overview"), icon: "squares-four" },
                { key: "sound", label: qsTr("Sound"), icon: "speaker-high" },
                { key: "equalizer", label: qsTr("Equalizer"), icon: "sliders" },
                { key: "playback", label: qsTr("Playback"), icon: "play-circle" },
                { key: "recording", label: qsTr("Recording"), icon: "microphone" },
                { key: "mixer", label: qsTr("Mixer"), icon: "faders-horizontal" }
            ]

            delegate: ItemDelegate {
                id: navItem

                required property var modelData
                readonly property bool selected: modelData.key === root.currentKey

                objectName: "nav-" + modelData.key
                Layout.fillWidth: true
                Layout.leftMargin: root.compact ? 0 : Theme.space2
                Layout.rightMargin: root.compact ? 0 : Theme.space2
                Layout.preferredHeight: Theme.navItemHeight
                leftPadding: root.compact ? 0 : Theme.space2
                rightPadding: root.compact ? 0 : Theme.space2
                topPadding: 0
                bottomPadding: 0
                hoverEnabled: true
                focusPolicy: Qt.TabFocus
                Accessible.name: modelData.label
                Accessible.description: selected
                                        ? qsTr("Current page")
                                        : qsTr("Open %1").arg(modelData.label)
                onClicked: root.pageRequested(modelData.key)

                background: Rectangle {
                    radius: Theme.radiusSmall
                    color: navItem.selected
                           ? Theme.accentSubtle
                           : navItem.hovered ? Theme.surface : "transparent"
                    border.width: navItem.visualFocus ? 2 : 0
                    border.color: Theme.focus
                    clip: true

                    Rectangle {
                        visible: navItem.selected
                        width: 3
                        height: parent.height
                        color: Theme.accent
                    }
                }

                contentItem: RowLayout {
                    spacing: Theme.space3

                    ToolButton {
                        Layout.preferredWidth: root.compact
                                               ? Theme.sidebarWidthCompact
                                               : Theme.space5
                        Layout.preferredHeight: Theme.navItemHeight
                        Layout.alignment: Qt.AlignVCenter
                        padding: 0
                        display: AbstractButton.IconOnly
                        icon.source: Theme.iconSource(navItem.modelData.icon)
                        icon.width: root.opticalIconSize(navItem.modelData.icon)
                        icon.height: root.opticalIconSize(navItem.modelData.icon)
                        icon.color: navItem.selected
                                    ? Theme.accent
                                    : navItem.hovered
                                      ? Theme.textPrimary : Theme.textDisabled
                        background: Item {}
                        enabled: false
                        opacity: 1
                        focusPolicy: Qt.NoFocus
                        hoverEnabled: false
                        Accessible.ignored: true
                    }

                    Label {
                        visible: !root.compact
                        Layout.fillWidth: true
                        Layout.preferredHeight: Theme.navItemHeight
                        Layout.alignment: Qt.AlignVCenter
                        text: navItem.modelData.label
                        color: navItem.selected || navItem.hovered
                               ? Theme.textPrimary : Theme.textDisabled
                        font.pixelSize: Theme.fontBody
                        font.weight: navItem.selected ? Font.DemiBold : Font.Normal
                        verticalAlignment: Text.AlignVCenter
                    }
                }

                ToolTip.visible: root.compact && (hovered || activeFocus)
                ToolTip.text: modelData.label

                HoverHandler {
                    enabled: true
                    cursorShape: Qt.PointingHandCursor
                }
            }
        }

        Item {
            Layout.fillHeight: true
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.leftMargin: Theme.space2
            Layout.rightMargin: Theme.space2
            Layout.preferredHeight: 1
            Layout.bottomMargin: 8
            color: Theme.separator
        }

        Repeater {
            model: [
                { key: "lighting", label: qsTr("Lighting"), icon: "lightbulb" },
                { key: "device", label: qsTr("Device"), icon: "circuitry" },
                { key: "settings", label: qsTr("Settings"), icon: "gear" }
            ]

            delegate: ItemDelegate {
                id: utilityItem

                required property var modelData
                readonly property bool selected: modelData.key === root.currentKey

                objectName: "nav-" + modelData.key
                Layout.fillWidth: true
                Layout.leftMargin: root.compact ? 0 : Theme.space2
                Layout.rightMargin: root.compact ? 0 : Theme.space2
                Layout.preferredHeight: Theme.navItemHeight
                leftPadding: root.compact ? 0 : Theme.space2
                rightPadding: root.compact ? 0 : Theme.space2
                topPadding: 0
                bottomPadding: 0
                hoverEnabled: true
                focusPolicy: Qt.TabFocus
                Accessible.name: modelData.label
                Accessible.description: selected
                                        ? qsTr("Current page")
                                        : qsTr("Open %1").arg(modelData.label)
                onClicked: root.pageRequested(modelData.key)

                background: Rectangle {
                    radius: Theme.radiusSmall
                    color: utilityItem.selected
                           ? Theme.accentSubtle
                           : utilityItem.hovered ? Theme.surface : "transparent"
                    border.width: utilityItem.visualFocus ? 2 : 0
                    border.color: Theme.focus
                    clip: true

                    Rectangle {
                        visible: utilityItem.selected
                        width: 3
                        height: parent.height
                        color: Theme.accent
                    }
                }

                contentItem: RowLayout {
                    spacing: Theme.space3

                    ToolButton {
                        Layout.preferredWidth: root.compact
                                               ? Theme.sidebarWidthCompact
                                               : Theme.space5
                        Layout.preferredHeight: Theme.navItemHeight
                        Layout.alignment: Qt.AlignVCenter
                        padding: 0
                        display: AbstractButton.IconOnly
                        icon.source: Theme.iconSource(utilityItem.modelData.icon)
                        icon.width: root.opticalIconSize(utilityItem.modelData.icon)
                        icon.height: root.opticalIconSize(utilityItem.modelData.icon)
                        icon.color: utilityItem.selected
                                    ? Theme.accent
                                    : utilityItem.hovered
                                      ? Theme.textPrimary : Theme.textDisabled
                        background: Item {}
                        enabled: false
                        opacity: 1
                        focusPolicy: Qt.NoFocus
                        hoverEnabled: false
                        Accessible.ignored: true
                    }

                    Label {
                        visible: !root.compact
                        Layout.fillWidth: true
                        Layout.preferredHeight: Theme.navItemHeight
                        Layout.alignment: Qt.AlignVCenter
                        text: utilityItem.modelData.label
                        color: utilityItem.selected || utilityItem.hovered
                               ? Theme.textPrimary : Theme.textDisabled
                        font.pixelSize: Theme.fontBody
                        font.weight: utilityItem.selected ? Font.DemiBold : Font.Normal
                        verticalAlignment: Text.AlignVCenter
                    }
                }

                ToolTip.visible: root.compact && (hovered || activeFocus)
                ToolTip.text: modelData.label

                HoverHandler {
                    enabled: true
                    cursorShape: Qt.PointingHandCursor
                }
            }
        }
    }

    Rectangle {
        anchors.top: parent.top
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        width: 1
        color: Theme.separator
    }
}
