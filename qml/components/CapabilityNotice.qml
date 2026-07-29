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

    readonly property color stateColor: statusCode === "ready" ? Theme.success
                                               : statusCode === "connecting" ? Theme.accent
                                               : statusCode === "partial" ? Theme.modified
                                               : Theme.error

    implicitHeight: content.implicitHeight + 16
    radius: Theme.radiusSmall
    color: Qt.rgba(stateColor.r, stateColor.g, stateColor.b, 0.08)
    border.color: Qt.rgba(stateColor.r, stateColor.g, stateColor.b, 0.55)

    RowLayout {
        id: content

        anchors.fill: parent
        anchors.margins: 8
        spacing: 10

        Rectangle {
            Layout.preferredWidth: 8
            Layout.preferredHeight: 8
            radius: 4
            color: root.stateColor
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2

            Label {
                Layout.fillWidth: true
                text: root.title
                color: Theme.textPrimary
                font.pixelSize: 12
                font.weight: Font.DemiBold
            }

            Label {
                Layout.fillWidth: true
                text: root.detail
                color: Theme.textSecondary
                font.pixelSize: 11
                wrapMode: Text.Wrap
            }
        }

        Button {
            text: qsTr("Retry")
            flat: true
            Accessible.name: qsTr("Retry live device connection")
            onClicked: root.retryRequested()
        }
    }
}
