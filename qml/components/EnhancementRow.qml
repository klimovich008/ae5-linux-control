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

    implicitHeight: Math.max(Theme.controlHeightLarge, trackColumn.implicitHeight)

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

        IconButton {
            Layout.preferredWidth: Theme.iconButtonSize
            Layout.preferredHeight: Theme.iconButtonSize
            iconName: "info"
            accessibleName: qsTr("About %1").arg(root.title)
            tooltipText: root.interactive
                         ? qsTr("%1 support depends on the active AE-5 capability path.").arg(root.title)
                         : root.blockedReason
        }

        AppSwitch {
            id: enabledSwitch

            checked: root.initiallyEnabled
            enabled: root.interactive
            blockedReason: root.blockedReason
            Accessible.name: qsTr("Enable %1").arg(root.title)
            Accessible.description: enabled ? qsTr("Toggle %1").arg(root.title)
                                            : root.blockedReason
            onClicked: root.appState.updateEffectsDraft(
                           root.controlKey, checked, Math.round(level.value))
        }

        ColumnLayout {
            id: trackColumn

            Layout.fillWidth: true
            spacing: 0

            AppSlider {
                id: level

                Layout.fillWidth: true
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

            RowLayout {
                visible: root.leftPole.length > 0 || root.rightPole.length > 0
                Layout.fillWidth: true
                Layout.topMargin: -4

                Label {
                    text: root.leftPole
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMicro
                }

                Item {
                    Layout.fillWidth: true
                }

                Label {
                    text: root.rightPole
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontMicro
                }
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
