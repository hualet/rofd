//! Qt-facing state. Worker threads never access QObjects.
// qmetaobject 0.2 emits this conversion in generated sibling impls.
#![allow(clippy::useless_transmute)]
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use qmetaobject::prelude::*;
use serde_json::json;

use crate::cache::RenderKey;
use crate::worker::{Command, Event, Worker};

#[derive(Default, QObject)]
pub struct OfdViewer {
    base: qt_base_class!(trait QObject),
    pages_json: qt_property!(QString; NOTIFY pages_json_changed),
    pages_json_changed: qt_signal!(),
    document_title: qt_property!(QString; NOTIFY document_title_changed),
    document_title_changed: qt_signal!(),
    page_count: qt_property!(i32; NOTIFY page_count_changed),
    page_count_changed: qt_signal!(),
    current_page: qt_property!(i32; WRITE set_current_page NOTIFY current_page_changed),
    current_page_changed: qt_signal!(),
    generation: qt_property!(i32; NOTIFY generation_changed),
    generation_changed: qt_signal!(),
    busy: qt_property!(bool; NOTIFY busy_changed),
    busy_changed: qt_signal!(),
    error_message: qt_property!(QString; NOTIFY error_message_changed),
    error_message_changed: qt_signal!(),
    search_json: qt_property!(QString; NOTIFY search_json_changed),
    search_json_changed: qt_signal!(),
    search_status: qt_property!(QString; NOTIFY search_status_changed),
    search_status_changed: qt_signal!(),
    search_busy: qt_property!(bool; NOTIFY search_busy_changed),
    search_busy_changed: qt_signal!(),
    render_ready: qt_signal!(payload: QString),
    open_file: qt_method!(fn(&mut self, url: QString)),
    poll: qt_method!(fn(&mut self)),
    request_page: qt_method!(fn(&mut self, page: i32, scale: f64, thumbnail: bool) -> i32),
    cancel_render: qt_method!(fn(&mut self, request: i32)),
    search: qt_method!(fn(&mut self, query: QString)),
    clear_error: qt_method!(fn(&mut self)),
    worker: Option<Worker>,
    open_request: u64,
    search_request: u64,
    active_generation: u64,
    render_request: i32,
}

impl OfdViewer {
    pub fn new() -> Self {
        Self {
            pages_json: "[]".into(),
            search_json: "[]".into(),
            search_status: "输入文字搜索文档".into(),
            ..Self::default()
        }
    }

    pub fn open_file(&mut self, url: QString) {
        let path = match local_path_from_url(&url.to_string()) {
            Ok(path) => path,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };
        self.open_request += 1;
        let request = self.open_request;
        let worker = self.worker.get_or_insert_with(Worker::start);
        worker.cancellation.open.store(request, Ordering::Relaxed);
        let sent = worker
            .commands
            .send(Command::Open { request, path })
            .is_ok();
        self.busy = sent;
        self.busy_changed();
        self.clear_error();
        if !sent {
            self.set_error("后台阅读任务已停止，请重新启动阅读器".into());
        }
    }

    fn set_current_page(&mut self, page: i32) {
        let page = page.clamp(0, (self.page_count - 1).max(0));
        if self.current_page != page {
            self.current_page = page;
            self.current_page_changed();
        }
    }

    fn request_page(&mut self, page: i32, scale: f64, thumbnail: bool) -> i32 {
        if page < 0 || page >= self.page_count || !scale.is_finite() || scale <= 0. {
            return 0;
        }
        self.render_request = self.render_request.wrapping_add(1).max(1);
        if let Some(worker) = &self.worker {
            let key = RenderKey::new(page as usize, scale, thumbnail);
            worker.request_render(self.active_generation, key, self.render_request);
        }
        self.render_request
    }
    fn cancel_render(&mut self, request: i32) {
        if let Some(worker) = &self.worker {
            worker.cancel_render(request);
        }
    }

    fn search(&mut self, query: QString) {
        self.search_request += 1;
        let query = query.to_string();
        self.search_json = "[]".into();
        self.search_json_changed();
        self.search_busy = !query.is_empty() && self.page_count > 0;
        self.search_busy_changed();
        self.search_status = if self.search_busy {
            "正在搜索…"
        } else {
            "输入文字搜索文档"
        }
        .into();
        self.search_status_changed();
        if let Some(worker) = &self.worker {
            worker
                .cancellation
                .search
                .store(self.search_request, Ordering::Relaxed);
            let _ = worker.commands.send(Command::Search {
                generation: self.active_generation,
                request: self.search_request,
                query,
            });
        }
    }

    fn clear_error(&mut self) {
        self.set_error(String::new());
    }
    fn set_error(&mut self, message: String) {
        self.error_message = message.into();
        self.error_message_changed();
    }

    fn poll(&mut self) {
        // Bound work per timer tick, including when the viewport generates many requests.
        for _ in 0..32 {
            let event = self
                .worker
                .as_ref()
                .and_then(|worker| worker.events.try_recv().ok());
            let Some(event) = event else {
                break;
            };
            self.apply_event(event);
        }
    }

    fn apply_event(&mut self, event: Event) {
        match event {
            Event::Opened {
                request,
                title,
                pages,
            } if request == self.open_request => {
                if let Some(worker) = &self.worker {
                    let _ = worker.commands.send(Command::Activate { request });
                }
                self.active_generation = request;
                self.page_count = pages.len() as i32;
                self.pages_json = serde_json::to_string(&pages)
                    .expect("finite page sizes")
                    .into();
                self.document_title = title.into();
                self.current_page = 0;
                self.busy = false;
                self.generation = self.generation.wrapping_add(1);
                self.pages_json_changed();
                self.page_count_changed();
                self.document_title_changed();
                self.current_page_changed();
                self.busy_changed();
                self.search("".into());
                self.clear_error();
                self.generation_changed();
            }
            Event::OpenFailed { request, error } if request == self.open_request => {
                self.busy = false;
                self.busy_changed();
                self.set_error(format!("无法打开文档：{error}"));
            }
            Event::Rendered {
                generation,
                key,
                source,
                error,
                reduced,
            } if generation == self.active_generation => {
                let source = source.as_deref().map(String::as_str).unwrap_or_default();
                self.render_ready(json!({ "page": key.page, "scale": key.scale(),
                    "thumbnail": key.thumbnail, "source": source, "error": error, "reduced": reduced }).to_string().into());
            }
            Event::Searched {
                generation,
                request,
                hits,
                status,
            } if generation == self.active_generation && request == self.search_request => {
                self.search_json = serde_json::to_string(&hits)
                    .expect("finite search rectangles")
                    .into();
                self.search_status = status.into();
                self.search_busy = false;
                self.search_json_changed();
                self.search_status_changed();
                self.search_busy_changed();
            }
            _ => {}
        }
    }
}

fn local_path_from_url(input: &str) -> Result<PathBuf, String> {
    let Some(url) = input.strip_prefix("file://") else {
        if input.contains("://") {
            return Err("请选择本地 OFD 文件".into());
        }
        return Ok(PathBuf::from(input));
    };
    let url = url
        .strip_prefix("localhost/")
        .map(|path| format!("/{path}"))
        .unwrap_or_else(|| url.to_owned());
    if !url.starts_with('/') {
        return Err("不支持远程文件地址".into());
    }
    let bytes = url.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let pair = bytes
                .get(index + 1..index + 3)
                .ok_or("文件地址编码不完整")?;
            let hex = std::str::from_utf8(pair).map_err(|_| "文件地址编码无效")?;
            decoded.push(u8::from_str_radix(hex, 16).map_err(|_| "文件地址编码无效")?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded)
        .map(PathBuf::from)
        .map_err(|_| "文件地址不是有效 UTF-8".into())
}

#[cfg(test)]
fn file_url(path: &std::path::Path) -> String {
    let mut url = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            url.push(byte as char);
        } else {
            url.push_str(&format!("%{byte:02X}"));
        }
    }
    url
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_with_literal_percent_and_file_url_reserved_characters_round_trip() {
        let path = PathBuf::from("/tmp/中文 100% #?.ofd");
        assert_eq!(local_path_from_url(path.to_str().unwrap()).unwrap(), path);
        assert_eq!(local_path_from_url(&file_url(&path)).unwrap(), path);
        assert!(local_path_from_url("file://remote/a.ofd").is_err());
        assert!(local_path_from_url("file:///a%ZZ.ofd").is_err());
    }
    #[test]
    fn page_navigation_is_bounded_and_stale_events_do_not_change_state() {
        let mut viewer = OfdViewer::new();
        viewer.page_count = 3;
        viewer.set_current_page(100);
        assert_eq!(viewer.current_page, 2);
        viewer.set_current_page(-1);
        assert_eq!(viewer.current_page, 0);
        viewer.open_request = 2;
        viewer.apply_event(Event::OpenFailed {
            request: 1,
            error: "stale".into(),
        });
        assert!(viewer.error_message.to_string().is_empty());
        viewer.search_request = 4;
        viewer.apply_event(Event::Searched {
            generation: 0,
            request: 3,
            hits: vec![],
            status: "stale".into(),
        });
        assert_ne!(viewer.search_status.to_string(), "stale");
    }
}
