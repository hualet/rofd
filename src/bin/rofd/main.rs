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
    let paintable = widgets::OFDWidget::default();
    picture.set_paintable(Some(&paintable));
    picture.set_focusable(true);


    let ev_ctrl = gtk::EventControllerKey::new();
    ev_ctrl.connect_key_released(move |_ctrl, key, _keycode, modifier| {
        debug!("key: {}, modifier: {:?}", key, modifier);
        match (key, modifier) {
            (gdk::Key::equal, gdk::ModifierType::CONTROL_MASK) => {
                zoom_in();
            },
            (gdk::Key::minus, gdk::ModifierType::CONTROL_MASK) => {
                zoom_out();
            },
            _ => ()
        }
    });
    picture.add_controller(ev_ctrl);

    window.set_child(Some(&picture));
    window.present();
}

fn zoom_in() {
    debug!("zoom in");
}

fn zoom_out() {
    debug!("zoom out");
}