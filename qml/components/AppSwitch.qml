import QtQuick
import QtQuick.Controls
import io.github.klimovich008.ae5control

Switch {
    id: root

    property string blockedReason

    implicitWidth: 48
    implicitHeight: Theme.controlHeight
    leftPadding: 0
    rightPadding: 0
    topPadding: 0
    bottomPadding: 0
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    ToolTip.visible: (hovered || activeFocus) && !enabled
                     && blockedReason.length > 0
    ToolTip.text: blockedReason

    indicator: Rectangle {
        implicitWidth: 44
        implicitHeight: 24
        x: (root.width - width) / 2
        y: (root.height - height) / 2
        radius: height / 2
        color: !root.enabled
               ? Theme.surfaceSunken
               : root.checked ? Theme.accent : Theme.surfaceSunken
        border.width: root.visualFocus ? 2 : 1
        border.color: root.visualFocus
                      ? Theme.focus
                      : root.checked && root.enabled ? Theme.accent : Theme.separatorStrong

        Rectangle {
            width: 18
            height: 18
            radius: 9
            x: root.checked ? parent.width - width - 3 : 3
            anchors.verticalCenter: parent.verticalCenter
            color: root.enabled
                   ? root.checked ? Theme.textOnAccent : Theme.textSecondary
                   : Theme.textDisabled

            Behavior on x {
                NumberAnimation {
                    duration: Theme.durationFast
                    easing.type: Easing.OutCubic
                }
            }
        }

        Behavior on color {
            ColorAnimation { duration: Theme.durationFast }
        }
    }

    contentItem: Item {}

    HoverHandler {
        enabled: root.enabled
        cursorShape: Qt.PointingHandCursor
    }
}
