import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "components"
import "pages"

ApplicationWindow {
    id: root

    width: 1280
    height: 800
    minimumWidth: 1024
    minimumHeight: 680
    visible: true
    title: qsTr("AE5 Control")
    color: Theme.background
    palette.window: Theme.background
    palette.windowText: Theme.textPrimary
    palette.base: Theme.surface
    palette.text: Theme.textPrimary
    palette.button: Theme.surfaceRaised
    palette.buttonText: Theme.textPrimary
    palette.highlight: Theme.accent
    palette.highlightedText: Theme.background

    readonly property bool compact: width < 1120

    AppState {
        id: appState
    }

    Component.onCompleted: Qt.callLater(function() {
        appState.refreshFromDaemon()
    })

    Timer {
        interval: 5000
        repeat: true
        running: true
        onTriggered: appState.refreshFromDaemon()
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            NavigationSidebar {
                Layout.preferredWidth: root.compact ? 72 : 208
                Layout.fillHeight: true
                compact: root.compact
            }

            SoundPage {
                id: soundPage

                Layout.fillWidth: true
                Layout.fillHeight: true
                appState: appState
                compact: root.compact
            }
        }

        HardwareFaceplate {
            Layout.fillWidth: true
            Layout.preferredHeight: root.compact ? 76 : 88
            appState: appState
            compact: root.compact
            onReviewRequested: soundPage.reviewUnsaved()
        }
    }

    Shortcut {
        sequence: StandardKey.Save
        onActivated: soundPage.saveFocusedObject(false)
    }

    Shortcut {
        sequence: "Ctrl+Shift+S"
        onActivated: soundPage.saveFocusedObject(true)
    }
}
