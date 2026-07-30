// SPDX-License-Identifier: MIT OR Apache-2.0
#pragma once

#include "cxx-qt-lib/qcoreapplication.h"

#include <QtGui/QGuiApplication>
#include <QtWidgets/QApplication>

#include <memory>

namespace ae5 {

inline std::unique_ptr<QGuiApplication>
qapplicationNew(const QVector<QByteArray>& arguments)
{
    // QApplication is required by Qt.labs.platform's native tray icon and
    // menu implementation. Keep returning the QGuiApplication base type so
    // the rest of the Rust entry point can use cxx-qt-lib's safe wrapper.
    auto argumentData = new rust::cxxqtlib1::ApplicationArgsData(arguments);
    std::unique_ptr<QGuiApplication> application =
        std::make_unique<QApplication>(
            argumentData->size(), argumentData->data());
    Q_ASSERT(application != nullptr);
    argumentData->setParent(application.get());
    return application;
}

} // namespace ae5
