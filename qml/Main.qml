import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "components"
import "pages"

ApplicationWindow {
    id: root

    function qaWindowDimension(part, fallback) {
        for (let index = 0; index < Qt.application.arguments.length; ++index) {
            const argument = Qt.application.arguments[index]
            if (!argument.startsWith("--qa-window="))
                continue
            const dimensions = argument.substring(12).split("x")
            if (dimensions.length !== 2)
                return fallback
            const parsed = Number(dimensions[part])
            return Number.isFinite(parsed) ? parsed : fallback
        }
        return fallback
    }

    function initialPage() {
        const validPages = ["overview", "sound", "equalizer", "playback",
                            "recording", "mixer", "lighting", "device",
                            "settings"]
        for (let index = 0; index < Qt.application.arguments.length; ++index) {
            const argument = Qt.application.arguments[index]
            if (!argument.startsWith("--qa-page="))
                continue
            const requested = argument.substring(10)
            return validPages.indexOf(requested) >= 0 ? requested : "sound"
        }
        return "sound"
    }

    function pageIndex(key) {
        switch (key) {
        case "overview": return 0
        case "sound": return 1
        case "equalizer": return 2
        case "playback": return 3
        case "recording": return 4
        case "mixer": return 5
        case "lighting": return 6
        case "device": return 7
        case "settings": return 8
        default: return 1
        }
    }

    width: qaWindowDimension(0, 1280)
    height: qaWindowDimension(1, 800)
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
    palette.alternateBase: Theme.surfaceRaised
    palette.brightText: Theme.textPrimary
    palette.dark: Theme.surfaceSunken
    // Qt's default delegates consume these colors for hover as well as
    // selection. Keep both readable in dark and light themes.
    palette.highlight: Theme.accentSubtle
    palette.highlightedText: Theme.textPrimary
    palette.light: Theme.surface
    palette.link: Theme.accent
    palette.mid: Theme.separatorStrong
    palette.midlight: Theme.separator
    palette.placeholderText: Theme.textDisabled
    palette.toolTipBase: Theme.surfaceRaised
    palette.toolTipText: Theme.textPrimary

    readonly property bool compact: width < Theme.compactBreakpoint
    readonly property bool wide: width >= Theme.wideBreakpoint
    readonly property bool effectsModified: appState.effectsState === "Modified"
    readonly property bool eqModified: appState.eqState === "Modified"
    readonly property bool qaFocusAuditRequested:
        Qt.application.arguments.indexOf("--qa-focus-audit") >= 0
    readonly property bool qaStateSmokeRequested:
        Qt.application.arguments.indexOf("--qa-state-smoke") >= 0
    property string activePage: initialPage()
    property bool closeConfirmed: false
    property var closeReturnFocusItem: null

    function focusFirstUnsavedAction() {
        if (effectsSaveButton.visible)
            effectsSaveButton.forceActiveFocus()
        else if (eqSaveButton.visible)
            eqSaveButton.forceActiveFocus()
        else
            cancelCloseButton.forceActiveFocus()
    }

    function finishCloseIfClean() {
        if (appState.unsavedCount === 0) {
            closeConfirmed = true
            unsavedDialog.close()
            root.close()
        } else {
            Qt.callLater(root.focusFirstUnsavedAction)
        }
    }

    function saveBeforeClose(objectName) {
        const readOnly = objectName === "effects"
                         ? appState.effectsReadOnly : appState.eqReadOnly
        if (readOnly) {
            closeReturnFocusItem = null
            unsavedDialog.close()
            root.activePage = "sound"
            soundPage.reviewObject(objectName, true)
            return
        }

        if (objectName === "effects")
            appState.saveEffectsDraft()
        else
            appState.saveEqDraft()
        finishCloseIfClean()
    }

    function discardAndClose() {
        closeConfirmed = true
        unsavedDialog.close()
        root.close()
    }

    function collectTabStops(item, result) {
        if (!item || item.visible === false || item.enabled === false)
            return
        if (item.activeFocusOnTab === true)
            result.push(item)
        if (!item.children)
            return
        for (let index = 0; index < item.children.length; ++index)
            collectTabStops(item.children[index], result)
    }

    function expectedQaFocusOrder() {
        let order = [
            "nav-overview", "nav-sound", "nav-equalizer", "nav-playback",
            "nav-recording", "nav-mixer", "nav-lighting", "nav-device",
            "nav-settings"
        ]
        if (appState.qaScenario === "both-modified")
            order.push("eq-revert", "eq-save", "eq-actions")
        else
            order.push("eq-selector", "eq-actions")
        order.push(
            "eq-band-0", "eq-band-1", "eq-band-2", "eq-band-3", "eq-band-4",
            "eq-band-5", "eq-band-6", "eq-band-7", "eq-band-8", "eq-band-9",
        )
        if (appState.qaScenario === "both-modified")
            order.push("effects-revert", "effects-save", "effects-actions")
        else
            order.push("effects-selector", "effects-actions")
        order.push(
            "direct-mode",
            "effect-surround-switch", "effect-surround-level",
            "effect-crystalizer-switch",
            "effect-crystalizer-level",
            "effect-bass-switch", "effect-bass-level",
            "effect-smart-volume-switch",
            "effect-smart-volume-level",
            "effect-dialog-switch", "effect-dialog-level",
            "output-speakers", "output-headphones", "output-digital",
            "master-mute", "master-volume"
        )
        if (appState.qaScenario === "both-modified")
            order.push("unsaved-review")
        return order
    }

    function runQaFocusAudit() {
        if (!appState.qaMode
                || (appState.qaScenario !== "ready"
                    && appState.qaScenario !== "both-modified")) {
            console.error("AE5_QML_FOCUS_AUDIT requires --qa-state=ready or both-modified")
            Qt.exit(2)
            return
        }

        const expected = expectedQaFocusOrder()
        let discovered = []
        collectTabStops(root.contentItem, discovered)
        const discoveredNames = discovered.map(item => item.objectName)
        let failures = []

        for (let index = 0; index < discoveredNames.length; ++index) {
            if (discoveredNames[index].length === 0)
                failures.push("unnamed tab stop at discovered index " + index)
            if (discoveredNames.indexOf(discoveredNames[index]) !== index)
                failures.push("duplicate tab stop " + discoveredNames[index])
        }
        for (let index = 0; index < expected.length; ++index) {
            if (discoveredNames.indexOf(expected[index]) < 0)
                failures.push("missing tab stop " + expected[index])
        }
        for (let index = 0; index < discoveredNames.length; ++index) {
            if (expected.indexOf(discoveredNames[index]) < 0)
                failures.push("unexpected tab stop " + discoveredNames[index])
        }

        const first = discovered.find(item => item.objectName === expected[0])
        let actual = []
        if (!first) {
            failures.push("cannot focus the first expected tab stop")
        } else {
            first.forceActiveFocus(Qt.TabFocusReason)
            let current = first
            for (let index = 0; index < expected.length + 2; ++index) {
                if (!current || actual.indexOf(current.objectName) >= 0)
                    break
                actual.push(current.objectName)
                current = current.nextItemInFocusChain(true)
            }
        }

        if (actual.join("|") !== expected.join("|")) {
            failures.push("focus chain differs: expected=" + expected.join(",")
                          + " actual=" + actual.join(","))
        }

        if (failures.length > 0) {
            console.error("AE5_QML_FOCUS_AUDIT result=failed " + failures.join("; "))
            Qt.exit(1)
        } else {
            console.log("AE5_QML_FOCUS_AUDIT result=passed count=" + expected.length
                        + " order=" + actual.join(","))
            Qt.exit(0)
        }
    }

    onClosing: function(closeEvent) {
        if (!closeConfirmed && appState.unsavedCount > 0) {
            closeEvent.accepted = false
            if (!unsavedDialog.visible) {
                closeReturnFocusItem = root.activeFocusItem
                unsavedDialog.open()
            }
        }
    }

    AppState {
        id: appState
    }

    Component.onCompleted: Qt.callLater(function() {
        appState.refreshFromDaemon()
    })

    Timer {
        interval: 5000
        repeat: true
        running: !appState.qaMode && appState.softwareEqState !== "applying"
        onTriggered: appState.refreshFromDaemon()
    }

    Timer {
        interval: 120
        running: root.qaFocusAuditRequested
        repeat: false
        onTriggered: root.runQaFocusAudit()
    }

    Timer {
        interval: 120
        running: root.qaStateSmokeRequested && !root.qaFocusAuditRequested
        repeat: false
        onTriggered: {
            if (!appState.qaMode) {
                console.error("AE5_QML_STATE_SMOKE requires --qa-state")
                Qt.exit(2)
            } else {
                console.log("AE5_QML_STATE_SMOKE result=rendered state="
                            + appState.qaScenario + " status=" + appState.statusCode
                            + " page=" + root.activePage)
                Qt.exit(0)
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            NavigationSidebar {
                Layout.preferredWidth: root.compact
                                       ? Theme.sidebarWidthCompact
                                       : root.wide ? Theme.sidebarWidthWide
                                                   : Theme.sidebarWidth
                Layout.fillHeight: true
                compact: root.compact
                currentKey: root.activePage
                onPageRequested: key => root.activePage = key
            }

            StackLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: root.pageIndex(root.activePage)

                OverviewPage {
                    appState: appState
                    compact: root.compact
                    onNavigateRequested: page => root.activePage = page
                }

                SoundPage {
                    id: soundPage

                    appState: appState
                    compact: root.compact
                }

                EqualizerPage {
                    id: equalizerPage

                    appState: appState
                    compact: root.compact
                }

                PlaybackPage {
                    appState: appState
                    compact: root.compact
                    onNavigateRequested: page => root.activePage = page
                }

                RecordingPage {
                    appState: appState
                    compact: root.compact
                }

                MixerPage {
                    appState: appState
                    compact: root.compact
                }

                LightingPage {
                    appState: appState
                    compact: root.compact
                }

                DevicePage {
                    appState: appState
                    compact: root.compact
                }

                SettingsPage {
                    appState: appState
                    compact: root.compact
                }
            }
        }

        HardwareFaceplate {
            Layout.fillWidth: true
            Layout.preferredHeight: root.compact
                                    ? Theme.faceplateHeightCompact
                                    : Theme.faceplateHeight
            appState: appState
            compact: root.compact
            wide: root.wide
            onReviewRequested: {
                root.activePage = "sound"
                Qt.callLater(soundPage.reviewUnsaved)
            }
        }
    }

    Shortcut {
        sequence: StandardKey.Save
        onActivated: {
            if (root.activePage === "equalizer")
                equalizerPage.saveCurrent(false)
            else
                soundPage.saveFocusedObject(false)
        }
    }

    Shortcut {
        sequence: "Ctrl+Shift+S"
        onActivated: {
            if (root.activePage === "equalizer")
                equalizerPage.saveCurrent(true)
            else
                soundPage.saveFocusedObject(true)
        }
    }

    Dialog {
        id: unsavedDialog

        parent: Overlay.overlay
        anchors.centerIn: parent
        width: Math.min(520, root.width - 48)
        modal: true
        closePolicy: Popup.CloseOnEscape
        title: qsTr("Save changes before closing?")

        onOpened: Qt.callLater(root.focusFirstUnsavedAction)
        onClosed: {
            if (!root.closeConfirmed && root.closeReturnFocusItem)
                root.closeReturnFocusItem.forceActiveFocus()
            root.closeReturnFocusItem = null
        }

        contentItem: ColumnLayout {
            spacing: 14
            Accessible.role: Accessible.Grouping
            Accessible.name: unsavedDialog.title
            Accessible.description: qsTr("Effects profiles and EQ presets are saved independently.")

            Label {
                Layout.fillWidth: true
                text: qsTr("Choose each modified object you want to save. Discard closes without changing saved profiles or live audio.")
                color: Theme.textSecondary
                wrapMode: Text.WordWrap
            }

            RowLayout {
                visible: root.effectsModified
                Layout.fillWidth: true
                spacing: 12
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Modified Effects profile")
                Accessible.description: appState.effectsDetail

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Label {
                        text: qsTr("Effects · %1").arg(appState.effectsProfile)
                        color: Theme.textPrimary
                        font.weight: Font.DemiBold
                    }

                    Label {
                        Layout.fillWidth: true
                        text: appState.effectsDetail
                        color: Theme.textSecondary
                        wrapMode: Text.WordWrap
                    }
                }

                AppButton {
                    id: effectsSaveButton

                    Layout.minimumHeight: 40
                    variant: "primary"
                    text: appState.effectsReadOnly ? qsTr("Review Save as…") : qsTr("Save Effects")
                    Accessible.name: appState.effectsReadOnly
                                     ? qsTr("Review Effects profile and save as a new object")
                                     : qsTr("Save Effects profile %1").arg(appState.effectsProfile)
                    onClicked: root.saveBeforeClose("effects")
                }
            }

            RowLayout {
                visible: root.eqModified
                Layout.fillWidth: true
                spacing: 12
                Accessible.role: Accessible.Grouping
                Accessible.name: qsTr("Modified EQ preset")
                Accessible.description: appState.eqDetail

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2

                    Label {
                        text: qsTr("Equalizer · %1").arg(appState.eqPreset)
                        color: Theme.textPrimary
                        font.weight: Font.DemiBold
                    }

                    Label {
                        Layout.fillWidth: true
                        text: appState.eqDetail
                        color: Theme.textSecondary
                        wrapMode: Text.WordWrap
                    }
                }

                AppButton {
                    id: eqSaveButton

                    Layout.minimumHeight: 40
                    variant: "primary"
                    text: appState.eqReadOnly ? qsTr("Review Save as…") : qsTr("Save EQ")
                    Accessible.name: appState.eqReadOnly
                                     ? qsTr("Review EQ preset and save as a new object")
                                     : qsTr("Save EQ preset %1").arg(appState.eqPreset)
                    onClicked: root.saveBeforeClose("eq")
                }
            }
        }

        footer: DialogButtonBox {
            AppButton {
                text: qsTr("Discard and close")
                variant: "danger"
                DialogButtonBox.buttonRole: DialogButtonBox.DestructiveRole
                Accessible.description: qsTr("Discard local drafts and close without changing saved profiles or live audio")
                onClicked: root.discardAndClose()
            }

            AppButton {
                id: cancelCloseButton

                text: qsTr("Cancel")
                DialogButtonBox.buttonRole: DialogButtonBox.RejectRole
                Accessible.description: qsTr("Return to AE5 Control without closing")
                onClicked: unsavedDialog.reject()
            }
        }
    }
}
