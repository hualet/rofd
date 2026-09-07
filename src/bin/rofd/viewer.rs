use log::{error, info};
use qmetaobject::prelude::*;

use rofd_core::{Document, LoadOptions};
use rofd_render::{CairoRenderer, RenderOptions};

#[derive(Default, QObject)]
pub struct OfdViewer {
    base: qt_base_class!(trait QObject),

    page_source: qt_property!(QString; NOTIFY page_source_changed),
    page_source_changed: qt_signal!(),

    current_page: qt_property!(i32; NOTIFY current_page_changed),
    current_page_changed: qt_signal!(),

    page_count: qt_property!(i32; NOTIFY page_count_changed),
    page_count_changed: qt_signal!(),

    open_file: qt_method!(fn(&self, url: QString)),
    next_page: qt_method!(fn(&mut self)),
    previous_page: qt_method!(fn(&mut self)),

    document: Option<Document>,
    render_counter: u32,
}

impl OfdViewer {
    pub fn open_file(&mut self, url: QString) {
        let path = local_path_from_url(&url.to_string());
        match Document::open(&path, LoadOptions::default()) {
            Ok(document) => {
                info!("opened {}: {} page(s)", path, document.page_count());
                self.page_count = document.page_count() as i32;
                self.page_count_changed();
                self.document = Some(document);
                self.show_page(0);
            }
            Err(e) => error!("failed to open {}: {}", path, e),
        }
    }

    fn next_page(&mut self) {
        self.show_page(self.current_page + 1);
    }

    fn previous_page(&mut self) {
        self.show_page(self.current_page - 1);
    }

    fn show_page(&mut self, index: i32) {
        let Some(document) = &self.document else {
            return;
        };
        if index < 0 || index >= document.page_count() as i32 {
            return;
        }

        let result = (|| -> std::result::Result<String, Box<dyn std::error::Error>> {
            let page = document.page(index as usize)?;
            let options = RenderOptions::default();
            let (width, height) = CairoRenderer::pixel_size(&page, &options)?;

            let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height)?;
            let context = cairo::Context::new(&surface)?;
            CairoRenderer.render_page(&page, &context, &options)?;
            drop(context);

            let path = std::env::temp_dir()
                .join(format!("rofd-page-{}-{}.png", self.render_counter, index));
            let mut file = std::fs::File::create(&path)?;
            surface.write_to_png(&mut file)?;
            Ok(format!("file://{}", path.display()))
        })();

        match result {
            Ok(source) => {
                self.render_counter += 1;
                self.page_source = QString::from(source);
                self.page_source_changed();
                self.current_page = index;
                self.current_page_changed();
            }
            Err(e) => error!("failed to render page {}: {}", index + 1, e),
        }
    }
}

fn local_path_from_url(url: &str) -> String {
    let path = url.strip_prefix("file://").unwrap_or(url);
    percent_decode(path.as_bytes())
}

fn percent_decode(bytes: &[u8]) -> String {
    fn hex_val(b: u8) -> Option<u8> {
        match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            b'A'..=b'F' => Some(b - b'A' + 10),
            _ => None,
        }
    }

    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                out.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
