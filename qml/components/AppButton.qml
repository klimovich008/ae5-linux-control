import QtQuick
import QtQuick.Controls
import io.github.klimovich008.ae5control

Button {
    id: root

    property string variant: "secondary"
    property string blockedReason
    property string tooltipText

    implicitWidth: Math.max(72, contentItem.implicitWidth + leftPadding + rightPadding)
    implicitHeight: Theme.controlHeight
    leftPadding: Theme.space3
    rightPadding: Theme.space3
    topPadding: 0
    bottomPadding: 0
    hoverEnabled: true
    focusPolicy: Qt.StrongFocus

    ToolTip.visible: (hovered || activeFocus)
                     && ((!enabled && blockedReason.length > 0)
                         || tooltipText.length > 0)
    ToolTip.text: !enabled && blockedReason.length > 0
                  ? tooltipText.length > 0
                    ? tooltipText + "\n" + blockedReason
                    : blockedReason
                  : tooltipText

    background: Rectangle {
        radius: Theme.radiusSmall
        color: {
            if (!root.enabled && root.checked)
                return Theme.accentSubtle
            if (!root.enabled)
                return Theme.surfaceRaised
            if (root.variant === "primary")
                return root.down ? Theme.accentPressed
                                  : root.hovered ? Theme.accentHover : Theme.accent
            if (root.variant === "danger")
                return root.hovered ? Theme.errorSubtle : Theme.surface
            if (root.checked)
                return Theme.accentSubtle
            if (root.variant === "ghost")
                return root.hovered ? Theme.surfaceRaised : "transparent"
            return root.hovered ? Theme.surfaceRaised : Theme.surface
        }
        border.width: root.visualFocus ? 2 : 1
        border.color: root.visualFocus
                      ? Theme.focus
                      : root.checked ? Theme.accent
                      : root.variant === "primary" ? Theme.accent
                      : root.variant === "danger" ? Theme.error
                      : Theme.separatorStrong

        Behavior on color {
            ColorAnimation { duration: Theme.durationFast }
        }
    }

    contentItem: Label {
        text: root.text
        color: !root.enabled && root.checked
               ? Theme.textPrimary
               : !root.enabled
               ? Theme.textDisabled
               : root.variant === "primary" ? Theme.textOnAccent
               : root.variant === "danger" ? Theme.error
               : root.checked ? Theme.textPrimary : Theme.textSecondary
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        font.pixelSize: Theme.fontLabel
        font.weight: root.variant === "primary" || root.checked
                     ? Font.DemiBold : Font.Normal
        elide: Text.ElideRight
    }

    HoverHandler {
        enabled: root.enabled
        cursorShape: Qt.PointingHandCursor
    }
}
