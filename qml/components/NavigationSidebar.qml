import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Rectangle {
    id: root

    property bool compact: false
    property string currentKey: "sound"

    color: Theme.sidebar

    ColumnLayout {
        anchors.fill: parent
        anchors.topMargin: 22
        anchors.bottomMargin: 16
        spacing: 2

        Label {
            Layout.leftMargin: root.compact ? 0 : 20
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

                Layout.fillWidth: true
                Layout.preferredHeight: Theme.navItemHeight
                leftPadding: root.compact ? 0 : Theme.space4
                rightPadding: root.compact ? 0 : Theme.space3
                hoverEnabled: true
                focusPolicy: selected ? Qt.TabFocus : Qt.NoFocus
                Accessible.name: modelData.label
                Accessible.description: selected
                                        ? qsTr("Current page")
                                        : qsTr("Coming in a later milestone")

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
                        display: AbstractButton.IconOnly
                        icon.source: Theme.iconSource(navItem.modelData.icon)
                        icon.width: 20
                        icon.height: 20
                        icon.color: navItem.selected ? Theme.accent : Theme.textDisabled
                        background: Item {}
                        focusPolicy: Qt.NoFocus
                        hoverEnabled: false
                        Accessible.ignored: true
                    }

                    Label {
                        visible: !root.compact
                        Layout.fillWidth: true
                        text: navItem.modelData.label
                        color: navItem.selected ? Theme.textPrimary : Theme.textDisabled
                        font.pixelSize: Theme.fontBody
                        font.weight: navItem.selected ? Font.DemiBold : Font.Normal
                    }
                }

                ToolTip.visible: root.compact && (hovered || activeFocus)
                ToolTip.text: selected
                              ? modelData.label
                              : qsTr("%1 — coming in a later milestone").arg(modelData.label)

                HoverHandler {
                    enabled: navItem.selected
                    cursorShape: Qt.PointingHandCursor
                }
            }
        }

        Item {
            Layout.fillHeight: true
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.leftMargin: 16
            Layout.rightMargin: 16
            Layout.preferredHeight: 1
            Layout.bottomMargin: 8
            color: Theme.separator
        }

        Repeater {
            model: [
                { label: qsTr("Lighting"), icon: "lightbulb" },
                { label: qsTr("Device"), icon: "circuitry" },
                { label: qsTr("Settings"), icon: "gear" }
            ]

            delegate: ItemDelegate {
                id: utilityItem

                required property var modelData

                Layout.fillWidth: true
                Layout.preferredHeight: Theme.navItemHeight
                leftPadding: root.compact ? 0 : Theme.space4
                rightPadding: root.compact ? 0 : Theme.space3
                hoverEnabled: true
                focusPolicy: Qt.NoFocus
                Accessible.name: modelData.label
                Accessible.description: qsTr("Coming in a later milestone")

                background: Rectangle {
                    color: utilityItem.hovered ? Theme.surface : "transparent"
                    radius: Theme.radiusSmall
                }

                contentItem: RowLayout {
                    spacing: Theme.space3

                    ToolButton {
                        Layout.preferredWidth: root.compact
                                               ? Theme.sidebarWidthCompact
                                               : Theme.space5
                        display: AbstractButton.IconOnly
                        icon.source: Theme.iconSource(utilityItem.modelData.icon)
                        icon.width: 20
                        icon.height: 20
                        icon.color: Theme.textDisabled
                        background: Item {}
                        focusPolicy: Qt.NoFocus
                        hoverEnabled: false
                        Accessible.ignored: true
                    }

                    Label {
                        visible: !root.compact
                        Layout.fillWidth: true
                        text: utilityItem.modelData.label
                        color: Theme.textDisabled
                        font.pixelSize: Theme.fontBody
                    }
                }

                ToolTip.visible: root.compact && (hovered || activeFocus)
                ToolTip.text: qsTr("%1 — coming in a later milestone").arg(modelData.label)
            }
        }
    }
}
