use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use serde::de::DeserializeOwned;

use crate::container::Container;
use crate::path::PackagePath;
use crate::raw::{DocumentRoot, OfdRoot};
use crate::{Error, LoadOptions, Result};

/// Descriptive metadata stored in an OFD document.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Metadata {
    /// Document identifier.
    pub document_id: Option<String>,
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject.
    pub subject: Option<String>,
    /// Document abstract.
    pub abstract_text: Option<String>,
    /// Producing application.
    pub creator: Option<String>,
    /// Producing application version.
    pub creator_version: Option<String>,
    /// Original creation date as stored by the producer.
    pub creation_date: Option<String>,
    /// Modification date as stored by the producer.
    pub modification_date: Option<String>,
}

/// Stable categories for recoverable OFD problems.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum WarningCode {
    /// The page omitted Area and inherited the document PageArea.
    PageAreaFallback,
}

/// A recoverable OFD conformance diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Warning {
    /// Machine-readable category.
    pub code: WarningCode,
    /// Path inside the OFD package.
    pub path: String,
    /// Human-readable explanation.
    pub message: String,
}

#[derive(Debug)]
struct PageReference {
    id: u64,
    path: PackagePath,
    cache: OnceLock<Arc<PageData>>,
}

#[derive(Debug)]
struct DocumentInner {
    container: Container,
    limits: crate::ResourceLimits,
    metadata: Metadata,
    default_page_area: crate::raw::PageArea,
    pages: Vec<PageReference>,
    strictness: crate::Strictness,
    warnings: Mutex<Vec<Warning>>,
}

/// A read-only OFD document.
#[derive(Clone, Debug)]
pub struct Document(Arc<DocumentInner>);

#[derive(Debug)]
struct PageData {
    size: crate::Rect,
    layers: Vec<crate::Layer>,
}

/// One parsed page in an OFD document.
#[derive(Clone, Debug)]
pub struct Page {
    _document: Arc<DocumentInner>,
    index: usize,
    object_id: u64,
    data: Arc<PageData>,
}

impl Page {
    /// Returns the zero-based page index.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns the OFD object identifier of the page.
    pub fn object_id(&self) -> u64 {
        self.object_id
    }

    /// Returns the effective physical page box in millimetres.
    pub fn size(&self) -> crate::Rect {
        self.data.size
    }

    /// Returns page layers in source order.
    pub fn layers(&self) -> &[crate::Layer] {
        &self.data.layers
    }
}

impl Document {
    /// Opens an OFD document from a host file path.
    pub fn open(path: impl AsRef<Path>, options: LoadOptions) -> Result<Self> {
        let path = path.as_ref();
        let bytes = fs::read(path).map_err(|source| Error::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::from_bytes(bytes, options)
    }

    /// Opens an OFD document from owned bytes.
    pub fn from_bytes(bytes: Vec<u8>, options: LoadOptions) -> Result<Self> {
        let strictness = options.strictness;
        let limits = options.limits;
        let container = Container::from_bytes(bytes, limits.clone())?;
        let entry_path = PackagePath::new("OFD.xml")?;
        let ofd: OfdRoot = parse_xml(&container, &entry_path)?;
        if ofd.doc_bodies.len() != 1 {
            return Err(Error::UnsupportedFeature(format!(
                "v0.2 requires exactly one DocBody, found {}",
                ofd.doc_bodies.len()
            )));
        }
        let body = ofd
            .doc_bodies
            .into_iter()
            .next()
            .ok_or_else(|| Error::InvalidStructure {
                path: entry_path.as_str().to_owned(),
                message: "DocBody is missing".to_owned(),
            })?;
        let document_path = entry_path.resolve(&body.doc_root)?;
        let root: DocumentRoot = parse_xml(&container, &document_path)?;
        let pages = root
            .pages
            .pages
            .into_iter()
            .map(|page| {
                Ok(PageReference {
                    id: page.id,
                    path: document_path.resolve(&page.base_loc)?,
                    cache: OnceLock::new(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let info = body.doc_info;
        Ok(Self(Arc::new(DocumentInner {
            container,
            limits,
            metadata: Metadata {
                document_id: info.document_id,
                title: info.title,
                author: info.author,
                subject: info.subject,
                abstract_text: info.abstract_,
                creator: info.creator,
                creator_version: info.creator_version,
                creation_date: info.creation_date,
                modification_date: info.mod_date,
            },
            default_page_area: root.common_data.page_area,
            pages,
            strictness,
            warnings: Mutex::new(Vec::new()),
        })))
    }

    /// Returns document metadata.
    pub fn metadata(&self) -> &Metadata {
        &self.0.metadata
    }

    /// Returns the number of indexed pages.
    pub fn page_count(&self) -> usize {
        self.0.pages.len()
    }

    /// Loads and returns a page by zero-based index.
    pub fn page(&self, index: usize) -> Result<Page> {
        let reference = self.0.pages.get(index).ok_or(Error::PageOutOfRange {
            index,
            page_count: self.page_count(),
        })?;
        if let Some(data) = reference.cache.get() {
            return Ok(Page {
                _document: Arc::clone(&self.0),
                index,
                object_id: reference.id,
                data: Arc::clone(data),
            });
        }

        let page: crate::raw::PageRoot = parse_page_xml(
            &self.0.container,
            &reference.path,
            self.0.limits.max_page_block_depth,
        )?;
        let area = match page.area {
            Some(area) => area,
            None if self.0.strictness == crate::Strictness::Strict => {
                return Err(Error::InvalidStructure {
                    path: reference.path.as_str().to_owned(),
                    message: "Page.Area is missing".to_owned(),
                });
            }
            None => {
                self.0
                    .warnings
                    .lock()
                    .map_err(|_| Error::InvalidStructure {
                        path: reference.path.as_str().to_owned(),
                        message: "warning store lock is poisoned".to_owned(),
                    })?
                    .push(Warning {
                        code: WarningCode::PageAreaFallback,
                        path: reference.path.as_str().to_owned(),
                        message: "Page.Area is missing; inherited Document PageArea".to_owned(),
                    });
                self.0.default_page_area.clone()
            }
        };
        let size = crate::Rect::parse(&area.physical_box)?;
        let layers =
            crate::content::convert_layers(page.content, &self.0.limits, reference.path.as_str())?;
        let parsed = Arc::new(PageData { size, layers });
        let data = reference.cache.get_or_init(|| Arc::clone(&parsed));
        Ok(Page {
            _document: Arc::clone(&self.0),
            index,
            object_id: reference.id,
            data: Arc::clone(data),
        })
    }

    /// Returns a snapshot of recoverable diagnostics collected so far.
    pub fn warnings(&self) -> Vec<Warning> {
        self.0
            .warnings
            .lock()
            .map(|warnings| warnings.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }
}

fn parse_xml<T: DeserializeOwned>(container: &Container, path: &PackagePath) -> Result<T> {
    let bytes = container.read(path)?;
    serde_xml_rs::from_reader(bytes.as_slice()).map_err(|error| Error::Xml {
        path: path.as_str().to_owned(),
        message: error.to_string(),
    })
}

fn parse_page_xml<T: DeserializeOwned>(
    container: &Container,
    path: &PackagePath,
    max_page_block_depth: usize,
) -> Result<T> {
    let bytes = container.read(path)?;
    preflight_page_xml(&bytes, path, max_page_block_depth)?;
    serde_xml_rs::from_reader(bytes.as_slice()).map_err(|error| Error::Xml {
        path: path.as_str().to_owned(),
        message: error.to_string(),
    })
}

fn preflight_page_xml(bytes: &[u8], path: &PackagePath, max_page_block_depth: usize) -> Result<()> {
    use xml::reader::{EventReader, XmlEvent};

    let mut elements = Vec::new();
    let mut page_block_depth = 0usize;
    for event in EventReader::new(bytes) {
        match event.map_err(|error| Error::Xml {
            path: path.as_str().to_owned(),
            message: error.to_string(),
        })? {
            XmlEvent::StartElement { name, .. } => {
                if matches!(
                    elements.last().map(String::as_str),
                    Some("Layer" | "PageBlock")
                ) && !matches!(
                    name.local_name.as_str(),
                    "PathObject" | "PageBlock" | "TextObject" | "ImageObject" | "CompositeObject"
                ) {
                    return Err(Error::InvalidStructure {
                        path: path.as_str().to_owned(),
                        message: format!("unknown graphic unit {}", name.local_name),
                    });
                }
                if name.local_name == "PageBlock" {
                    page_block_depth = page_block_depth.saturating_add(1);
                    if page_block_depth > max_page_block_depth {
                        return Err(Error::LimitExceeded(format!(
                            "page block depth {page_block_depth} exceeds limit {max_page_block_depth}"
                        )));
                    }
                }
                elements.push(name.local_name);
            }
            XmlEvent::EndElement { name } => {
                if name.local_name == "PageBlock" {
                    page_block_depth = page_block_depth.saturating_sub(1);
                }
                elements.pop();
            }
            _ => {}
        }
    }
    Ok(())
}
