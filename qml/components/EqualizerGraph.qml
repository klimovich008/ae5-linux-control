import QtQuick
import QtQuick.Controls
import QtQuick.Shapes
import io.github.klimovich008.ae5control

Rectangle {
    id: root

    property var appState
    property bool editingEnabled: false
    property var gains: [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
    readonly property var frequencies: ["31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"]
    readonly property real graphLeft: 42
    readonly property real graphRight: width - 22
    readonly property real graphTop: 18
    readonly property real graphBottom: height - 34

    color: Theme.surface
    radius: Theme.radiusMedium
    border.color: Theme.separator
    implicitHeight: 232
    Accessible.role: Accessible.Chart
    Accessible.name: qsTr("Ten-band equalizer")
    Accessible.description: qsTr("Ten accessible sliders from 31 hertz to 16 kilohertz, with gains from minus 12 to plus 12 decibels.")

    function xFor(index) {
        return graphLeft + index * (graphRight - graphLeft) / 9
    }

    function yFor(gain) {
        return graphTop + (12 - gain) * (graphBottom - graphTop) / 24
    }

    function setGain(index, value) {
        const updated = gains.slice()
        updated[index] = Math.round(value)
        gains = updated
        appState.updateEqBand(index, updated[index] * 10)
    }

    function loadSelectedPreset() {
        if (!appState)
            return
        const loaded = []
        for (let index = 0; index < appState.eqBandGainsTenthsDb.length; ++index)
            loaded.push(Number(appState.eqBandGainsTenthsDb[index]) / 10)
        if (loaded.length === 10)
            gains = loaded
    }

    Component.onCompleted: loadSelectedPreset()

    Connections {
        target: root.appState

        function onEqSelectionRevisionChanged() {
            root.loadSelectedPreset()
        }
    }

    Repeater {
        model: [-12, -6, 0, 6, 12]

        delegate: Rectangle {
            required property int modelData

            x: root.graphLeft
            y: root.yFor(modelData)
            width: root.graphRight - root.graphLeft
            height: modelData === 0 ? 2 : 1
            color: modelData === 0
                   ? Qt.rgba(Theme.textSecondary.r, Theme.textSecondary.g,
                             Theme.textSecondary.b, 0.48)
                   : Qt.rgba(Theme.separator.r, Theme.separator.g, Theme.separator.b, 0.75)

            Label {
                anchors.right: parent.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                text: modelData > 0 ? "+" + modelData : modelData
                color: Theme.textSecondary
                font.pixelSize: 10
            }
        }
    }

    Repeater {
        model: 10

        delegate: Rectangle {
            required property int index

            x: root.xFor(index)
            y: root.graphTop
            width: 1
            height: root.graphBottom - root.graphTop
            color: Qt.rgba(Theme.separator.r, Theme.separator.g, Theme.separator.b, 0.55)
        }
    }

    Shape {
        anchors.fill: parent
        layer.enabled: true
        layer.samples: 4

        ShapePath {
            strokeColor: Theme.accent
            strokeWidth: 2
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin

            startX: root.xFor(0)
            startY: root.yFor(root.gains[0])
            PathLine { x: root.xFor(1); y: root.yFor(root.gains[1]) }
            PathLine { x: root.xFor(2); y: root.yFor(root.gains[2]) }
            PathLine { x: root.xFor(3); y: root.yFor(root.gains[3]) }
            PathLine { x: root.xFor(4); y: root.yFor(root.gains[4]) }
            PathLine { x: root.xFor(5); y: root.yFor(root.gains[5]) }
            PathLine { x: root.xFor(6); y: root.yFor(root.gains[6]) }
            PathLine { x: root.xFor(7); y: root.yFor(root.gains[7]) }
            PathLine { x: root.xFor(8); y: root.yFor(root.gains[8]) }
            PathLine { x: root.xFor(9); y: root.yFor(root.gains[9]) }
        }
    }

    Repeater {
        model: 10

        delegate: Slider {
            id: band

            required property int index

            x: root.xFor(index) - 17
            y: root.graphTop
            width: 34
            height: root.graphBottom - root.graphTop
            from: -12
            to: 12
            stepSize: 1
            value: root.gains[index]
            orientation: Qt.Vertical
            enabled: root.editingEnabled && root.appState.eqEnabled
                     && !root.appState.directMode
            focusPolicy: Qt.StrongFocus
            wheelEnabled: activeFocus
            Accessible.name: qsTr("%1 hertz equalizer band").arg(root.frequencies[index])
            Accessible.description: enabled
                                    ? qsTr("%1 decibels; arrow keys adjust by 1 dB").arg(value.toFixed(0))
                                    : !root.appState.eqEnabled
                                      ? qsTr("Enable this EQ preset before editing its bands.")
                                      : root.appState.directMode
                                        ? qsTr("Equalizer editing is unavailable while Direct Mode is active.")
                                        : qsTr("Equalizer presets are unavailable until the profile catalog loads.")
            ToolTip.visible: (hovered || activeFocus) && !enabled
            ToolTip.text: Accessible.description

            background: Item {}

            handle: Rectangle {
                x: (band.width - width) / 2
                y: root.yFor(band.value) - root.graphTop - height / 2
                width: band.activeFocus ? 15 : 12
                height: width
                radius: width / 2
                color: Theme.surfaceRaised
                border.width: band.activeFocus ? 3 : 2
                border.color: band.activeFocus ? Theme.focus : Theme.accent

                Label {
                    visible: band.activeFocus
                    anchors.horizontalCenter: parent.horizontalCenter
                    anchors.bottom: parent.top
                    anchors.bottomMargin: 5
                    text: (band.value > 0 ? "+" : "") + band.value.toFixed(0) + " dB"
                    color: Theme.textPrimary
                    font.pixelSize: 10
                    padding: 4

                    background: Rectangle {
                        color: Theme.surfaceRaised
                        radius: Theme.radiusSmall
                        border.color: Theme.focus
                    }
                }
            }

            onMoved: root.setGain(index, value)

            Keys.onPressed: event => {
                if (event.key === Qt.Key_Home) {
                    value = 0
                } else if (event.key === Qt.Key_PageUp) {
                    value = Math.min(to, value + 1)
                } else if (event.key === Qt.Key_PageDown) {
                    value = Math.max(from, value - 1)
                } else {
                    return
                }
                root.setGain(index, value)
                event.accepted = true
            }
        }
    }

    Repeater {
        model: 10

        delegate: Label {
            required property int index

            x: root.xFor(index) - width / 2
            y: root.graphBottom + 9
            text: root.frequencies[index]
            color: Theme.textSecondary
            font.pixelSize: 10
        }
    }

    Label {
        anchors.right: parent.right
        anchors.rightMargin: 12
        anchors.top: parent.top
        anchors.topMargin: 7
        text: "dB"
        color: Theme.textSecondary
        font.pixelSize: 10
    }
}
