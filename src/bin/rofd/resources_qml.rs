// TODO(hualet): https://github.com/gyroflow/gyroflow/blob/75be0451a561ec0cd551342d361b022347ea7ec9/build.rs

use qmetaobject::qrc;

qrc!(pub rsrc_qml,
    "/" {
        "src/bin/rofd/ui/main_window.qml",
    }
);