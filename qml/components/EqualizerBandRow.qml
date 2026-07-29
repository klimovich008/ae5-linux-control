import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control

Item {
    id: root

    property var appState
    property int bandIndex: 0
    property string frequency
    property bool editingEnabled: true

    implicitHeight: Theme.controlHeightLarge

    function syncValue() {
        if (!root.appState
                || root.bandIndex < 0
                || root.bandIndex >= root.appState.eqBandGainsTenthsDb.length)
            return
        const parsed = Number(root.appState.eqBandGainsTenthsDb[root.bandIndex])
        if (Number.isFinite(parsed) && !bandSlider.pressed)
            bandSlider.value = parsed / 10
    }

    RowLayout {
        anchors.fill: parent
        spacing: Theme.space3

        Label {
            Layout.preferredWidth: 52
            text: root.frequency
            color: Theme.textPrimary
            font.pixelSize: Theme.fontLabel
        }

        AppSlider {
            id: bandSlider

            objectName: "eq-detail-band-" + root.bandIndex
            Layout.fillWidth: true
            from: -12
            to: 12
            stepSize: 0.1
            enabled: root.editingEnabled
            blockedReason: qsTr("The profile library is unavailable.")
            Accessible.name: qsTr("%1 equalizer band").arg(root.frequency)
            Accessible.description: qsTr("%1 decibels").arg(value.toFixed(1))
            onMoved: root.appState.updateEqBand(root.bandIndex,
                                                Math.round(value * 10))
            Component.onCompleted: root.syncValue()
        }

        Label {
            Layout.preferredWidth: 58
            text: (bandSlider.value >= 0 ? "+" : "")
                  + bandSlider.value.toFixed(1) + " dB"
            color: Theme.textPrimary
            font.pixelSize: Theme.fontCaption
            horizontalAlignment: Text.AlignRight
        }
    }

    Connections {
        target: root.appState

        function onEqBandGainsTenthsDbChanged() {
            root.syncValue()
        }

        function onEqSelectionRevisionChanged() {
            root.syncValue()
        }
    }
}
