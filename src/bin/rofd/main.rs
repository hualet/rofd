/*
use env_logger;

use gtk::{gdk, glib, prelude::*};

use log::debug;

mod widgets;

fn main() -> glib::ExitCode {
    env_logger::init();

    let application = gtk::Application::builder()
        .application_id("org.hualet.rofd")
        .build();
    application.connect_activate(build_ui);
    application.run()
}

fn build_ui(application: &gtk::Application) {
    // window
    let window = gtk::ApplicationWindow::new(application);

    window.set_title(Some("OFD Reader"));
    window.set_default_size(400, 300);

    // header_bar
    let header_bar = gtk::HeaderBar::new();
    window.set_titlebar(Some(&header_bar));

    let zoom_in_button = gtk::Button::new();
    zoom_in_button.set_icon_name("zoom-in-symbolic");
    let zoom_out_button = gtk::Button::new();
    zoom_out_button.set_icon_name("zoom-out-symbolic");

    let model = gtk::StringList::new(&["25%", "50%", "75%", "100%", "125%",
        "150%", "175%", "200%"]);
    let zoom_drop_down = gtk::DropDown::new(Some(model), gtk::Expression::NONE);

    header_bar.pack_end(&zoom_in_button);
    header_bar.pack_end(&zoom_drop_down);
    header_bar.pack_end(&zoom_out_button);

    // ofd widget
    let picture = gtk::Picture::builder()
        .content_fit(gtk::ContentFit::ScaleDown)
        .build();
    let ofd_widget = widgets::OFDWidget::default();
    picture.set_paintable(Some(&ofd_widget));
    picture.set_focusable(true);


    let ev_ctrl = gtk::EventControllerKey::new();
    // see https://github.com/gtk-rs/gtk4-rs/blob/main/examples/text_viewer/main.rs
    ev_ctrl.connect_key_released(glib::clone!(
        #[weak] picture,
        #[weak] ofd_widget,
        move |_ctrl, key, _keycode, modifier| {
            debug!("key: {}, modifier: {:?}", key, modifier);
            match (key, modifier) {
                (gdk::Key::equal, gdk::ModifierType::CONTROL_MASK) => {
                    ofd_widget.zoom_in();
                    picture.queue_draw();
                },
                (gdk::Key::minus, gdk::ModifierType::CONTROL_MASK) => {
                    ofd_widget.zoom_out();
                    picture.queue_draw();
                },
                _ => ()
            }
        }
    ));
    picture.add_controller(ev_ctrl);

    window.set_child(Some(&picture));
    window.present();
}
*/

use rofd::{read_ofd, render_ofd, TargetDevice};

use cpp::cpp;
use log::debug;
use qmetaobject::{QImage, QPainter, QSize, ImageFormat};
use qmetaobject::prelude::*;

use std::fs;
use std::path::Path;

fn qpainter_from_qimage(value: QImage) -> QPainter {
    cpp!(unsafe [value as "QImage*"] -> QPainter as "QPainter*" {
        QPainter *painter = new QPainter(value);
        return painter;
    })
}

fn main() {
    env_logger::init();

    let mut ofd_node = read_ofd("learning/test.ofd").unwrap();

    let size = QSize{ width: 300, height: 300 };
    let qimage = QImage::new(size, ImageFormat::ARGB32);
    let qpainter = qpainter_from_qimage(qimage);

    render_ofd(
        &mut ofd_node,
        &mut TargetDevice::QPainter(qpainter),
    );
    // qimage.save("qimage_target.png");

    let mut engine = QmlEngine::new();
    let qml_root_path = Path::new(file!()).parent().unwrap()
        .join("ui/main_window.qml");
    let qml_root_content = fs::read_to_string(qml_root_path).unwrap();

    engine.load_data(qml_root_content.into());
    engine.exec();
}
