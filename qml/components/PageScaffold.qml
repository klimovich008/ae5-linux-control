import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Rectangle {
    id: root

    property var appState
    property bool compact: false
    property string pageTitle
    property string pageDescription
    property bool showDeviceNotice: true
    default property alias pageContent: contentBody.data
    signal retryRequested

    readonly property int pageGutter: compact ? 20 : Theme.space6

    color: Theme.background
    Accessible.role: Accessible.Pane
    Accessible.name: root.pageTitle
    Accessible.description: root.pageDescription

    ScrollView {
        id: pageScroll

        anchors.fill: parent
        clip: true
        contentWidth: availableWidth
        bottomPadding: Theme.space5
        ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
        ScrollBar.vertical.policy: ScrollBar.AsNeeded
        ScrollBar.vertical.active: root.compact || ScrollBar.vertical.pressed

        ColumnLayout {
            width: Math.min(pageScroll.availableWidth, Theme.contentMaxWidth)
            x: Math.max(0, (pageScroll.availableWidth - width) / 2)
            spacing: Theme.space3

            Item {
                Layout.preferredHeight: Theme.space2
            }

            ColumnLayout {
                Layout.fillWidth: true
                Layout.leftMargin: root.pageGutter
                Layout.rightMargin: root.pageGutter
                spacing: Theme.space1

                Label {
                    text: root.pageTitle
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontPageTitle
                    font.weight: Font.DemiBold
                }

                Label {
                    Layout.fillWidth: true
                    text: root.pageDescription
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontLabel
                    wrapMode: Text.Wrap
                }
            }

            CapabilityNotice {
                visible: root.showDeviceNotice
                         && root.appState
                         && root.appState.statusCode !== "ready"
                Layout.fillWidth: true
                Layout.leftMargin: root.pageGutter
                Layout.rightMargin: root.pageGutter
                statusCode: root.appState ? root.appState.statusCode : "daemon-unavailable"
                title: root.appState ? root.appState.deviceStatus : qsTr("Daemon unavailable")
                detail: root.appState ? root.appState.statusDetail : qsTr("No device state is available.")
                onRetryRequested: root.retryRequested()
            }

            ColumnLayout {
                id: contentBody

                Layout.fillWidth: true
                Layout.leftMargin: root.pageGutter
                Layout.rightMargin: root.pageGutter
                spacing: Theme.space3
            }

            Item {
                Layout.preferredHeight: Theme.space3
            }
        }
    }
}
