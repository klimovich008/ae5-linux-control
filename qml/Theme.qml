pragma Singleton

import QtQuick

QtObject {
    readonly property bool light: Qt.application.arguments.indexOf("--light") >= 0

    // Surfaces
    readonly property color background: light ? "#F4F8FA" : "#071725"
    readonly property color sidebar: light ? "#E8F0F4" : "#081B2A"
    readonly property color surface: light ? "#FFFFFF" : "#0F2839"
    readonly property color surfaceRaised: light ? "#EDF3F7" : "#163449"
    readonly property color surfaceSunken: light ? "#DDE7EE" : "#051019"
    readonly property color faceplate: light ? "#E2ECF2" : "#06131F"
    readonly property color separator: light ? "#C6D5DE" : "#294354"
    readonly property color separatorStrong: light ? "#9FB5C2" : "#3C5C72"

    // Text
    readonly property color textPrimary: light ? "#10212C" : "#F1F6F9"
    readonly property color textSecondary: light ? "#405967" : "#9FB1BC"
    readonly property color textDisabled: light ? "#778B97" : "#879CA9"
    readonly property color textOnAccent: light ? "#FFFFFF" : "#04141C"

    // Interaction
    readonly property color accent: light ? "#006F86" : "#139CC0"
    readonly property color accentHover: light ? "#00596C" : "#32B1D0"
    readonly property color accentPressed: light ? "#004353" : "#0E7F9C"
    readonly property color accentSubtle: light ? "#DCEFF4" : "#123F52"
    readonly property color focus: light ? "#6941C6" : "#9B73F4"

    // Status
    readonly property color success: light ? "#187A32" : "#55C96A"
    readonly property color successSubtle: light ? "#E2F4E6" : "#0E2A19"
    readonly property color modified: light ? "#8A5200" : "#F3A72A"
    readonly property color modifiedSubtle: light ? "#FBF0DC" : "#2E220B"
    readonly property color error: light ? "#B42335" : "#F05F6D"
    readonly property color errorSubtle: light ? "#FCE8EA" : "#2B1319"
    readonly property color disabled: textDisabled

    // Four-pixel spacing scale.
    readonly property int space1: 4
    readonly property int space2: 8
    readonly property int space3: 12
    readonly property int space4: 16
    readonly property int space5: 24
    readonly property int space6: 32

    readonly property int radiusSmall: 4
    readonly property int radiusMedium: 6
    readonly property int radiusPill: 999

    readonly property int fontPageTitle: 28
    readonly property int fontSectionTitle: 18
    readonly property int fontBody: 14
    readonly property int fontLabel: 13
    readonly property int fontCaption: 12
    readonly property int fontMicro: 11

    readonly property int controlHeight: 36
    readonly property int controlHeightLarge: 40
    readonly property int iconButtonSize: 36
    readonly property int navItemHeight: 44
    readonly property int faceplateHeight: 88
    readonly property int faceplateHeightCompact: 76
    readonly property int sidebarWidth: 208
    readonly property int sidebarWidthCompact: 72
    readonly property int sidebarWidthWide: 224
    readonly property int contentMaxWidth: 1280
    readonly property int compactBreakpoint: 1120
    readonly property int wideBreakpoint: 1440
    readonly property int effectsColumnsBreakpoint: 1000

    readonly property int durationFast: 120
    readonly property int durationNormal: 160

    function iconSource(name) {
        return "qrc:/qt/qml/io/github/klimovich008/ae5control/assets/icons/phosphor/"
                + name + ".svg"
    }
}
