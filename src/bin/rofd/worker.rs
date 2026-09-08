//! Background document session. Only plain Rust values cross the GUI boundary.
use crate::{cache::RenderKey, reader::SearchHit};
use base64::Engine;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, Arc, Mutex,
};

#[derive(Clone, Debug, Serialize)]
pub struct PageInfo {
    pub width: f64,
    pub height: f64,
    pub error: String,
}
#[derive(Debug)]
pub enum Event {
    Opened {
        request: u64,
        title: String,
        pages: Vec<PageInfo>,
    },
    OpenFailed {
        request: u64,
        error: String,
    },
    Rendered {
        generation: u64,
        key: RenderKey,
        source: Option<Arc<String>>,
        error: String,
        reduced: bool,
    },
    Searched {
        generation: u64,
        request: u64,
        hits: Vec<SearchHit>,
        status: String,
    },
}
#[derive(Debug)]
pub enum Command {
    Activate {
        request: u64,
    },
    Open {
        request: u64,
        path: PathBuf,
    },
    Render {
        generation: u64,
        key: RenderKey,
        request: i32,
    },
    Search {
        generation: u64,
        request: u64,
        query: String,
    },
}
#[derive(Default)]
pub struct Cancellation {
    pub open: AtomicU64,
    pub search: AtomicU64,
    renders: Mutex<std::collections::HashMap<(u64, usize, bool), i32>>,
}
pub struct Worker {
    pub commands: mpsc::Sender<Command>,
    pub events: mpsc::Receiver<Event>,
    pub cancellation: Arc<Cancellation>,
}
impl Cancellation {
    fn current_render(&self, generation: u64, key: RenderKey, request: i32) -> bool {
        self.renders
            .lock()
            .unwrap()
            .get(&(generation, key.page, key.thumbnail))
            == Some(&request)
    }
}
impl Worker {
    pub fn request_render(&self, generation: u64, key: RenderKey, request: i32) {
        self.cancellation
            .renders
            .lock()
            .unwrap()
            .insert((generation, key.page, key.thumbnail), request);
        let _ = self.commands.send(Command::Render {
            generation,
            key,
            request,
        });
    }
    pub fn cancel_render(&self, request: i32) {
        self.cancellation
            .renders
            .lock()
            .unwrap()
            .retain(|_, current| *current != request);
    }
    pub fn start() -> Self {
        let (commands, inbox) = mpsc::channel();
        let (outbox, events) = mpsc::sync_channel(4);
        let cancellation = Arc::new(Cancellation::default());
        let worker_cancellation = cancellation.clone();
        std::thread::spawn(move || run(inbox, outbox, worker_cancellation));
        Self {
            commands,
            events,
            cancellation,
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancellation.open.store(u64::MAX, Ordering::Relaxed);
        self.cancellation.search.store(u64::MAX, Ordering::Relaxed);
    }
}

struct Session {
    document: rofd_core::Document,
    generation: u64,
    cache: crate::cache::RasterCache,
    fonts: Option<rofd_render::SystemFontResolver>,
    images: rofd_render::ImageDecoder,
}
struct Search {
    generation: u64,
    request: u64,
    query: String,
    page: usize,
    hits: Vec<SearchHit>,
    has_text: bool,
    errors: usize,
}

fn run(
    inbox: mpsc::Receiver<Command>,
    outbox: mpsc::SyncSender<Event>,
    cancellation: Arc<Cancellation>,
) {
    let mut session: Option<Session> = None;
    let mut candidate: Option<Session> = None;
    let mut search: Option<Search> = None;
    let mut pending = std::collections::VecDeque::new();
    loop {
        if pending.is_empty() && search.is_none() {
            match inbox.recv() {
                Ok(command) => pending.push_back(command),
                Err(_) => break,
            }
        }
        // Coalesce obsolete resolutions before spending time rasterizing them.
        loop {
            match inbox.try_recv() {
                Ok(command) => {
                    if let Command::Render {
                        generation, key, ..
                    } = &command
                    {
                        pending.retain(|previous| !matches!(previous,
                            Command::Render { generation: old, key: old_key, .. }
                            if old == generation && old_key.page == key.page && old_key.thumbnail == key.thumbnail));
                    }
                    pending.push_back(command);
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => return,
            }
        }
        // Opening/replacing documents and search cancellation take priority over thumbnails.
        let priority = pending
            .iter()
            .position(|cmd| !matches!(cmd, Command::Render { .. }))
            .or_else(|| {
                pending
                    .iter()
                    .position(|cmd| matches!(cmd, Command::Render { key, .. } if !key.thumbnail))
            });
        let command = if let Some(index) = priority {
            pending.remove(index)
        } else {
            pending.pop_front()
        };
        if let Some(command) = command {
            match command {
                Command::Activate { request } => {
                    if candidate
                        .as_ref()
                        .is_some_and(|candidate| candidate.generation == request)
                    {
                        session = candidate.take();
                        search = None;
                    }
                }
                Command::Open { request, path } => {
                    if cancellation.open.load(Ordering::Relaxed) != request {
                        continue;
                    }
                    candidate = None;
                    match open_session(request, &path, &cancellation) {
                        Ok(Some((opened, pages))) => {
                            if cancellation.open.load(Ordering::Relaxed) != request {
                                continue;
                            }
                            let title = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned();
                            candidate = Some(opened);
                            if outbox
                                .send(Event::Opened {
                                    request,
                                    title,
                                    pages,
                                })
                                .is_err()
                            {
                                return;
                            }
                        }
                        Ok(None) => {}
                        Err(error) => {
                            if outbox.send(Event::OpenFailed { request, error }).is_err() {
                                return;
                            }
                        }
                    }
                }
                Command::Render {
                    generation,
                    key,
                    request,
                } => {
                    if !cancellation.current_render(generation, key, request) {
                        continue;
                    }
                    let Some(active) = session
                        .as_mut()
                        .filter(|active| active.generation == generation)
                    else {
                        continue;
                    };
                    let (source, error, reduced) = match render(active, key) {
                        Ok((path, reduced)) => (Some(path), String::new(), reduced),
                        Err(error) => (None, error, false),
                    };
                    if !cancellation.current_render(generation, key, request) {
                        continue;
                    }
                    if outbox
                        .send(Event::Rendered {
                            generation,
                            key,
                            source,
                            error,
                            reduced,
                        })
                        .is_err()
                    {
                        return;
                    }
                }
                Command::Search {
                    generation,
                    request,
                    query,
                } => {
                    if cancellation.search.load(Ordering::Relaxed) != request {
                        continue;
                    }
                    if session
                        .as_ref()
                        .is_some_and(|active| active.generation == generation)
                    {
                        search = Some(Search {
                            generation,
                            request,
                            query,
                            page: 0,
                            hits: Vec::new(),
                            has_text: false,
                            errors: 0,
                        });
                    }
                }
            }
        }
        // One page per scheduling turn allows navigation to interrupt a large search.
        if let (Some(job), Some(active)) = (search.as_mut(), session.as_ref()) {
            if cancellation.search.load(Ordering::Relaxed) != job.request
                || active.generation != job.generation
            {
                search = None;
                continue;
            }
            if !job.query.is_empty() && job.page < active.document.page_count() {
                match active.document.page(job.page) {
                    Ok(page) => {
                        job.has_text |= crate::reader::search_page(&page, &job.query, &mut job.hits)
                    }
                    Err(_) => job.errors += 1,
                }
                job.page += 1;
            }
            if job.query.is_empty()
                || job.page >= active.document.page_count()
                || job.hits.len() >= crate::reader::MAX_MATCHES
            {
                let mut status = if job.query.is_empty() {
                    "输入文字搜索文档".into()
                } else if job.hits.len() >= crate::reader::MAX_MATCHES {
                    "至少 1000 条匹配（已达到显示上限）".into()
                } else if !job.has_text && job.errors == 0 {
                    "未检测到可搜索文字".into()
                } else if job.hits.is_empty() {
                    "未找到匹配内容".into()
                } else {
                    format!("找到 {} 条匹配", job.hits.len())
                };
                if job.errors > 0 {
                    status += &format!(" · {} 页无法解析，搜索不完整", job.errors);
                }
                let event = Event::Searched {
                    generation: job.generation,
                    request: job.request,
                    hits: std::mem::take(&mut job.hits),
                    status,
                };
                if outbox.send(event).is_err() {
                    return;
                }
                search = None;
            }
        }
    }
}

fn open_session(
    request: u64,
    path: &std::path::Path,
    cancellation: &Cancellation,
) -> Result<Option<(Session, Vec<PageInfo>)>, String> {
    let document = rofd_core::Document::open(path, rofd_core::LoadOptions::default())
        .map_err(|e| e.to_string())?;
    let mut pages = Vec::with_capacity(document.page_count());
    for index in 0..document.page_count() {
        if cancellation.open.load(Ordering::Relaxed) != request {
            return Ok(None);
        }
        pages.push(match document.page_size(index) {
            Ok(size) => PageInfo {
                width: size.width * crate::reader::PX_PER_MM,
                height: size.height * crate::reader::PX_PER_MM,
                error: String::new(),
            },
            Err(error) => PageInfo {
                width: 210. * crate::reader::PX_PER_MM,
                height: 297. * crate::reader::PX_PER_MM,
                error: error.to_string(),
            },
        });
    }
    Ok(Some((
        Session {
            document,
            generation: request,
            cache: crate::cache::RasterCache::new(128 * 1024 * 1024),
            fonts: None,
            images: rofd_render::ImageDecoder::default(),
        },
        pages,
    )))
}

fn render(session: &mut Session, key: RenderKey) -> Result<(Arc<String>, bool), String> {
    if let Some(cached) = session.cache.get(key) {
        return Ok(cached);
    }
    let page = session.document.page(key.page).map_err(|e| e.to_string())?;
    let (scale, reduced) = crate::reader::raster_scale(page.size(), key.scale());
    let options = rofd_render::RenderOptions {
        scale,
        ..Default::default()
    };
    let (width, height) =
        rofd_render::CairoRenderer::pixel_size(&page, &options).map_err(|e| e.to_string())?;
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, width, height)
        .map_err(|e| e.to_string())?;
    let context = cairo::Context::new(&surface).map_err(|e| e.to_string())?;
    let fonts = session.fonts.get_or_insert_with(|| {
        rofd_render::SystemFontResolver::with_system_fonts(
            vec!["Noto Sans CJK SC".into(), "sans-serif".into()],
            page.resource_limits().max_font_bytes,
        )
    });
    rofd_render::CairoRenderer
        .render_page_with_services(&page, &context, &options, fonts, &session.images)
        .map_err(|e| e.to_string())?;
    drop(context);
    let mut png = Vec::new();
    surface.write_to_png(&mut png).map_err(|e| e.to_string())?;
    let source = Arc::new(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(png)
    ));
    let bytes = width as u64 * height as u64 * 4 + source.len() as u64;
    session.cache.insert(key, source.clone(), bytes, reduced);
    Ok((source, reduced))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("learning/test.ofd")
    }
    fn receive(worker: &Worker) -> Event {
        worker.events.recv_timeout(Duration::from_secs(30)).unwrap()
    }
    #[test]
    fn superseded_and_offscreen_render_requests_are_cancelled() {
        let worker = Worker::start();
        let key = RenderKey::new(0, 1., false);
        worker.request_render(1, key, 1);
        assert!(worker.cancellation.current_render(1, key, 1));
        worker.request_render(1, RenderKey::new(0, 2., false), 2);
        assert!(!worker.cancellation.current_render(1, key, 1));
        worker.cancel_render(1);
        assert!(worker.cancellation.current_render(1, key, 2));
        worker.cancel_render(2);
        assert!(!worker.cancellation.current_render(1, key, 2));
    }
    #[test]
    fn actual_raster_is_owned_png_and_cached_without_temporary_files() {
        let cancellation = Cancellation::default();
        cancellation.open.store(1, Ordering::Relaxed);
        let (mut session, _) = open_session(1, &fixture(), &cancellation).unwrap().unwrap();
        let key = RenderKey::new(0, 0.25, true);
        let (source, reduced) = render(&mut session, key).unwrap();
        assert!(!reduced);
        let encoded = source.strip_prefix("data:image/png;base64,").unwrap();
        let png = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .unwrap();
        assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"));
        let cached = render(&mut session, key).unwrap().0;
        assert!(Arc::ptr_eq(&source, &cached));
        drop(session);
        assert!(source.starts_with("data:image/png;base64,"));
    }
    #[test]
    fn unaccepted_replacement_followed_by_failure_keeps_original_session() {
        let worker = Worker::start();
        worker.cancellation.open.store(1, Ordering::Relaxed);
        worker
            .commands
            .send(Command::Open {
                request: 1,
                path: fixture(),
            })
            .unwrap();
        assert!(matches!(receive(&worker), Event::Opened { request: 1, .. }));
        worker
            .commands
            .send(Command::Activate { request: 1 })
            .unwrap();
        worker.cancellation.open.store(2, Ordering::Relaxed);
        worker
            .commands
            .send(Command::Open {
                request: 2,
                path: fixture(),
            })
            .unwrap();
        assert!(matches!(receive(&worker), Event::Opened { request: 2, .. }));
        // The GUI has not accepted B when C is requested and fails.
        worker.cancellation.open.store(3, Ordering::Relaxed);
        worker
            .commands
            .send(Command::Open {
                request: 3,
                path: PathBuf::from("/missing/ofd"),
            })
            .unwrap();
        assert!(matches!(
            receive(&worker),
            Event::OpenFailed { request: 3, .. }
        ));
        worker.cancellation.search.store(1, Ordering::Relaxed);
        worker
            .commands
            .send(Command::Search {
                generation: 1,
                request: 1,
                query: String::new(),
            })
            .unwrap();
        assert!(matches!(
            worker.events.recv_timeout(Duration::from_secs(1)),
            Ok(Event::Searched { generation: 1, .. })
        ));
    }
    #[test]
    fn failed_replacement_preserves_document_and_search_still_works() {
        let worker = Worker::start();
        worker.cancellation.open.store(1, Ordering::Relaxed);
        worker
            .commands
            .send(Command::Open {
                request: 1,
                path: fixture(),
            })
            .unwrap();
        match receive(&worker) {
            Event::Opened { pages, .. } => assert_eq!(pages.len(), 1),
            event => panic!("{event:?}"),
        }
        worker
            .commands
            .send(Command::Activate { request: 1 })
            .unwrap();
        worker.cancellation.open.store(2, Ordering::Relaxed);
        worker
            .commands
            .send(Command::Open {
                request: 2,
                path: PathBuf::from("/nonexistent/rofd.ofd"),
            })
            .unwrap();
        assert!(matches!(
            receive(&worker),
            Event::OpenFailed { request: 2, .. }
        ));
        worker.cancellation.search.store(1, Ordering::Relaxed);
        worker
            .commands
            .send(Command::Search {
                generation: 1,
                request: 1,
                query: "不存在的词".into(),
            })
            .unwrap();
        assert!(matches!(
            receive(&worker),
            Event::Searched {
                generation: 1,
                request: 1,
                ..
            }
        ));
    }
    #[test]
    fn stale_open_and_search_are_discarded() {
        let worker = Worker::start();
        worker.cancellation.open.store(2, Ordering::Relaxed);
        worker
            .commands
            .send(Command::Open {
                request: 1,
                path: fixture(),
            })
            .unwrap();
        worker
            .commands
            .send(Command::Open {
                request: 2,
                path: fixture(),
            })
            .unwrap();
        assert!(matches!(receive(&worker), Event::Opened { request: 2, .. }));
        worker
            .commands
            .send(Command::Activate { request: 2 })
            .unwrap();
        worker.cancellation.search.store(2, Ordering::Relaxed);
        for request in 1..=2 {
            worker
                .commands
                .send(Command::Search {
                    generation: 2,
                    request,
                    query: "发票".into(),
                })
                .unwrap();
        }
        assert!(matches!(
            receive(&worker),
            Event::Searched { request: 2, .. }
        ));
    }
}
