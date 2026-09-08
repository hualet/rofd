use qmetaobject::prelude::*;
use qmetaobject::QUrl;

mod cache;
mod reader;
mod resources_qml;
mod viewer;
mod worker;

fn main() {
    env_logger::init();

    resources_qml::rsrc_qml();

    let mut viewer = viewer::OfdViewer::new();

    // Optionally open a document passed on the command line.
    if let Some(path) = std::env::args().nth(1) {
        viewer.open_file(QString::from(path));
    }

    let viewer = qmetaobject::QObjectBox::new(viewer);

    let mut engine = QmlEngine::new();
    engine.set_object_property(QString::from("ofdViewer"), viewer.pinned());
    engine.load_url(QUrl::from(QString::from("qrc:/main_window.qml")));

    engine.exec();
}
