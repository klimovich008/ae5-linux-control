pragma Singleton

import QtQuick

QtObject {
    readonly property bool light: Qt.application.arguments.indexOf("--light") >= 0

    readonly property color background: light ? "#F4F8FA" : "#071725"
    readonly property color sidebar: light ? "#E8F0F4" : "#081B2A"
    readonly property color surface: light ? "#FFFFFF" : "#0C2131"
    readonly property color surfaceRaised: light ? "#E5F1F6" : "#102A3C"
    readonly property color faceplate: light ? "#E2ECF2" : "#06131F"
    readonly property color separator: light ? "#A9BEC9" : "#294354"
    readonly property color textPrimary: light ? "#10212C" : "#F1F6F9"
    readonly property color textSecondary: light ? "#405967" : "#9FB1BC"
    readonly property color accent: light ? "#006F86" : "#00C7E6"
    readonly property color focus: light ? "#6941C6" : "#9B73F4"
    readonly property color success: light ? "#187A32" : "#55C96A"
    readonly property color modified: light ? "#8A5200" : "#F3A72A"
    readonly property color error: light ? "#B42335" : "#F05F6D"
    readonly property color disabled: light ? "#6E818D" : "#607582"

    readonly property int radiusSmall: 4
    readonly property int radiusMedium: 6
}
