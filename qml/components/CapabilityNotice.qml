import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Rectangle {
    id: root

    property string statusCode: "connecting"
    property string title
    property string detail
    signal retryRequested

    readonly property bool actionable: statusCode !== "ready"
                                       && statusCode !== "connecting"
    readonly property bool quiet: statusCode === "ready"
    readonly property string actionText: statusCode === "no-device"
                                         ? qsTr("Scan again")
                                         : statusCode === "daemon-unavailable"
                                           ? qsTr("Reconnect")
                                           : qsTr("Refresh")
    readonly property string actionDescription: statusCode === "no-device"
                                                ? qsTr("Scan again for a compatible AE-5")
                                                : statusCode === "daemon-unavailable"
                                                  ? qsTr("Reconnect to the ae5d user service")
                                                  : qsTr("Refresh live device state")
    readonly property color stateColor: statusCode === "ready" ? Theme.success
                                               : statusCode === "connecting" ? Theme.accent
                                               : statusCode === "partial" ? Theme.modified
                                               : Theme.error

    implicitHeight: root.quiet ? Theme.controlHeightLarge
                               : content.implicitHeight + Theme.space4
    radius: Theme.radiusSmall
    color: root.quiet
           ? "transparent"
           : statusCode === "partial" ? Theme.modifiedSubtle
           : statusCode === "connecting" ? Theme.accentSubtle
                                         : Theme.errorSubtle
    border.width: root.quiet ? 0 : 1
    border.color: root.stateColor
    Accessible.role: statusCode === "ready" || statusCode === "connecting"
                     ? Accessible.StatusBar : Accessible.AlertMessage
    Accessible.name: title
    Accessible.description: detail

    RowLayout {
        id: content

        anchors.fill: parent
        anchors.leftMargin: root.quiet ? 0 : Theme.space2
        anchors.rightMargin: root.quiet ? 0 : Theme.space2
        anchors.topMargin: root.quiet ? 0 : Theme.space2
        anchors.bottomMargin: root.quiet ? 0 : Theme.space2
        spacing: Theme.space3

        Rectangle {
            visible: !root.quiet
            Layout.preferredWidth: 8
            Layout.preferredHeight: 8
            radius: 4
            color: root.stateColor
        }

        Label {
            visible: root.quiet
            Layout.fillWidth: true
            text: root.title + "  ·  " + root.detail
            color: Theme.textSecondary
            font.pixelSize: Theme.fontCaption
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
        }

        ColumnLayout {
            visible: !root.quiet
            Layout.fillWidth: true
            spacing: Theme.space1

            Label {
                Layout.fillWidth: true
                text: root.title
                color: Theme.textPrimary
                font.pixelSize: Theme.fontLabel
                font.weight: Font.DemiBold
            }

            Label {
                Layout.fillWidth: true
                text: root.detail
                color: Theme.textSecondary
                font.pixelSize: Theme.fontCaption
                wrapMode: Text.Wrap
            }
        }

        AppButton {
            visible: root.actionable
            enabled: visible
            Accessible.ignored: !visible
            text: root.actionText
            variant: "secondary"
            Accessible.name: root.actionDescription
            onClicked: root.retryRequested()
        }
    }
}
