import QtQuick
import QtQuick.Controls
import io.github.klimovich008.ae5control

ToolButton {
    id: root

    required property string iconName
    required property string accessibleName
    property string blockedReason
    property string tooltipText
    property string variant: "ghost"

    implicitWidth: Theme.iconButtonSize
    implicitHeight: Theme.iconButtonSize
    display: AbstractButton.IconOnly
    icon.source: Theme.iconSource(iconName)
    icon.width: 18
    icon.height: 18
    icon.color: !enabled
                ? Theme.textDisabled
                : variant === "danger" ? Theme.error : Theme.textSecondary
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus
    Accessible.name: accessibleName
    Accessible.description: blockedReason.length > 0 ? blockedReason : accessibleName

    ToolTip.visible: hovered || activeFocus
    ToolTip.text: enabled || blockedReason.length === 0
                  ? tooltipText.length > 0 ? tooltipText : accessibleName
                  : blockedReason

    background: Rectangle {
        radius: Theme.radiusSmall
        color: !root.enabled
               ? "transparent"
               : root.down ? Theme.surfaceSunken
               : root.hovered ? Theme.surfaceRaised : "transparent"
        border.width: root.visualFocus ? 2 : 0
        border.color: Theme.focus

        Behavior on color {
            ColorAnimation { duration: Theme.durationFast }
        }
    }

    HoverHandler {
        cursorShape: root.enabled ? Qt.PointingHandCursor : Qt.ForbiddenCursor
    }
}
