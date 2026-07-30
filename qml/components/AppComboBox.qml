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

    delegate: ItemDelegate {
        id: option

        required property int index
        required property var modelData

        width: root.width
        implicitHeight: Theme.controlHeight
        leftPadding: Theme.space3
        rightPadding: Theme.space3
        topPadding: 0
        bottomPadding: 0
        text: modelData
        highlighted: root.highlightedIndex === index
        hoverEnabled: true

        background: Rectangle {
            radius: Theme.radiusSmall
            color: option.highlighted || option.hovered
                   ? Theme.accentSubtle : "transparent"
        }

        contentItem: Label {
            text: option.text
            color: option.enabled ? Theme.textPrimary : Theme.textDisabled
            verticalAlignment: Text.AlignVCenter
            font.pixelSize: Theme.fontBody
            elide: Text.ElideRight
        }

        HoverHandler {
            cursorShape: option.enabled
                         ? Qt.PointingHandCursor : Qt.ForbiddenCursor
        }
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

    popup: Popup {
        y: root.height + Theme.space1
        width: root.width
        implicitHeight: Math.min(contentItem.implicitHeight
                                 + topPadding + bottomPadding, 280)
        topPadding: Theme.space1
        bottomPadding: Theme.space1
        leftPadding: Theme.space1
        rightPadding: Theme.space1

        contentItem: ListView {
            clip: true
            implicitHeight: contentHeight
            model: root.popup.visible ? root.delegateModel : null
            currentIndex: root.highlightedIndex
            ScrollIndicator.vertical: ScrollIndicator {}
        }

        background: Rectangle {
            color: Theme.surface
            radius: Theme.radiusSmall
            border.width: 1
            border.color: Theme.separatorStrong
        }
    }

    HoverHandler {
        cursorShape: root.enabled ? Qt.PointingHandCursor : Qt.ForbiddenCursor
    }
}
