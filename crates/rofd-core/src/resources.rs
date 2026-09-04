use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use crate::container::Container;
use crate::path::PackagePath;
use crate::raw::{FontEntry, MultiMediaEntry, ResourceRoot};
use crate::{Error, ResourceLimits, Result};

/// A document resource category.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ResourceKind {
    /// A font resource.
    Font,
    /// An encoded raster image resource.
    Image,
}

/// An encoded image format supported by the resource index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ImageFormat {
    /// Portable Network Graphics.
    Png,
    /// JPEG image data.
    Jpeg,
}

/// Immutable metadata and optional embedded bytes for an OFD font resource.
#[derive(Clone, Debug)]
pub struct FontResource {
    id: u64,
    font_name: String,
    family_name: Option<String>,
    charset: Option<String>,
    bytes: Option<Arc<[u8]>>,
}

impl FontResource {
    /// Returns the document-wide OFD object identifier.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the declared font name.
    pub fn font_name(&self) -> &str {
        &self.font_name
    }

    /// Returns the declared font family name, when present.
    pub fn family_name(&self) -> Option<&str> {
        self.family_name.as_deref()
    }

    /// Returns the declared character set, when present.
    pub fn charset(&self) -> Option<&str> {
        self.charset.as_deref()
    }

    /// Returns the bounded encoded font file, when the catalog embeds one.
    pub fn encoded_bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
}

/// Immutable metadata and encoded bytes for an OFD image resource.
#[derive(Clone, Debug)]
pub struct ImageResource {
    id: u64,
    format: ImageFormat,
    bytes: Arc<[u8]>,
}

impl ImageResource {
    /// Returns the document-wide OFD object identifier.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Returns the declared and supported encoded image format.
    pub fn format(&self) -> ImageFormat {
        self.format
    }

    /// Returns the bounded encoded image bytes without decoding them.
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

#[derive(Debug)]
pub(crate) struct ResourceCatalog {
    entries: HashMap<u64, ResourceEntry>,
}

#[derive(Debug)]
enum ResourceEntry {
    Font(FontRecord),
    Image(ImageRecord),
}

#[derive(Debug)]
struct FontRecord {
    id: u64,
    font_name: String,
    family_name: Option<String>,
    charset: Option<String>,
    file: Option<Asset>,
}

#[derive(Debug)]
struct ImageRecord {
    id: u64,
    format: ImageFormat,
    file: Asset,
}

#[derive(Debug)]
struct Asset {
    path: PackagePath,
    bytes: OnceLock<Arc<[u8]>>,
}

impl Asset {
    fn new(path: PackagePath) -> Self {
        Self {
            path,
            bytes: OnceLock::new(),
        }
    }

    fn load(&self, container: &Container, limit: u64, label: &str) -> Result<Arc<[u8]>> {
        if let Some(bytes) = self.bytes.get() {
            return Ok(Arc::clone(bytes));
        }
        let bytes: Arc<[u8]> = container.read_with_limit(&self.path, limit, label)?.into();
        Ok(Arc::clone(self.bytes.get_or_init(|| Arc::clone(&bytes))))
    }
}

impl ResourceCatalog {
    pub(crate) fn empty() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    pub(crate) fn load(
        container: &Container,
        paths: &[PackagePath],
        limits: &ResourceLimits,
    ) -> Result<Self> {
        let mut count = 0usize;
        let mut documents = Vec::with_capacity(paths.len());
        for path in paths {
            let bytes = container.read(path)?;
            preflight(&bytes, path, limits, &mut count)?;
            documents.push((path, bytes));
        }

        let mut catalog = Self::empty();
        for (path, bytes) in documents {
            let root: ResourceRoot =
                serde_xml_rs::from_reader(bytes.as_slice()).map_err(|error| Error::Xml {
                    path: path.as_str().to_owned(),
                    message: error.to_string(),
                })?;
            for font in root.fonts.into_iter().flat_map(|fonts| fonts.entries) {
                catalog.insert_font(font, &root.base_loc, path)?;
            }
            for image in root
                .multi_medias
                .into_iter()
                .flat_map(|images| images.entries)
            {
                catalog.insert_image(image, &root.base_loc, path)?;
            }
        }
        Ok(catalog)
    }

    pub(crate) fn font(&self, id: u64, container: &Container, limit: u64) -> Result<FontResource> {
        match self.entries.get(&id) {
            Some(ResourceEntry::Font(font)) => Ok(FontResource {
                id: font.id,
                font_name: font.font_name.clone(),
                family_name: font.family_name.clone(),
                charset: font.charset.clone(),
                bytes: font
                    .file
                    .as_ref()
                    .map(|file| file.load(container, limit, "font resource"))
                    .transpose()?,
            }),
            Some(ResourceEntry::Image(_)) => {
                Err(kind_mismatch(id, ResourceKind::Font, ResourceKind::Image))
            }
            None => Err(Error::UnknownResource { object_id: id }),
        }
    }

    pub(crate) fn image(
        &self,
        id: u64,
        container: &Container,
        limit: u64,
    ) -> Result<ImageResource> {
        match self.entries.get(&id) {
            Some(ResourceEntry::Image(image)) => Ok(ImageResource {
                id: image.id,
                format: image.format,
                bytes: image.file.load(container, limit, "image resource")?,
            }),
            Some(ResourceEntry::Font(_)) => {
                Err(kind_mismatch(id, ResourceKind::Image, ResourceKind::Font))
            }
            None => Err(Error::UnknownResource { object_id: id }),
        }
    }

    fn insert_font(
        &mut self,
        entry: FontEntry,
        base_loc: &Option<String>,
        catalog_path: &PackagePath,
    ) -> Result<()> {
        let id = parse_id(&entry.id, catalog_path)?;
        let font_name = required(entry.font_name, "FontName", id, catalog_path)?;
        let file = entry
            .font_file
            .map(|value| {
                required(Some(value), "FontFile", id, catalog_path).and_then(|value| {
                    asset_path(catalog_path, base_loc.as_deref(), &value).map(Asset::new)
                })
            })
            .transpose()?;
        self.insert(
            id,
            ResourceEntry::Font(FontRecord {
                id,
                font_name,
                family_name: entry.family_name,
                charset: entry.charset,
                file,
            }),
            catalog_path,
        )
    }

    fn insert_image(
        &mut self,
        entry: MultiMediaEntry,
        base_loc: &Option<String>,
        catalog_path: &PackagePath,
    ) -> Result<()> {
        let id = parse_id(&entry.id, catalog_path)?;
        let kind = required(entry.kind, "Type", id, catalog_path)?;
        if !kind.eq_ignore_ascii_case("image") {
            return Err(invalid_resource(
                catalog_path,
                Some(id),
                "Type",
                format!("expected Image, found {kind}"),
            ));
        }
        let raw_format = required(entry.format, "Format", id, catalog_path)?;
        let format = match raw_format.to_ascii_lowercase().as_str() {
            "png" => ImageFormat::Png,
            "jpg" | "jpeg" => ImageFormat::Jpeg,
            _ => {
                return Err(invalid_resource(
                    catalog_path,
                    Some(id),
                    "Format",
                    format!("unsupported encoded image format {raw_format}"),
                ))
            }
        };
        let media_file = required(entry.media_file, "MediaFile", id, catalog_path)?;
        let path = asset_path(catalog_path, base_loc.as_deref(), &media_file)?;
        self.insert(
            id,
            ResourceEntry::Image(ImageRecord {
                id,
                format,
                file: Asset::new(path),
            }),
            catalog_path,
        )
    }

    fn insert(&mut self, id: u64, entry: ResourceEntry, path: &PackagePath) -> Result<()> {
        if self.entries.insert(id, entry).is_some() {
            return Err(Error::InvalidStructure {
                path: path.as_str().to_owned(),
                message: format!("duplicate resource ID {id}"),
            });
        }
        Ok(())
    }
}

fn preflight(
    bytes: &[u8],
    path: &PackagePath,
    limits: &ResourceLimits,
    total: &mut usize,
) -> Result<()> {
    use xml::reader::{EventReader, XmlEvent};

    #[derive(Clone, Copy)]
    enum Marker {
        Res,
        Fonts,
        MultiMedias,
        Other,
    }

    let mut elements = Vec::new();
    for event in EventReader::new(bytes) {
        match event.map_err(|error| Error::Xml {
            path: path.as_str().to_owned(),
            message: error.to_string(),
        })? {
            XmlEvent::StartElement { name, .. } => {
                let depth = elements.len().checked_add(1).ok_or_else(|| {
                    Error::LimitExceeded("resource XML depth overflow".to_owned())
                })?;
                if depth > limits.max_xml_depth {
                    return Err(Error::LimitExceeded(format!(
                        "XML depth {depth} exceeds limit {}",
                        limits.max_xml_depth
                    )));
                }
                if elements.is_empty() && name.local_name != "Res" {
                    return Err(Error::InvalidStructure {
                        path: path.as_str().to_owned(),
                        message: format!(
                            "resource catalog root must be Res, found {}",
                            name.local_name
                        ),
                    });
                }
                let parent = elements.last().copied();
                let is_res = elements.is_empty() && name.local_name == "Res";
                let is_fonts = matches!(parent, Some(Marker::Res)) && name.local_name == "Fonts";
                let is_multi_medias =
                    matches!(parent, Some(Marker::Res)) && name.local_name == "MultiMedias";
                let is_resource = (matches!(parent, Some(Marker::Fonts))
                    && name.local_name == "Font")
                    || (matches!(parent, Some(Marker::MultiMedias))
                        && name.local_name == "MultiMedia");
                if is_resource {
                    *total = total.checked_add(1).ok_or_else(|| {
                        Error::LimitExceeded("resource count overflow".to_owned())
                    })?;
                    if *total > limits.max_resources {
                        return Err(Error::LimitExceeded(format!(
                            "resource count {} exceeds limit {}",
                            *total, limits.max_resources
                        )));
                    }
                }
                elements.push(if is_res {
                    Marker::Res
                } else if is_fonts {
                    Marker::Fonts
                } else if is_multi_medias {
                    Marker::MultiMedias
                } else {
                    Marker::Other
                });
            }
            XmlEvent::EndElement { .. } => {
                elements.pop();
            }
            _ => {}
        }
    }
    Ok(())
}

fn parse_id(value: &str, path: &PackagePath) -> Result<u64> {
    let id = value.parse::<u64>().map_err(|_| Error::InvalidValue {
        field: "resource ID",
        value: value.to_owned(),
        path: Some(path.as_str().to_owned()),
    })?;
    if id == 0 {
        return Err(Error::InvalidValue {
            field: "resource ID",
            value: value.to_owned(),
            path: Some(path.as_str().to_owned()),
        });
    }
    Ok(id)
}

fn required(
    value: Option<String>,
    field: &'static str,
    id: u64,
    path: &PackagePath,
) -> Result<String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            invalid_resource(
                path,
                Some(id),
                field,
                "required value is missing".to_owned(),
            )
        })
}

fn asset_path(path: &PackagePath, base_loc: Option<&str>, value: &str) -> Result<PackagePath> {
    let relative = match base_loc {
        Some(base) if !base.is_empty() => format!("{base}/{value}"),
        _ => value.to_owned(),
    };
    path.resolve(&relative).map_err(|error| match error {
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
    })
}

fn invalid_resource(
    path: &PackagePath,
    object_id: Option<u64>,
    field: &'static str,
    message: String,
) -> Error {
    Error::InvalidResource {
        path: path.as_str().to_owned(),
        object_id,
        field,
        message,
    }
}

fn kind_mismatch(id: u64, expected: ResourceKind, actual: ResourceKind) -> Error {
    Error::ResourceKindMismatch {
        object_id: id,
        expected,
        actual,
    }
}
