import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Item {
    id: root

    property string stateText
    readonly property string normalizedState: stateText.toLowerCase()
    readonly property color stateColor: normalizedState.indexOf("modified") >= 0
                                          ? Theme.modified
                                          : normalizedState.indexOf("saved") >= 0
                                            ? Theme.success
                                            : normalizedState.indexOf("applying") >= 0
                                              ? Theme.accent
                                              : normalizedState.indexOf("error") >= 0
                                                || normalizedState.indexOf("not applied") >= 0
                                                ? Theme.error
                                                : Theme.textSecondary

    implicitWidth: badgeRow.implicitWidth
    implicitHeight: 24
    Accessible.role: Accessible.StaticText
    Accessible.name: stateText

    RowLayout {
        id: badgeRow

        anchors.verticalCenter: parent.verticalCenter
        spacing: Theme.space2

        Rectangle {
            Layout.preferredWidth: 8
            Layout.preferredHeight: 8
            radius: 4
            color: root.stateColor
        }

        Label {
            text: root.stateText
            color: root.stateColor
            font.pixelSize: Theme.fontCaption
            font.weight: Font.DemiBold
        }
    }
}
