#[cfg(feature = "qml-gui")]
use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    #[cfg(feature = "qml-gui")]
    CxxQtBuilder::new_qml_module(
        QmlModule::new("io.github.klimovich008.ae5control")
            .qml_files([
                "qml/Main.qml",
                "qml/components/NavigationSidebar.qml",
                "qml/components/ObjectHeader.qml",
                "qml/components/CapabilityNotice.qml",
                "qml/components/EqualizerGraph.qml",
                "qml/components/EnhancementRow.qml",
                "qml/components/HardwareFaceplate.qml",
                "qml/pages/SoundPage.qml",
            ])
            .qml_file(QmlFile::from("qml/Theme.qml").singleton(true))
            .depend("QtQuick")
            .depend("QtQuick.Controls")
            .depend("QtQuick.Layouts")
            .depend("QtQuick.Shapes"),
    )
    .qt_module("Network")
    .qt_module("Quick")
    .files(["src/qml_app_state.rs"])
    .build();
}
