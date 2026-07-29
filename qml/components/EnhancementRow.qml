import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Item {
    id: root

    property var appState
    property string title
    property string unit: "%"
    property int initialValue
    property bool initiallyEnabled: true
    property bool controlsEnabled: true
    property bool available: true
    property bool editingEnabled: false
    property string unavailableReason: qsTr("Editing this Effects profile is not connected yet.")
    property string leftPole
    property string rightPole
    readonly property bool interactive: available && controlsEnabled && editingEnabled

    implicitHeight: 36

    RowLayout {
        anchors.fill: parent
        spacing: 12

        Label {
            Layout.preferredWidth: 118
            text: root.title
            color: root.available && root.controlsEnabled ? Theme.textPrimary : Theme.disabled
            font.pixelSize: 14
        }

        ToolButton {
            Layout.preferredWidth: 28
            display: AbstractButton.IconOnly
            icon.name: "help-about-symbolic"
            icon.color: Theme.textSecondary
            Accessible.name: qsTr("About %1").arg(root.title)
            ToolTip.visible: hovered
            ToolTip.text: root.available
                          ? root.editingEnabled
                            ? qsTr("%1 support depends on the active AE-5 capability path.").arg(root.title)
                            : root.unavailableReason
                          : qsTr("%1 is not present in this profile.").arg(root.title)
        }

        Switch {
            id: enabledSwitch

            checked: root.initiallyEnabled
            enabled: root.interactive
            Accessible.name: qsTr("Enable %1").arg(root.title)
            Accessible.description: enabled ? qsTr("Toggle %1").arg(root.title)
                                            : root.unavailableReason
            ToolTip.visible: hovered && !enabled
            ToolTip.text: Accessible.description
            onToggled: root.appState.markEffectsModified()
        }

        Label {
            visible: root.leftPole.length > 0
            text: root.leftPole
            color: Theme.textSecondary
            font.pixelSize: 11
        }

        Slider {
            id: level

            Layout.fillWidth: true
            from: 0
            to: 100
            value: root.initialValue
            enabled: root.interactive && enabledSwitch.checked
            focusPolicy: Qt.StrongFocus
            Accessible.name: root.title
            Accessible.description: enabled
                                    ? qsTr("%1 percent").arg(Math.round(value))
                                    : root.unavailableReason
            ToolTip.visible: hovered && !enabled
            ToolTip.text: Accessible.description
            onMoved: root.appState.markEffectsModified()
        }

        Label {
            visible: root.rightPole.length > 0
            text: root.rightPole
            color: Theme.textSecondary
            font.pixelSize: 11
        }

        Label {
            Layout.preferredWidth: 54
            text: root.available
                  ? enabledSwitch.checked ? Math.round(level.value) + root.unit : qsTr("Off")
                  : qsTr("N/A")
            color: root.available && root.controlsEnabled ? Theme.textPrimary : Theme.disabled
            horizontalAlignment: Text.AlignRight
            font.pixelSize: 13
            font.family: "monospace"
        }
    }
}
