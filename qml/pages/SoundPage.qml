import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import io.github.klimovich008.ae5control
import "../components"

Rectangle {
    id: root

    property var appState
    property bool compact: false
    readonly property int pageGutter: compact ? 20 : Theme.space6
    readonly property string noticeCode:
        root.appState.statusCode !== "ready"
        ? root.appState.statusCode
        : root.appState.profileCatalogStatus === "ready"
          ? "ready"
          : root.appState.profileCatalogStatus === "stale" ? "partial"
                                                            : "profile-unavailable"

    color: Theme.background
    Accessible.role: Accessible.Pane
    Accessible.name: qsTr("Sound")
    Accessible.description: root.appState.statusCode === "ready"
                            ? qsTr("Volume, mute, profile saving, guarded hardware Effects and checked software EQ are live. %1")
                              .arg(root.appState.profileCatalogDetail)
                            : root.appState.statusDetail

    function containsFocusItem(container) {
        let item = root.Window.window ? root.Window.window.activeFocusItem : null
        while (item) {
            if (item === container)
                return true
            item = item.parent
        }
        return false
    }

    function revealSection(section, header, openSaveAs) {
        const flickable = pageScroll.contentItem
        const position = section.mapToItem(contentColumn, 0, 0).y
        flickable.contentY = Math.max(
                    0, Math.min(position - 8,
                                contentColumn.implicitHeight - flickable.height))
        Qt.callLater(function() {
            if (!header.focusEditor())
                section.forceActiveFocus(Qt.TabFocusReason)
            if (openSaveAs)
                header.openSaveAs()
        })
    }

    function reviewObject(objectName, openSaveAs) {
        if (objectName === "effects")
            revealSection(effectsSection, effectsHeader, openSaveAs)
        else
            revealSection(eqSection, eqHeader, openSaveAs)
    }

    function reviewUnsaved() {
        if (root.appState.effectsState === "Modified"
                && containsFocusItem(effectsSection)) {
            reviewObject("effects", false)
        } else if (root.appState.eqState === "Modified") {
            reviewObject("eq", false)
        } else if (root.appState.effectsState === "Modified") {
            reviewObject("effects", false)
        }
    }

    function saveFocusedObject(forceSaveAs) {
        if (eqHeader.modalOpen || effectsHeader.modalOpen)
            return

        let header = null
        if (containsFocusItem(eqSection))
            header = eqHeader
        else if (containsFocusItem(effectsSection))
            header = effectsHeader
        else if (root.appState.eqState === "Modified"
                 && root.appState.effectsState !== "Modified")
            header = eqHeader
        else if (root.appState.effectsState === "Modified"
                 && root.appState.eqState !== "Modified")
            header = effectsHeader
        else if (root.appState.eqState === "Modified"
                 && root.appState.effectsState === "Modified") {
            reviewUnsaved()
            return
        }

        if (header && (forceSaveAs || header.modified))
            header.saveCurrent(forceSaveAs)
    }

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
            id: contentColumn

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
                    text: qsTr("Sound")
                    color: Theme.textPrimary
                    font.pixelSize: Theme.fontPageTitle
                    font.weight: Font.DemiBold
                }

                Label {
                    text: qsTr("Hardware output, Effects profiles and EQ presets remain separate.")
                    color: Theme.textSecondary
                    font.pixelSize: Theme.fontLabel
                }
            }

            CapabilityNotice {
                visible: root.noticeCode !== "ready"
                Layout.fillWidth: true
                Layout.leftMargin: root.pageGutter
                Layout.rightMargin: root.pageGutter
                statusCode: root.noticeCode
                title: root.appState.statusCode !== "ready"
                       ? root.appState.deviceStatus
                       : root.appState.profileCatalogStatus === "stale"
                         ? qsTr("Using cached profile data")
                         : qsTr("Profile library unavailable")
                detail: root.appState.statusCode !== "ready"
                        ? root.appState.statusDetail
                        : root.appState.profileCatalogDetail
                onRetryRequested: {
                    if (root.appState.writeErrorActive)
                        root.appState.retryStatus()
                    else
                        root.appState.refreshFromDaemon()
                }
            }

            ColumnLayout {
                id: eqSection

                Layout.fillWidth: true
                Layout.leftMargin: root.pageGutter
                Layout.rightMargin: root.pageGutter
                spacing: Theme.space3

                ObjectHeader {
                    id: eqHeader

                    objectKey: "eq"
                    Layout.fillWidth: true
                    objectTitle: qsTr("Equalizer")
                    selectorLabel: qsTr("EQ preset")
                    currentName: root.appState.eqPreset
                    stateText: root.appState.eqState
                    statusDetail: root.appState.eqDetail
                    readOnly: root.appState.eqReadOnly
                    options: root.appState.eqPresetNames
                    onSelectionRequested: name => root.appState.selectEqPreset(name)
                    onSaveRequested: root.appState.saveEqDraft()
                    onSaveAsRequested: name => root.appState.saveEqDraftAs(name)
                    onRevertRequested: root.appState.revertEqDraft()
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.controlHeightLarge + Theme.space2
                    radius: Theme.radiusSmall
                    color: Theme.surface
                    border.color: root.appState.softwareEqState === "error"
                                  ? Theme.error
                                  : root.appState.softwareEqActive
                                    ? Theme.accent
                                    : Theme.separator

                    RowLayout {
                        id: eqRuntimeLayout

                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.leftMargin: Theme.space3
                        anchors.rightMargin: Theme.space3
                        spacing: Theme.space2

                        Label {
                            text: qsTr("Live EQ")
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontLabel
                            font.weight: Font.DemiBold
                        }

                        StateBadge {
                            id: runtimeStateBadge
                            stateKind: root.appState.softwareEqState
                            stateText: {
                                switch (root.appState.softwareEqState) {
                                case "current": return qsTr("Active")
                                case "configured": return qsTr("Saved only")
                                case "different": return qsTr("Different graph")
                                case "applying": return qsTr("Applying")
                                case "error": return qsTr("Error")
                                case "unavailable": return qsTr("Unavailable")
                                default: return qsTr("Inactive")
                                }
                            }
                        }

                        Label {
                            Layout.fillWidth: true
                            text: root.appState.softwareEqDetail
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontCaption
                            elide: Text.ElideRight
                            ToolTip.visible: truncated && runtimeDetailHover.hovered
                            ToolTip.text: text

                            HoverHandler {
                                id: runtimeDetailHover
                            }
                        }

                        AppButton {
                            id: disableEqButton

                            objectName: "live-eq-disable"
                            visible: root.appState.softwareEqActive
                            enabled: visible && root.appState.softwareEqState !== "applying"
                            Accessible.ignored: !visible
                            text: qsTr("Disable EQ")
                            Accessible.name: qsTr("Disable live software equalizer")
                            onClicked: root.appState.disableSoftwareEq()
                        }

                        AppButton {
                            id: applyEqButton

                            objectName: "live-eq-apply"
                            readonly property string applyBlockReason: !root.appState.eqEnabled
                                                                         ? qsTr("Enable this EQ preset before applying it.")
                                                                         : root.appState.eqApplyBlockReason

                            enabled: root.appState.eqEnabled
                                     && root.appState.eqApplyAvailable
                                     && root.appState.softwareEqState !== "applying"
                            blockedReason: applyBlockReason
                            text: qsTr("Apply EQ")
                            Accessible.name: qsTr("Apply selected EQ draft")
                            Accessible.description: enabled
                                                    ? qsTr("Applies this draft to the live AE-5 PipeWire output.")
                                                    : applyBlockReason
                            onClicked: root.appState.applyEqDraft()
                        }
                    }

                    Accessible.role: root.appState.softwareEqState === "error"
                                     ? Accessible.AlertMessage : Accessible.StatusBar
                    Accessible.name: qsTr("Live software EQ: %1").arg(runtimeStateBadge.stateText)
                    Accessible.description: root.appState.softwareEqDetail
                }

                EqualizerGraph {
                    Layout.fillWidth: true
                    Layout.minimumHeight: 180
                    Layout.preferredHeight: root.compact
                                            ? 180
                                            : root.width >= 1200 ? 230 : 200
                    appState: root.appState
                    editingEnabled: root.appState.profileCatalogStatus === "ready"
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.leftMargin: root.pageGutter
                Layout.rightMargin: root.pageGutter
                Layout.preferredHeight: 1
                color: Theme.separatorStrong
            }

            ColumnLayout {
                id: effectsSection

                Layout.fillWidth: true
                Layout.leftMargin: root.pageGutter
                Layout.rightMargin: root.pageGutter
                Layout.bottomMargin: Theme.space4
                spacing: Theme.space2

                ObjectHeader {
                    id: effectsHeader

                    objectKey: "effects"
                    Layout.fillWidth: true
                    objectTitle: qsTr("Effects")
                    selectorLabel: qsTr("Effects profile")
                    currentName: root.appState.effectsProfile
                    stateText: root.appState.effectsState
                    statusDetail: root.appState.effectsDetail
                    readOnly: root.appState.effectsReadOnly
                    options: root.appState.effectsProfileNames
                    onSelectionRequested: name => root.appState.selectEffectsProfile(name)
                    onSaveRequested: root.appState.saveEffectsDraft()
                    onSaveAsRequested: name => root.appState.saveEffectsDraftAs(name)
                    onRevertRequested: root.appState.revertEffectsDraft()
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.controlHeightLarge + Theme.space2
                    radius: Theme.radiusSmall
                    color: Theme.surface
                    border.color: root.appState.hardwareEffectsState === "error"
                                  ? Theme.error
                                  : root.appState.hardwareEffectsActive
                                    ? Theme.accent
                                    : Theme.separator

                    RowLayout {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.leftMargin: Theme.space3
                        anchors.rightMargin: Theme.space3
                        spacing: Theme.space2

                        Label {
                            text: qsTr("Hardware Effects")
                            color: Theme.textPrimary
                            font.pixelSize: Theme.fontLabel
                            font.weight: Font.DemiBold
                        }

                        StateBadge {
                            id: effectsRuntimeStateBadge
                            stateKind: root.appState.hardwareEffectsState
                            stateText: {
                                switch (root.appState.hardwareEffectsState) {
                                case "current": return qsTr("Active")
                                case "configured": return qsTr("Saved only")
                                case "different": return qsTr("Changed outside app")
                                case "applying": return qsTr("Applying")
                                case "error": return qsTr("Error")
                                case "unavailable": return qsTr("Unavailable")
                                default: return qsTr("Inactive")
                                }
                            }
                        }

                        Label {
                            Layout.fillWidth: true
                            text: root.appState.hardwareEffectsDetail
                            color: Theme.textSecondary
                            font.pixelSize: Theme.fontCaption
                            elide: Text.ElideRight
                            ToolTip.visible: truncated && effectsRuntimeDetailHover.hovered
                            ToolTip.text: text

                            HoverHandler {
                                id: effectsRuntimeDetailHover
                            }
                        }

                        AppButton {
                            objectName: "live-effects-disable"
                            visible: root.appState.hardwareEffectsActive
                            enabled: visible
                                     && root.appState.hardwareEffectsState !== "applying"
                            Accessible.ignored: !visible
                            text: qsTr("Disable")
                            Accessible.name: qsTr("Bypass hardware Effects")
                            onClicked: root.appState.disableHardwareEffects()
                        }

                        AppButton {
                            objectName: "live-effects-apply"
                            readonly property string applyBlockReason:
                                !root.appState.effectsOutfxEnabled
                                ? qsTr("Enable the Effects master before applying this profile.")
                                : root.appState.effectsApplyBlockReason

                            enabled: root.appState.effectsOutfxEnabled
                                     && root.appState.effectsApplyAvailable
                                     && root.appState.hardwareEffectsState !== "applying"
                            blockedReason: applyBlockReason
                            text: qsTr("Apply Effects")
                            Accessible.name: qsTr("Apply selected Effects draft")
                            Accessible.description: enabled
                                                    ? qsTr("Parks active streams, writes the complete hardware profile, enables OutFX last, and verifies ALSA readback.")
                                                    : applyBlockReason
                            onClicked: root.appState.applyEffectsDraft()
                        }
                    }

                    Accessible.role: root.appState.hardwareEffectsState === "error"
                                     ? Accessible.AlertMessage : Accessible.StatusBar
                    Accessible.name: qsTr("Live hardware Effects: %1")
                                     .arg(effectsRuntimeStateBadge.stateText)
                    Accessible.description: root.appState.hardwareEffectsDetail
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: effectsMasterRow.implicitHeight + Theme.space3
                    radius: Theme.radiusSmall
                    color: root.appState.effectsOutfxEnabled
                           ? Theme.accentSubtle
                           : Theme.surface
                    border.color: root.appState.effectsOutfxEnabled
                                  ? Theme.accent
                                  : Theme.separator

                    RowLayout {
                        id: effectsMasterRow

                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.leftMargin: Theme.space3
                        anchors.rightMargin: Theme.space3
                        spacing: Theme.space3

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.space1

                            Label {
                                text: qsTr("Effects master")
                                color: Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                            }

                            Label {
                                text: qsTr("Applies enabled controls as one verified hardware transaction. OutFX is enabled last.")
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontCaption
                            }
                        }

                        AppSwitch {
                            objectName: "effects-master"
                            checked: root.appState.effectsOutfxEnabled
                            enabled: root.appState.profileCatalogStatus === "ready"
                                     && !root.appState.directMode
                            blockedReason: root.appState.directMode
                                           ? qsTr("Direct Mode bypasses Effects.")
                                           : qsTr("Effects profiles are unavailable.")
                            Accessible.name: qsTr("Effects master")
                            Accessible.description: enabled
                                                    ? qsTr("Include or bypass the enabled controls when applying this Effects profile.")
                                                    : blockedReason
                            onClicked: root.appState.updateEffectsDraft("master",
                                                                        checked, 0)
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: directModeRow.implicitHeight + Theme.space3
                    radius: Theme.radiusSmall
                    color: root.appState.directMode
                           ? Theme.accentSubtle
                           : Theme.surface
                    border.color: root.appState.directMode ? Theme.accent : Theme.separator

                    RowLayout {
                        id: directModeRow

                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        anchors.leftMargin: Theme.space3
                        anchors.rightMargin: Theme.space3
                        spacing: Theme.space3

                        ColumnLayout {
                            Layout.fillWidth: true
                            spacing: Theme.space1

                            Label {
                                text: qsTr("Direct Mode")
                                color: Theme.textPrimary
                                font.pixelSize: Theme.fontBody
                            }

                            Label {
                                text: root.appState.directMode
                                      ? qsTr("EQ and enhancements are visible but bypassed.")
                                      : qsTr("Bypasses EQ and enhancements when enabled.")
                                color: Theme.textSecondary
                                font.pixelSize: Theme.fontCaption
                            }
                        }

                        AppSwitch {
                            objectName: "direct-mode"
                            checked: root.appState.directMode
                            enabled: root.appState.directModeAvailable
                                     && root.appState.directModeWriteEnabled
                            blockedReason: root.appState.directModeAvailable
                                           ? root.appState.hardwareWriteBlockReason
                                           : qsTr("Direct Mode is not exposed by the current driver.")
                            Accessible.name: qsTr("Direct Mode")
                            Accessible.description: enabled
                                                    ? qsTr("Toggle Direct Mode")
                                                    : root.appState.directModeAvailable
                                                      ? root.appState.hardwareWriteBlockReason
                                                      : qsTr("Direct Mode is not exposed by the current driver.")
                            onClicked: root.appState.setPreviewDirectMode(checked)
                        }
                    }
                }

                GridLayout {
                    Layout.fillWidth: true
                    columns: root.width >= Theme.effectsColumnsBreakpoint ? 2 : 1
                    columnSpacing: Theme.space5
                    rowSpacing: Theme.space1

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        controlKey: "surround"
                        title: qsTr("Surround")
                        initialValue: root.appState.surroundLevel
                        initiallyEnabled: root.appState.surroundEnabled
                        available: root.appState.surroundAvailable
                        editingEnabled: root.appState.profileCatalogStatus === "ready"
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        controlKey: "crystalizer"
                        title: qsTr("Crystalizer")
                        initialValue: root.appState.crystalizerLevel
                        initiallyEnabled: root.appState.crystalizerEnabled
                        available: root.appState.crystalizerAvailable
                        editingEnabled: root.appState.profileCatalogStatus === "ready"
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        controlKey: "bass"
                        title: qsTr("Bass")
                        initialValue: root.appState.bassLevel
                        initiallyEnabled: root.appState.bassEnabled
                        available: root.appState.bassAvailable
                        editingEnabled: root.appState.profileCatalogStatus === "ready"
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        controlKey: "smart-volume"
                        title: qsTr("Smart Volume")
                        initialValue: root.appState.smartVolumeLevel
                        initiallyEnabled: root.appState.smartVolumeEnabled
                        available: root.appState.smartVolumeAvailable
                        editingEnabled: root.appState.profileCatalogStatus === "ready"
                        leftPole: qsTr("Night")
                        rightPole: qsTr("Loud")
                        controlsEnabled: !root.appState.directMode
                    }

                    EnhancementRow {
                        Layout.fillWidth: true
                        appState: root.appState
                        controlKey: "dialog"
                        title: qsTr("Dialog+")
                        initialValue: root.appState.dialogLevel
                        initiallyEnabled: root.appState.dialogEnabled
                        available: root.appState.dialogAvailable
                        editingEnabled: root.appState.profileCatalogStatus === "ready"
                        controlsEnabled: !root.appState.directMode
                    }
                }
            }
        }
    }
}
