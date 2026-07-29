import QtQuick
import QtQuick.Controls
import io.github.klimovich008.ae5control

ComboBox {
    id: root

    property string blockedReason

    implicitHeight: Theme.controlHeightLarge
    leftPadding: Theme.space3
    rightPadding: 38
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    ToolTip.visible: (hovered || activeFocus) && !enabled
                     && blockedReason.length > 0
    ToolTip.text: blockedReason

    background: Rectangle {
        radius: Theme.radiusSmall
        color: !root.enabled
               ? Theme.surfaceSunken
               : root.pressed || root.popup.visible ? Theme.surfaceSunken
               : root.hovered ? Theme.surfaceRaised : Theme.surface
        border.width: root.visualFocus ? 2 : 1
        border.color: root.visualFocus
                      ? Theme.focus
                      : !root.enabled ? Theme.separator : Theme.separatorStrong

        Behavior on color {
            ColorAnimation { duration: Theme.durationFast }
        }
    }

    contentItem: Label {
        leftPadding: 0
        rightPadding: 0
        text: root.displayText
        color: root.enabled ? Theme.textPrimary : Theme.textSecondary
        verticalAlignment: Text.AlignVCenter
        font.pixelSize: Theme.fontBody
        elide: Text.ElideRight
    }

    indicator: ToolButton {
        x: root.width - width
        y: 0
        width: 38
        height: root.height
        display: AbstractButton.IconOnly
        icon.source: Theme.iconSource("caret-down")
        icon.width: 16
        icon.height: 16
        icon.color: root.enabled ? Theme.textSecondary : Theme.textDisabled
        background: Item {}
        enabled: false
        opacity: 1
        Accessible.ignored: true
    }

    HoverHandler {
        enabled: root.enabled
        cursorShape: Qt.PointingHandCursor
    }
}
