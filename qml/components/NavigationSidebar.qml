import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Rectangle {
    id: root

    property bool compact: false

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
                { label: qsTr("Overview"), icon: "go-home-symbolic" },
                { label: qsTr("Sound"), icon: "audio-speakers-symbolic" },
                { label: qsTr("Equalizer"), icon: "multimedia-volume-control-symbolic" },
                { label: qsTr("Playback"), icon: "media-playback-start-symbolic" },
                { label: qsTr("Recording"), icon: "audio-input-microphone-symbolic" },
                { label: qsTr("Mixer"), icon: "audio-volume-high-symbolic" }
            ]

            delegate: ItemDelegate {
                id: navItem

                required property var modelData
                readonly property bool selected: modelData.label === qsTr("Sound")

                Layout.fillWidth: true
                Layout.preferredHeight: 44
                leftPadding: root.compact ? 0 : 16
                rightPadding: root.compact ? 0 : 12
                hoverEnabled: true
                enabled: selected
                focusPolicy: selected ? Qt.TabFocus : Qt.NoFocus
                Accessible.name: modelData.label
                Accessible.description: selected
                                        ? qsTr("Current page")
                                        : qsTr("Coming in a later milestone")

                background: Rectangle {
                    radius: Theme.radiusSmall
                    color: navItem.selected
                           ? Qt.rgba(Theme.accent.r, Theme.accent.g, Theme.accent.b, 0.1)
                                            : navItem.hovered ? Theme.surface : "transparent"
                    border.width: navItem.visualFocus ? 3 : 0
                    border.color: Theme.focus

                    Rectangle {
                        visible: navItem.selected
                        width: 3
                        height: parent.height
                        color: Theme.accent
                    }
                }

                contentItem: RowLayout {
                    spacing: 12

                    ToolButton {
                        Layout.preferredWidth: root.compact ? 72 : 24
                        display: AbstractButton.IconOnly
                        icon.name: navItem.modelData.icon
                        icon.color: navItem.selected ? Theme.accent : Theme.textSecondary
                        background: Item {}
                        enabled: false
                        Accessible.ignored: true
                    }

                    Label {
                        visible: !root.compact
                        Layout.fillWidth: true
                        text: navItem.modelData.label
                        color: navItem.selected ? Theme.textPrimary : Theme.textSecondary
                        font.pixelSize: 14
                        font.weight: navItem.selected ? Font.DemiBold : Font.Normal
                    }
                }

                ToolTip.visible: root.compact && (hovered || activeFocus)
                ToolTip.text: modelData.label
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
                { label: qsTr("Lighting"), icon: "preferences-desktop-theme-symbolic" },
                { label: qsTr("Device"), icon: "drive-harddisk-symbolic" },
                { label: qsTr("Settings"), icon: "preferences-system-symbolic" }
            ]

            delegate: ItemDelegate {
                id: utilityItem

                required property var modelData

                Layout.fillWidth: true
                Layout.preferredHeight: 44
                leftPadding: root.compact ? 0 : 16
                rightPadding: root.compact ? 0 : 12
                hoverEnabled: true
                enabled: false
                Accessible.name: modelData.label
                Accessible.description: qsTr("Coming in a later milestone")

                background: Item {}

                contentItem: RowLayout {
                    spacing: 12

                    ToolButton {
                        Layout.preferredWidth: root.compact ? 72 : 24
                        display: AbstractButton.IconOnly
                        icon.name: utilityItem.modelData.icon
                        icon.color: Theme.disabled
                        background: Item {}
                        enabled: false
                        Accessible.ignored: true
                    }

                    Label {
                        visible: !root.compact
                        Layout.fillWidth: true
                        text: utilityItem.modelData.label
                        color: Theme.disabled
                        font.pixelSize: 14
                    }
                }

                ToolTip.visible: root.compact && (hovered || activeFocus)
                ToolTip.text: qsTr("%1 — coming in a later milestone").arg(modelData.label)
            }
        }
    }
}
