use cxx_qt_lib::{QByteArray, QGuiApplication, QVector};

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qbytearray.h");
        type QByteArray = cxx_qt_lib::QByteArray;

        include!("cxx-qt-lib/core/qvector/qvector_QByteArray.h");
        type QVector_QByteArray = cxx_qt_lib::QVector<QByteArray>;

        include!("cxx-qt-lib/qguiapplication.h");
        type QGuiApplication = cxx_qt_lib::QGuiApplication;

        include!("ae5-control/ae5_qapplication.h");
        #[namespace = "ae5"]
        #[rust_name = "qapplication_new"]
        fn qapplicationNew(arguments: &QVector_QByteArray) -> UniquePtr<QGuiApplication>;
    }
}

/// Creates the Qt application object used by the QML frontend.
///
/// The returned base pointer owns a `QApplication`, which is necessary for
/// the native system-tray icon and its context menu.
pub fn new_qapplication() -> cxx::UniquePtr<QGuiApplication> {
    let mut arguments = QVector::<QByteArray>::default();

    for argument in std::env::args_os() {
        #[cfg(unix)]
        use std::os::unix::ffi::OsStrExt;

        #[cfg(windows)]
        let argument = argument.to_string_lossy();

        arguments.append(QByteArray::from(argument.as_bytes()));
    }

    ffi::qapplication_new(&arguments)
}
