use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use serde::de::DeserializeOwned;

use crate::container::Container;
use crate::path::PackagePath;
use crate::raw::{CommonData, DocumentRoot, OfdRoot};
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
    initialization: Mutex<()>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TemplateZOrder {
    Background,
    Foreground,
}

#[derive(Debug)]
struct TemplateReference {
    id: u64,
    path: PackagePath,
    default_z_order: TemplateZOrder,
    cache: OnceLock<Arc<TemplateData>>,
}

#[derive(Debug)]
struct DocumentInner {
    container: Container,
    document_path: PackagePath,
    limits: crate::ResourceLimits,
    metadata: Metadata,
    default_page_area: crate::raw::PageArea,
    pages: Vec<PageReference>,
    templates: HashMap<u64, TemplateReference>,
    strictness: crate::Strictness,
    warnings: Mutex<Vec<Warning>>,
    resource_paths: Vec<PackagePath>,
    resource_catalog: OnceLock<Arc<crate::resources::ResourceCatalog>>,
    resource_initialization: Mutex<()>,
}

/// A read-only OFD document.
#[derive(Clone, Debug)]
pub struct Document(Arc<DocumentInner>);

#[derive(Debug)]
struct PageData {
    size: crate::Rect,
    layers: Vec<crate::Layer>,
}

#[derive(Debug)]
struct TemplateData {
    layers: Vec<crate::Layer>,
    usage: crate::content::ContentUsage,
    referenced_templates: Vec<u64>,
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

    /// Returns immutable layers in effective template/page paint order.
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
        let ofd: OfdRoot = parse_xml(&container, &entry_path, limits.max_xml_depth)?;
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
        let root: DocumentRoot = parse_xml(&container, &document_path, limits.max_xml_depth)?;
        let CommonData {
            page_area,
            public_res,
            document_res,
            template_pages,
        } = root.common_data;
        let mut resource_paths = Vec::new();
        for declaration in [public_res, document_res].into_iter().flatten() {
            if resource_paths.len() >= limits.max_resource_files {
                return Err(Error::LimitExceeded(format!(
                    "resource file count {} exceeds limit {}",
                    resource_paths.len().saturating_add(1),
                    limits.max_resource_files
                )));
            }
            resource_paths.push(
                document_path
                    .resolve(&declaration)
                    .map_err(|error| with_error_path(error, &document_path))?,
            );
        }
        let pages = root
            .pages
            .pages
            .into_iter()
            .map(|page| {
                Ok(PageReference {
                    id: page.id,
                    path: document_path.resolve(&page.base_loc)?,
                    cache: OnceLock::new(),
                    initialization: Mutex::new(()),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut templates = HashMap::new();
        for template in template_pages {
            let id = parse_template_id(&template.id, &document_path)?;
            if templates.contains_key(&id) {
                return Err(Error::InvalidStructure {
                    path: document_path.as_str().to_owned(),
                    message: format!("duplicate template ID {id}"),
                });
            }
            let path = document_path
                .resolve(&template.base_loc)
                .map_err(|error| with_error_path(error, &document_path))?;
            let default_z_order = parse_template_z_order(
                template.z_order.as_deref(),
                TemplateZOrder::Background,
                &document_path,
            )?;
            templates.insert(
                id,
                TemplateReference {
                    id,
                    path,
                    default_z_order,
                    cache: OnceLock::new(),
                },
            );
        }
        let info = body.doc_info;
        Ok(Self(Arc::new(DocumentInner {
            container,
            document_path,
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
            default_page_area: page_area,
            pages,
            templates,
            strictness,
            warnings: Mutex::new(Vec::new()),
            resource_paths,
            resource_catalog: OnceLock::new(),
            resource_initialization: Mutex::new(()),
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

    /// Looks up a font resource after atomically validating all declared catalogs.
    ///
    /// An optional embedded font file is read only when its font is requested
    /// and is bounded by [`crate::ResourceLimits::max_font_bytes`].
    pub fn font_resource(&self, object_id: u64) -> Result<crate::FontResource> {
        self.resource_catalog()?
            .font(object_id, &self.0.container, self.0.limits.max_font_bytes)
    }

    /// Looks up an image resource after atomically validating all declared catalogs.
    ///
    /// Encoded bytes are read only when the image is requested and are bounded
    /// by [`crate::ResourceLimits::max_encoded_image_bytes`].
    pub fn image_resource(&self, object_id: u64) -> Result<crate::ImageResource> {
        self.resource_catalog()?.image(
            object_id,
            &self.0.container,
            self.0.limits.max_encoded_image_bytes,
        )
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

        let _initialization =
            reference
                .initialization
                .lock()
                .map_err(|_| Error::InvalidStructure {
                    path: reference.path.as_str().to_owned(),
                    message: "page initialization lock is poisoned".to_owned(),
                })?;
        if let Some(data) = reference.cache.get() {
            return Ok(Page {
                _document: Arc::clone(&self.0),
                index,
                object_id: reference.id,
                data: Arc::clone(data),
            });
        }

        let page: crate::raw::PageRoot =
            parse_page_xml(&self.0.container, &reference.path, &self.0.limits)?;
        let (area, pending_warning) = match page.area {
            Some(area) => (area, None),
            None if self.0.strictness == crate::Strictness::Strict => {
                return Err(Error::InvalidStructure {
                    path: reference.path.as_str().to_owned(),
                    message: "Page.Area is missing".to_owned(),
                });
            }
            None => (
                self.0.default_page_area.clone(),
                Some(Warning {
                    code: WarningCode::PageAreaFallback,
                    path: reference.path.as_str().to_owned(),
                    message: "Page.Area is missing; inherited Document PageArea".to_owned(),
                }),
            ),
        };
        let size = crate::Rect::parse(&area.physical_box)
            .map_err(|error| with_error_path(error, &reference.path))?;
        let (direct_layers, direct_usage) = crate::content::convert_layers(
            page.content,
            &self.0.limits,
            reference.path.as_str(),
            crate::LayerSource::Page,
        )
        .map_err(|error| with_error_path(error, &reference.path))?;
        let layers = self
            .merge_effective_layers(
                page.templates,
                direct_layers,
                direct_usage,
                &reference.path,
                &mut Vec::new(),
            )?
            .layers;
        let parsed = Arc::new(PageData { size, layers });
        if let Some(warning) = pending_warning {
            self.0
                .warnings
                .lock()
                .map_err(|_| Error::InvalidStructure {
                    path: reference.path.as_str().to_owned(),
                    message: "warning store lock is poisoned".to_owned(),
                })?
                .push(warning);
        }
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

    fn resource_catalog(&self) -> Result<&Arc<crate::resources::ResourceCatalog>> {
        if let Some(catalog) = self.0.resource_catalog.get() {
            return Ok(catalog);
        }
        let _initialization =
            self.0
                .resource_initialization
                .lock()
                .map_err(|_| Error::InvalidStructure {
                    path: self.0.document_path.as_str().to_owned(),
                    message: "resource catalog initialization lock is poisoned".to_owned(),
                })?;
        if let Some(catalog) = self.0.resource_catalog.get() {
            return Ok(catalog);
        }
        let parsed = Arc::new(crate::resources::ResourceCatalog::load(
            &self.0.container,
            &self.0.resource_paths,
            &self.0.limits,
        )?);
        Ok(self.0.resource_catalog.get_or_init(|| Arc::clone(&parsed)))
    }

    fn resolve_template(
        &self,
        id: u64,
        referring_path: &PackagePath,
        active: &mut Vec<u64>,
    ) -> Result<Arc<TemplateData>> {
        if let Some(position) = active.iter().position(|active_id| *active_id == id) {
            let mut cycle = active[position..]
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>();
            cycle.push(id.to_string());
            return Err(Error::InvalidStructure {
                path: referring_path.as_str().to_owned(),
                message: format!("template reference cycle: {}", cycle.join(" -> ")),
            });
        }
        let reference = self
            .0
            .templates
            .get(&id)
            .ok_or_else(|| Error::InvalidStructure {
                path: referring_path.as_str().to_owned(),
                message: format!("unknown template ID {id}"),
            })?;
        debug_assert_eq!(reference.id, id);

        active.push(id);
        let loaded = (|| {
            if active.len() > self.0.limits.max_page_block_depth {
                return Err(Error::LimitExceeded(format!(
                    "template reference depth {} exceeds limit {}",
                    active.len(),
                    self.0.limits.max_page_block_depth
                )));
            }
            if let Some(data) = reference.cache.get() {
                for referenced_id in &data.referenced_templates {
                    self.resolve_template(*referenced_id, &reference.path, active)?;
                }
                return Ok(Arc::clone(data));
            }
            let root: crate::raw::PageRoot =
                parse_page_xml(&self.0.container, &reference.path, &self.0.limits)?;
            let (direct_layers, direct_usage) = crate::content::convert_layers(
                root.content,
                &self.0.limits,
                reference.path.as_str(),
                crate::LayerSource::Template(id),
            )
            .map_err(|error| with_error_path(error, &reference.path))?;
            self.merge_effective_layers(
                root.templates,
                direct_layers,
                direct_usage,
                &reference.path,
                active,
            )
            .map(Arc::new)
        })();
        active.pop();
        let loaded = loaded?;
        Ok(Arc::clone(
            reference.cache.get_or_init(|| Arc::clone(&loaded)),
        ))
    }

    fn merge_effective_layers(
        &self,
        template_references: Vec<crate::raw::TemplateReference>,
        direct_layers: Vec<crate::Layer>,
        mut usage: crate::content::ContentUsage,
        path: &PackagePath,
        active: &mut Vec<u64>,
    ) -> Result<TemplateData> {
        let mut background_templates = Vec::new();
        let mut foreground_templates = Vec::new();
        let mut referenced_templates = Vec::with_capacity(template_references.len());
        for template in template_references {
            usage = add_effective_usage(
                usage,
                crate::content::ContentUsage::template_reference(),
                &self.0.limits,
            )?;
            let id = parse_template_id(&template.template_id, path)?;
            referenced_templates.push(id);
            let declaration = self
                .0
                .templates
                .get(&id)
                .ok_or_else(|| Error::InvalidStructure {
                    path: path.as_str().to_owned(),
                    message: format!("unknown template ID {id}"),
                })?;
            let z_order = parse_template_z_order(
                template.z_order.as_deref(),
                declaration.default_z_order,
                path,
            )?;
            let resolved = self.resolve_template(id, path, active)?;
            usage = add_effective_usage(usage, resolved.usage, &self.0.limits)?;
            match z_order {
                TemplateZOrder::Background => {
                    background_templates.extend(resolved.layers.iter().cloned())
                }
                TemplateZOrder::Foreground => {
                    foreground_templates.extend(resolved.layers.iter().cloned())
                }
            }
        }

        validate_effective_usage(usage, &self.0.limits)?;
        let mut background = Vec::new();
        let mut body = Vec::new();
        let mut foreground = Vec::new();
        for layer in direct_layers {
            match layer.kind() {
                crate::LayerType::Background => background.push(layer),
                crate::LayerType::Body => body.push(layer),
                crate::LayerType::Foreground => foreground.push(layer),
            }
        }
        background_templates.extend(background);
        background_templates.extend(body);
        background_templates.extend(foreground);
        background_templates.extend(foreground_templates);
        Ok(TemplateData {
            layers: background_templates,
            usage,
            referenced_templates,
        })
    }
}

fn parse_template_id(value: &str, path: &PackagePath) -> Result<u64> {
    match value.parse::<u64>() {
        Ok(id) if id != 0 => Ok(id),
        _ => Err(Error::InvalidValue {
            field: "template ID",
            value: value.to_owned(),
            path: Some(path.as_str().to_owned()),
        }),
    }
}

fn parse_template_z_order(
    value: Option<&str>,
    default: TemplateZOrder,
    path: &PackagePath,
) -> Result<TemplateZOrder> {
    match value {
        None => Ok(default),
        Some("Background") => Ok(TemplateZOrder::Background),
        Some("Foreground") => Ok(TemplateZOrder::Foreground),
        Some(value) => Err(Error::InvalidValue {
            field: "template ZOrder",
            value: value.to_owned(),
            path: Some(path.as_str().to_owned()),
        }),
    }
}

fn add_effective_usage(
    current: crate::content::ContentUsage,
    added: crate::content::ContentUsage,
    limits: &crate::ResourceLimits,
) -> Result<crate::content::ContentUsage> {
    let total = current.checked_add(added).ok_or_else(|| {
        Error::LimitExceeded("effective page resource accounting overflowed".to_owned())
    })?;
    validate_effective_usage(total, limits)?;
    Ok(total)
}

fn validate_effective_usage(
    usage: crate::content::ContentUsage,
    limits: &crate::ResourceLimits,
) -> Result<()> {
    if usage.page_objects > limits.max_page_objects {
        return Err(Error::LimitExceeded(format!(
            "effective page object count {} exceeds limit {}",
            usage.page_objects, limits.max_page_objects
        )));
    }
    if usage.path_commands > limits.max_path_commands {
        return Err(Error::LimitExceeded(format!(
            "effective page path command count {} exceeds limit {}",
            usage.path_commands, limits.max_path_commands
        )));
    }
    Ok(())
}

fn with_error_path(error: Error, path: &PackagePath) -> Error {
    match error {
        Error::InvalidValue {
            field,
            value,
            path: None,
        } => Error::InvalidValue {
            field,
            value,
            path: Some(path.as_str().to_owned()),
        },
        error => error,
    }
}

fn parse_xml<T: DeserializeOwned>(
    container: &Container,
    path: &PackagePath,
    max_xml_depth: usize,
) -> Result<T> {
    let bytes = container.read(path)?;
    preflight_xml_depth(&bytes, path, max_xml_depth)?;
    serde_xml_rs::from_reader(bytes.as_slice()).map_err(|error| Error::Xml {
        path: path.as_str().to_owned(),
        message: error.to_string(),
    })
}

fn parse_page_xml<T: DeserializeOwned>(
    container: &Container,
    path: &PackagePath,
    limits: &crate::ResourceLimits,
) -> Result<T> {
    let bytes = container.read(path)?;
    preflight_page_xml(&bytes, path, limits)?;
    serde_xml_rs::from_reader(bytes.as_slice()).map_err(|error| Error::Xml {
        path: path.as_str().to_owned(),
        message: error.to_string(),
    })
}

fn preflight_xml_depth(bytes: &[u8], path: &PackagePath, max_xml_depth: usize) -> Result<()> {
    use xml::reader::{EventReader, XmlEvent};

    let mut depth = 0usize;
    for event in EventReader::new(bytes) {
        match event.map_err(|error| xml_error(path, error))? {
            XmlEvent::StartElement { .. } => {
                depth = depth.saturating_add(1);
                if depth > max_xml_depth {
                    return Err(xml_depth_error(depth, max_xml_depth));
                }
            }
            XmlEvent::EndElement { .. } => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

fn preflight_page_xml(
    bytes: &[u8],
    path: &PackagePath,
    limits: &crate::ResourceLimits,
) -> Result<()> {
    use xml::reader::{EventReader, XmlEvent};

    #[derive(Clone, Copy)]
    enum ElementMarker {
        Page,
        Content,
        Layer,
        PageBlock,
        PathObject,
        Clips,
        Clip,
        ClipArea,
        Other,
    }

    let mut elements: Vec<ElementMarker> = Vec::new();
    let mut page_block_depth = 0usize;
    let mut page_object_count = 0usize;
    for event in EventReader::new(bytes) {
        match event.map_err(|error| xml_error(path, error))? {
            XmlEvent::StartElement { name, .. } => {
                let depth = elements.len().saturating_add(1);
                if depth > limits.max_xml_depth {
                    return Err(xml_depth_error(depth, limits.max_xml_depth));
                }
                let parent = elements.last().copied();
                let is_page = elements.is_empty() && name.local_name == "Page";
                let is_content =
                    matches!(parent, Some(ElementMarker::Page)) && name.local_name == "Content";
                let is_template_reference =
                    matches!(parent, Some(ElementMarker::Page)) && name.local_name == "Template";
                let is_layer =
                    matches!(parent, Some(ElementMarker::Content)) && name.local_name == "Layer";
                let parent_is_object_container = matches!(
                    parent,
                    Some(ElementMarker::Layer | ElementMarker::PageBlock)
                );
                let is_graphic_unit = parent_is_object_container
                    && matches!(
                        name.local_name.as_str(),
                        "PathObject"
                            | "PageBlock"
                            | "TextObject"
                            | "ImageObject"
                            | "CompositeObject"
                    );
                let is_clips =
                    matches!(parent, Some(ElementMarker::PathObject)) && name.local_name == "Clips";
                let is_clip =
                    matches!(parent, Some(ElementMarker::Clips)) && name.local_name == "Clip";
                let is_clip_area =
                    matches!(parent, Some(ElementMarker::Clip)) && name.local_name == "Area";
                let is_clip_child = matches!(parent, Some(ElementMarker::ClipArea))
                    && matches!(name.local_name.as_str(), "Path" | "Text");
                if parent_is_object_container && !is_graphic_unit {
                    return Err(Error::InvalidStructure {
                        path: path.as_str().to_owned(),
                        message: format!("unknown graphic unit {}", name.local_name),
                    });
                }
                if is_template_reference
                    || is_layer
                    || is_graphic_unit
                    || is_clip
                    || is_clip_area
                    || is_clip_child
                {
                    if page_object_count >= limits.max_page_objects {
                        return Err(Error::LimitExceeded(format!(
                            "page object count {} exceeds limit {}",
                            page_object_count.saturating_add(1),
                            limits.max_page_objects
                        )));
                    }
                    page_object_count += 1;
                }
                let is_page_block = is_graphic_unit && name.local_name == "PageBlock";
                if is_page_block {
                    page_block_depth = page_block_depth.saturating_add(1);
                    if page_block_depth > limits.max_page_block_depth {
                        return Err(Error::LimitExceeded(format!(
                            "page block depth {page_block_depth} exceeds limit {}",
                            limits.max_page_block_depth
                        )));
                    }
                }
                elements.push(match () {
                    _ if is_page => ElementMarker::Page,
                    _ if is_content => ElementMarker::Content,
                    _ if is_layer => ElementMarker::Layer,
                    _ if is_page_block => ElementMarker::PageBlock,
                    _ if is_graphic_unit && name.local_name == "PathObject" => {
                        ElementMarker::PathObject
                    }
                    _ if is_clips => ElementMarker::Clips,
                    _ if is_clip => ElementMarker::Clip,
                    _ if is_clip_area => ElementMarker::ClipArea,
                    _ => ElementMarker::Other,
                });
            }
            XmlEvent::EndElement { .. } => {
                if matches!(elements.pop(), Some(ElementMarker::PageBlock)) {
                    page_block_depth = page_block_depth.saturating_sub(1);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

fn xml_error(path: &PackagePath, error: xml::reader::Error) -> Error {
    Error::Xml {
        path: path.as_str().to_owned(),
        message: error.to_string(),
    }
}

fn xml_depth_error(depth: usize, limit: usize) -> Error {
    Error::LimitExceeded(format!("XML depth {depth} exceeds limit {limit}"))
}
