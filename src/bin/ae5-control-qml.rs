use cxx_qt::casting::Upcast;
use cxx_qt_lib::{QQmlApplicationEngine, QQmlEngine, QQuickStyle, QString, QUrl};
use std::pin::Pin;

fn main() {
    if let Err(error) = ae5_control::qml_app_state::validate_qa_arguments() {
        eprintln!("ae5-control-qml: {error}");
        std::process::exit(2);
    }

    // Make the generated C++ initializer a direct executable dependency.
    // CXX-Qt's automatic initializer archive otherwise appears after this
    // crate's archive, which GNU gold cannot resolve in a single pass.
    cxx_qt::init_crate!(ae5_control);
    ae5_control::qml_app_state::initialize();
    QQuickStyle::set_style(&QString::from("Basic"));

    let mut app = ae5_control::qapplication::new_qapplication();
    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(
            "qrc:/qt/qml/io/github/klimovich008/ae5control/qml/Main.qml",
        ));
    }

    if let Some(engine) = engine.as_mut() {
        let engine: Pin<&mut QQmlEngine> = engine.upcast_pin();
        engine.on_quit(|_| {}).release();
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
