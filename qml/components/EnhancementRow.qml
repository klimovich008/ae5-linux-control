import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Item {
    id: root

    property var appState
    property string controlKey
    property string title
    property string unit: "%"
    property int initialValue
    property bool initiallyEnabled: true
    property bool controlsEnabled: true
    property bool available: true
    property bool editingEnabled: true
    property string unavailableReason: qsTr("This control is unavailable in the selected Effects profile.")
    property string leftPole
    property string rightPole
    readonly property bool interactive: available && controlsEnabled && editingEnabled
    readonly property string blockedReason: !available
                                             ? qsTr("%1 is not present in this profile.").arg(title)
                                             : !editingEnabled
                                               ? unavailableReason
                                               : !controlsEnabled
                                                 ? qsTr("%1 is bypassed while Direct Mode is active.").arg(title)
                                                 : ""

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
            Layout.preferredWidth: 32
            Layout.preferredHeight: 32
            display: AbstractButton.IconOnly
            icon.name: "help-about-symbolic"
            icon.color: Theme.textSecondary
            Accessible.name: qsTr("About %1").arg(root.title)
            Accessible.description: root.interactive
                                    ? qsTr("%1 support depends on the active AE-5 capability path.").arg(root.title)
                                    : root.blockedReason
            ToolTip.visible: hovered || activeFocus
            ToolTip.text: Accessible.description
        }

        Switch {
            id: enabledSwitch

            checked: root.initiallyEnabled
            enabled: root.interactive
            Accessible.name: qsTr("Enable %1").arg(root.title)
            Accessible.description: enabled ? qsTr("Toggle %1").arg(root.title)
                                            : root.blockedReason
            ToolTip.visible: (hovered || activeFocus) && !enabled
            ToolTip.text: Accessible.description
            onClicked: root.appState.updateEffectsDraft(
                           root.controlKey, checked, Math.round(level.value))
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
                                    : root.blockedReason
            ToolTip.visible: (hovered || activeFocus) && !enabled
            ToolTip.text: Accessible.description
            onMoved: root.appState.updateEffectsDraft(
                         root.controlKey, enabledSwitch.checked, Math.round(value))
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
