import QtQuick
import QtQuick.Controls
import io.github.klimovich008.ae5control

Slider {
    id: root

    property string blockedReason

    implicitHeight: Theme.controlHeight
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    wheelEnabled: activeFocus

    ToolTip.visible: (hovered || activeFocus) && !enabled
                     && blockedReason.length > 0
    ToolTip.text: blockedReason

    background: Rectangle {
        x: root.leftPadding
        y: root.topPadding + root.availableHeight / 2 - height / 2
        implicitWidth: 180
        implicitHeight: 4
        width: root.availableWidth
        height: 4
        radius: 2
        color: Theme.surfaceSunken

        Rectangle {
            width: root.visualPosition * parent.width
            height: parent.height
            radius: parent.radius
            color: root.enabled ? Theme.accent : Theme.textDisabled
        }
    }

    handle: Rectangle {
        x: root.leftPadding + root.visualPosition * (root.availableWidth - width)
        y: root.topPadding + root.availableHeight / 2 - height / 2
        implicitWidth: root.hovered || root.visualFocus ? 18 : 16
        implicitHeight: implicitWidth
        radius: width / 2
        color: Theme.surface
        border.width: root.visualFocus ? 2 : 1
        border.color: root.visualFocus
                      ? Theme.focus
                      : root.enabled ? Theme.accent : Theme.textDisabled

        Behavior on implicitWidth {
            NumberAnimation { duration: Theme.durationFast }
        }
    }

    HoverHandler {
        enabled: root.enabled
        cursorShape: root.pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
    }
}
