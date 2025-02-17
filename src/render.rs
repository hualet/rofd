use std::path::Path;
use std::io::Cursor;
use std::io::Read;

use log::debug;

use cairo;
use cairo::Error;

use qmetaobject::{QRectF, QColor, QPen};

use crate::ofd::Ofd;
use crate::document::Document;
use crate::page::Page;
use crate::elements::*;

use crate::types::{mmtopx, ct};

pub trait Renderable {
    fn render_to_cairo_context(&self, context: &mut cairo::Context,
        ofd: &mut Ofd, document: &Document) -> Result<(), Error>;
    fn render_to_qpainter(&self, qpainter: &mut qmetaobject::QPainter,
        ofd: &mut Ofd, document: &Document);
}

impl Renderable for Document {
    fn render_to_cairo_context(&self, _context: &mut cairo::Context,
        _ofd: &mut Ofd, _document: &Document) -> Result<(), Error> {
        debug!("render document to cairo");
        Ok(())
    }

    fn render_to_qpainter(&self, qpainter: &mut qmetaobject::QPainter,
        _ofd: &mut Ofd, _document: &Document) {
        debug!("render document to qpainter");
    }
}

impl Renderable for Page {
    fn render_to_cairo_context(&self, context: &mut cairo::Context,
        ofd: &mut Ofd, document: &Document) -> Result<(), Error> {
        debug!("render page");
        _render_page_block_to_cairo(self.content.layer.events.clone(),
            context, ofd, document)
    }

    fn render_to_qpainter(&self, qpainter: &mut qmetaobject::QPainter,
        ofd: &mut Ofd, document: &Document) {
        debug!("render page to qpainter");
        _render_page_block_to_qpainter(self.content.layer.events.clone(),
            qpainter, ofd, document)
    }
}

impl Renderable for PathObject {
    fn render_to_cairo_context(&self, context: &mut cairo::Context,
        _ofd: &mut Ofd, _document: &Document) -> Result<(), Error> {
        context.save()?;

        // TODO(hualet): implement ctm.
        let boundary = ct::Box::from(self.boundary.clone()).to_pixel();
        let color = ct::Color::from(
            self.stroke_color.as_ref().unwrap().value.clone());

        context.set_source_rgb(color.value[0] as f64 / 255.0,
            color.value[1] as f64 / 255.0,
            color.value[2] as f64 / 255.0);
        context.set_line_width(mmtopx(self.line_width));

        context.move_to(boundary.x as f64, boundary.y as f64);
        context.line_to((boundary.x + boundary.width) as f64,
            boundary.y as f64);
        context.line_to((boundary.x + boundary.width) as f64,
            (boundary.y + boundary.height) as f64);
        context.line_to(boundary.x as f64,
            (boundary.y + boundary.height) as f64);
        context.line_to(boundary.x as f64, boundary.y as f64);

        context.stroke()?;

        context.restore()
    }

    fn render_to_qpainter(&self, painter: &mut qmetaobject::QPainter,
        _ofd: &mut Ofd, _document: &Document) {
        debug!("render path object to qpainter");

        painter.save();

        // TODO(hualet): implement ctm.
        let boundary = ct::Box::from(self.boundary.clone()).to_pixel();
        let color = ct::Color::from(
            self.stroke_color.as_ref().unwrap().value.clone());

        let pen_color = QColor::from_rgb(color.value[0], color.value[1],
            color.value[2]);
        let mut pen = QPen::from_color(pen_color);
        pen.set_width(mmtopx(self.line_width) as i32);
        painter.set_pen(pen);

        let rect = QRectF {
            x: boundary.x as f64,
            y: boundary.y as f64,
            width: boundary.width as f64,
            height: boundary.height as f64
        };
        painter.draw_rect(rect);

        painter.restore();
    }
}

impl Renderable for TextObject {
    fn render_to_cairo_context(&self, context: &mut cairo::Context,
        _ofd: &mut Ofd, document: &Document) -> Result<(), Error> {
        context.save()?;

        let boundary = ct::Box::from(self.boundary.clone()).to_pixel();
        let color = self.fill_color.as_ref().unwrap_or(&Color::default()).value.clone();
        let fill_color = ct::Color::from(color);

        let font_id = self.font;
        for font in document.public_res.fonts.font.iter() {
            if font.id == font_id {
                // TODO(hualet): custom font file loading.
                context.select_font_face(font.family_name.as_str(),
                    cairo::FontSlant::Normal, cairo::FontWeight::Normal);
                break;
            }
        }
        context.set_font_size(mmtopx(self.size) as f64);

        context.set_source_rgb(fill_color.value[0] as f64 / 255.0,
            fill_color.value[1] as f64 / 255.0,
            fill_color.value[2] as f64 / 255.0);

        // NOTE(hualet): transform should be used together with translate,
        // so the coordinate system is correct.
        // THEY ARE BOTH TRANSFORMATIONS!
        context.translate(boundary.x as f64 + mmtopx(self.text_code.x),
            boundary.y as f64 + mmtopx(self.text_code.y));
        if let Some(ctm) = self.ctm.as_ref() {
            debug!("render text object:{:?} with ctm: {:?}",
                self.text_code.value, ctm);
            let matrix = ct::Matrix::from(ctm.clone());
            let cairo_matrix: cairo::Matrix = matrix.into();
            context.transform(cairo_matrix);
        }

        context.move_to(0., 0.);
        context.show_text(self.text_code.value.as_str())?;

        context.restore()
    }

    fn render_to_qpainter(&self, qpainter: &mut qmetaobject::QPainter,
        _ofd: &mut Ofd, _document: &Document) {
        debug!("render text object to qpainter");
    }
}

// implement Renderable for ImageObject
impl Renderable for ImageObject {
    fn render_to_cairo_context(&self, context: &mut cairo::Context,
        ofd: &mut Ofd, document: &Document) -> Result<(), Error> {
        context.save()?;

        // TODO(hualet): implement ctm.
        let boundary = ct::Box::from(self.boundary.clone()).to_pixel();

        // find the image file:
        // 1) find the resource file in DocumentRes with the resource id
        // 2) construct the path of the image file
        // 3) load the image file and draw
        for resource in document.doc_res.multi_medias.multi_media.iter() {
            if resource.id == self.resource_id {
                let path = Path::new(ofd.node.doc_body.doc_root.as_str());
                let res_path = &path.parent().unwrap()
                    .join(document.doc_res.base_loc.as_str())
                    .join(resource.media_file.as_str());

                let mut file = ofd.zip_archive.by_name(res_path.to_str().unwrap()).unwrap();
                let mut content = Vec::new();
                let _size = file.read_to_end(&mut content).unwrap();

                let mut file_reader = Cursor::new(content);
                // FIXME(hualet): png is not for sure.
                let surface = cairo::ImageSurface::create_from_png(&mut file_reader).unwrap();
                context.scale(boundary.width/ surface.width() as f64,
                    boundary.height/ surface.height() as f64);
                context.set_source_surface(&surface,
                    boundary.x as f64,
                    boundary.y as f64)?;
                context.paint()?;
            }
        }


        context.restore()
    }

    fn render_to_qpainter(&self, qpainter: &mut qmetaobject::QPainter,
        ofd: &mut Ofd, document: &Document) {
        debug!("render pageblock to qpainter");
    }
}

impl Renderable for PageBlock {
    fn render_to_cairo_context(&self, context: &mut cairo::Context,
        ofd: &mut Ofd, document: &Document) -> Result<(), Error> {
        debug!("render pageblock");
        _render_page_block_to_cairo(self.events.clone(), context, ofd, document)
    }

    fn render_to_qpainter(&self, qpainter: &mut qmetaobject::QPainter,
        ofd: &mut Ofd, document: &Document) {
        debug!("render pageblock to qpainter");
    }
}


fn _render_page_block_to_cairo(events: Vec<Event>, context: &mut cairo::Context,
    ofd: &mut Ofd, document: &Document) -> Result<(), Error> {
    for event in events.iter() {
        match event {
            Event::PathObject(p) => {
                match p.render_to_cairo_context(context, ofd, document) {
                    Ok(_) => (),
                    Err(e) => return Err(e),
                }
            }
            Event::TextObject(t) => {
                match t.render_to_cairo_context(context, ofd, document) {
                    Ok(_) => (),
                    Err(e) => return Err(e),
                }
            }
            Event::ImageObject(i) => {
                match i.render_to_cairo_context(context, ofd, document) {
                    Ok(_) => (),
                    Err(e) => return Err(e),
                }
            }
            Event::PageBlock(p) => {
                match p.render_to_cairo_context(context, ofd, document) {
                    Ok(_) => (),
                    Err(e) => return Err(e),
                }
            }
        }
    }

    Ok(())
}

fn _render_page_block_to_qpainter(events: Vec<Event>, qpainter: &mut qmetaobject::QPainter,
    ofd: &mut Ofd, document: &Document) {
    for event in events.iter() {
        match event {
            Event::PathObject(p) => {
                p.render_to_qpainter(qpainter, ofd, document)
            }
            Event::TextObject(t) => {
                t.render_to_qpainter(qpainter, ofd, document)
            }
            Event::ImageObject(i) => {
                i.render_to_qpainter(qpainter, ofd, document)
            }
            Event::PageBlock(p) => {
                p.render_to_qpainter(qpainter, ofd, document)
            }
        }
    }
}

/*
    ct::Matrix

    | a b 0 |
    | c d 0 |
    | e f 1 |

    x'=ax+cy+e
    y'=bx+dy+f


    cairo::Matrix

    typedef struct {
        double xx; double yx;
        double xy; double yy;
        double x0; double y0;
    } cairo_matrix_t;

    x_new = xx * x + xy * y + x0;
    y_new = yx * x + yy * y + y0;
*/
impl From<ct::Matrix> for cairo::Matrix {
    fn from(value: ct::Matrix) -> Self {
        Self::new(
            value.a, // xx
            value.b, // yx
            value.c, // xy
            value.d, // yy
            value.e, // x0
            value.f  // y0
        )
    }
}