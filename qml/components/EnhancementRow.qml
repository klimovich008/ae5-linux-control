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

    implicitHeight: Theme.controlHeightLarge
    Accessible.role: Accessible.Grouping
    Accessible.name: root.title
    Accessible.description: root.interactive
                            ? qsTr("%1 support depends on the active AE-5 capability path.").arg(root.title)
                            : root.blockedReason

    RowLayout {
        anchors.fill: parent
        spacing: Theme.space2

        Label {
            Layout.preferredWidth: 104
            text: root.title
            color: root.available && root.controlsEnabled ? Theme.textPrimary : Theme.disabled
            font.pixelSize: Theme.fontBody
            elide: Text.ElideRight
        }

        Item {
            Layout.preferredWidth: Theme.iconButtonSize
            Layout.preferredHeight: Theme.iconButtonSize
            Accessible.ignored: true

            readonly property string helpText: root.interactive
                                                ? qsTr("%1 support depends on the active AE-5 capability path.").arg(root.title)
                                                : root.blockedReason

            ToolButton {
                anchors.centerIn: parent
                width: 18
                height: 18
                padding: 0
                display: AbstractButton.IconOnly
                icon.source: Theme.iconSource("info")
                icon.width: 18
                icon.height: 18
                icon.color: root.interactive
                            ? Theme.textSecondary : Theme.textDisabled
                background: Item {}
                enabled: false
                opacity: root.interactive ? 0.8 : 0.55
                Accessible.ignored: true
            }

            ToolTip.visible: helpHover.hovered
            ToolTip.text: helpText

            HoverHandler {
                id: helpHover
                cursorShape: Qt.WhatsThisCursor
            }
        }

        AppSwitch {
            id: enabledSwitch

            objectName: "effect-" + root.controlKey + "-switch"
            checked: root.initiallyEnabled
            enabled: root.interactive
            blockedReason: root.blockedReason
            Accessible.name: qsTr("Enable %1").arg(root.title)
            Accessible.description: enabled ? qsTr("Toggle %1").arg(root.title)
                                            : root.blockedReason
            onClicked: root.appState.updateEffectsDraft(
                           root.controlKey, checked, Math.round(level.value))
        }

        Item {
            id: trackColumn

            Layout.fillWidth: true
            Layout.preferredHeight: Theme.controlHeightLarge

            AppSlider {
                id: level

                objectName: "effect-" + root.controlKey + "-level"
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                from: 0
                to: 100
                value: root.initialValue
                enabled: root.interactive && enabledSwitch.checked
                blockedReason: root.blockedReason
                Accessible.name: root.title
                Accessible.description: enabled
                                        ? qsTr("%1 percent").arg(Math.round(value))
                                        : root.blockedReason
                onMoved: root.appState.updateEffectsDraft(
                             root.controlKey, enabledSwitch.checked, Math.round(value))
            }

            Label {
                visible: root.leftPole.length > 0
                anchors.left: parent.left
                anchors.top: parent.verticalCenter
                anchors.topMargin: Theme.space2
                text: root.leftPole
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMicro
            }

            Label {
                visible: root.rightPole.length > 0
                anchors.right: parent.right
                anchors.top: parent.verticalCenter
                anchors.topMargin: Theme.space2
                text: root.rightPole
                color: Theme.textSecondary
                font.pixelSize: Theme.fontMicro
            }
        }

        Label {
            Layout.preferredWidth: 48
            text: root.available
                  ? enabledSwitch.checked ? Math.round(level.value) + root.unit : qsTr("Off")
                  : qsTr("N/A")
            color: root.available && root.controlsEnabled ? Theme.textPrimary : Theme.disabled
            horizontalAlignment: Text.AlignRight
            font.pixelSize: Theme.fontLabel
        }
    }
}
