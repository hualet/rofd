use std::collections::HashMap;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use fontdb::{Database, Family, Query, Source, ID};
use freetype::face::LoadFlag;
use rofd_core::{FontResource, ResourceLimits, TextObject, Transform};

use crate::{Error, Result};

/// Describes where a selected face came from without exposing host paths.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum FontSource {
    /// Bytes embedded by the OFD document.
    Embedded {
        /// Document-wide font resource identifier.
        resource_id: u64,
    },
    /// A face in the resolver's configured system-font snapshot.
    System {
        /// Stable resolver-local, non-path identity.
        identity: String,
    },
    /// A face selected from the ordered configured fallback families.
    ConfiguredFallback {
        /// Stable resolver-local, non-path identity.
        identity: String,
    },
    /// No usable face or visible replacement glyph was available.
    Missing,
}

/// Immutable validated font bytes and selected collection face.
#[derive(Clone, Debug)]
pub struct ResolvedFont {
    bytes: Arc<[u8]>,
    face_index: u32,
    identity: Arc<str>,
    source: FontSource,
    family_name: Option<Arc<str>>,
    glyph_count: u32,
    units_per_em: u16,
    metrics: Arc<Mutex<FontMetricsCache>>,
}

#[derive(Debug, Default)]
struct FontMetricsCache {
    characters: HashMap<char, Option<u32>>,
    advances: HashMap<u32, (i64, i64)>,
}

impl ResolvedFont {
    /// Validates owned font bytes and constructs one backend-neutral face description.
    pub fn from_bytes(
        bytes: Arc<[u8]>,
        face_index: u32,
        identity: String,
        source: FontSource,
    ) -> Result<Self> {
        if identity.is_empty() {
            return Err(Error::InvalidFont {
                identity: "unnamed".to_owned(),
                message: "stable identity must not be empty".to_owned(),
            });
        }
        let face_index_value = isize::try_from(face_index).map_err(|_| Error::InvalidFont {
            identity: identity.clone(),
            message: "face index is not representable".to_owned(),
        })?;
        let library = freetype::Library::init().map_err(|error| Error::InvalidFont {
            identity: identity.clone(),
            message: error.to_string(),
        })?;
        let face = library
            .new_memory_face2(Arc::clone(&bytes), face_index_value)
            .map_err(|error| Error::InvalidFont {
                identity: identity.clone(),
                message: error.to_string(),
            })?;
        let glyph_count = u32::try_from(face.num_glyphs()).map_err(|_| Error::InvalidFont {
            identity: identity.clone(),
            message: "face reports an invalid glyph count".to_owned(),
        })?;
        let units_per_em = u16::try_from(face.em_size()).map_err(|_| Error::InvalidFont {
            identity: identity.clone(),
            message: "face reports an invalid units-per-em value".to_owned(),
        })?;
        if units_per_em == 0 {
            return Err(Error::InvalidFont {
                identity,
                message: "face reports zero units per em".to_owned(),
            });
        }
        let family_name = face.family_name().map(Arc::<str>::from);
        Ok(Self {
            bytes,
            face_index,
            identity: Arc::from(identity),
            source,
            family_name,
            glyph_count,
            units_per_em,
            metrics: Arc::new(Mutex::new(FontMetricsCache::default())),
        })
    }

    /// Returns the immutable encoded font bytes.
    pub fn encoded_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns a shared owner for the immutable encoded font bytes.
    pub fn encoded_bytes_arc(&self) -> Arc<[u8]> {
        Arc::clone(&self.bytes)
    }

    /// Returns the face index for a font collection, or zero for a single-face font.
    pub fn face_index(&self) -> u32 {
        self.face_index
    }

    /// Returns the stable non-path identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Returns the source category and its safe identity.
    pub fn source(&self) -> &FontSource {
        &self.source
    }

    /// Returns the family reported by FreeType, when present.
    pub fn family_name(&self) -> Option<&str> {
        self.family_name.as_deref()
    }

    /// Returns the number of glyphs in the selected face.
    pub fn glyph_count(&self) -> u32 {
        self.glyph_count
    }

    fn glyph_for(&self, character: char) -> Result<Option<u32>> {
        let mut metrics = self.metrics.lock().map_err(|_| Error::FontCache {
            message: "face metadata lock is poisoned".to_owned(),
        })?;
        if let Some(glyph_id) = metrics.characters.get(&character) {
            return Ok(*glyph_id);
        }
        let glyph_id = self.with_face(|face| Ok(face.get_char_index(character as usize)))?;
        metrics.characters.insert(character, glyph_id);
        Ok(glyph_id)
    }

    fn advance_mm(&self, glyph_id: u32, size_mm: f64) -> Result<(f64, f64)> {
        let mut metrics = self.metrics.lock().map_err(|_| Error::FontCache {
            message: "face metadata lock is poisoned".to_owned(),
        })?;
        let advance = if let Some(advance) = metrics.advances.get(&glyph_id) {
            *advance
        } else {
            let advance = self.with_face(|face| {
                face.load_glyph(glyph_id, LoadFlag::NO_SCALE | LoadFlag::NO_HINTING)
                    .map_err(|error| Error::InvalidFont {
                        identity: self.identity().to_owned(),
                        message: error.to_string(),
                    })?;
                let advance = face.glyph().advance();
                Ok((advance.x, advance.y))
            })?;
            metrics.advances.insert(glyph_id, advance);
            advance
        };
        let scale = size_mm / f64::from(self.units_per_em);
        let x = advance.0 as f64 * scale;
        let y = advance.1 as f64 * scale;
        if !x.is_finite() || !y.is_finite() {
            return Err(Error::InvalidFont {
                identity: self.identity().to_owned(),
                message: "glyph advance is non-finite".to_owned(),
            });
        }
        Ok((x, y))
    }

    fn with_face<T>(
        &self,
        operation: impl FnOnce(&freetype::Face<Arc<[u8]>>) -> Result<T>,
    ) -> Result<T> {
        let library = freetype::Library::init().map_err(|error| Error::InvalidFont {
            identity: self.identity().to_owned(),
            message: error.to_string(),
        })?;
        let face_index = isize::try_from(self.face_index).map_err(|_| Error::InvalidFont {
            identity: self.identity().to_owned(),
            message: "face index is not representable".to_owned(),
        })?;
        let face = library
            .new_memory_face2(Arc::clone(&self.bytes), face_index)
            .map_err(|error| Error::InvalidFont {
                identity: self.identity().to_owned(),
                message: error.to_string(),
            })?;
        operation(&face)
    }
}

/// A stable diagnostic emitted while selecting glyph faces.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FontDiagnostic {
    /// The declared font lacked the character and an ordered fallback was selected.
    FamilyFallback {
        /// Original Unicode scalar.
        character: char,
        /// Zero-based scalar index in the text object.
        scalar_index: usize,
        /// Declared family or font name.
        requested: String,
        /// Stable identity of the selected fallback face.
        selected: String,
    },
    /// No selected face contained the requested scalar.
    MissingGlyph {
        /// Original Unicode scalar.
        character: char,
        /// Zero-based scalar index in the text object.
        scalar_index: usize,
        /// OFD text object identifier.
        object_id: u64,
        /// Whether a visible replacement glyph was found.
        used_visible_replacement: bool,
    },
}

/// One exact glyph placement in OFD object-space millimetres.
#[derive(Clone, Debug)]
pub struct PositionedGlyph {
    glyph_id: u32,
    x: f64,
    y: f64,
    character: Option<char>,
    source_range: Range<usize>,
    scalar_index: usize,
    source_scalar_range: Range<usize>,
    font: Option<ResolvedFont>,
    font_source: FontSource,
    transform: Option<Transform>,
}

impl PositionedGlyph {
    /// Returns the selected face glyph identifier.
    pub fn glyph_id(&self) -> u32 {
        self.glyph_id
    }
    /// Returns the x coordinate in OFD object-space millimetres.
    pub fn x(&self) -> f64 {
        self.x
    }
    /// Returns the y coordinate in OFD object-space millimetres.
    pub fn y(&self) -> f64 {
        self.y
    }
    /// Returns the associated source scalar, or `None` for an extra mapped glyph.
    pub fn character(&self) -> Option<char> {
        self.character
    }
    /// Returns the byte range in the containing [`GlyphRun`]'s original text.
    pub fn source_range(&self) -> Range<usize> {
        self.source_range.clone()
    }
    /// Returns the zero-based scalar index in the complete text object.
    pub fn scalar_index(&self) -> usize {
        self.scalar_index
    }
    /// Returns the scalar range in the complete text object represented by this glyph.
    pub fn source_scalar_range(&self) -> Range<usize> {
        self.source_scalar_range.clone()
    }
    /// Returns the stable selected-font identity, or `missing`.
    pub fn font_identity(&self) -> &str {
        self.font.as_ref().map_or("missing", ResolvedFont::identity)
    }
    /// Returns the selected font source.
    pub fn font_source(&self) -> &FontSource {
        &self.font_source
    }
    /// Returns the immutable selected font when one exists.
    pub fn font(&self) -> Option<&ResolvedFont> {
        self.font.as_ref()
    }
    /// Returns whether the backend should draw the deterministic synthetic missing box.
    pub fn is_synthetic_box(&self) -> bool {
        matches!(self.font_source, FontSource::Missing)
    }
    /// Returns an optional per-glyph transform.
    ///
    /// The current core subset exposes CG glyph substitutions without matrix data,
    /// so this is currently `None` while keeping the positioned representation ready.
    pub fn transform(&self) -> Option<Transform> {
        self.transform
    }
}

/// One source `TextCode` and its ordered, unshaped positioned glyphs.
#[derive(Clone, Debug)]
pub struct GlyphRun {
    text: String,
    source_range: Range<usize>,
    size_mm: f64,
    glyphs: Vec<PositionedGlyph>,
    diagnostics: Vec<FontDiagnostic>,
}

impl GlyphRun {
    /// Returns the original `TextCode` Unicode text without normalization.
    pub fn text(&self) -> &str {
        &self.text
    }
    /// Returns the scalar range in the complete text object.
    pub fn source_range(&self) -> Range<usize> {
        self.source_range.clone()
    }
    /// Returns the positive text size in millimetres.
    pub fn size_mm(&self) -> f64 {
        self.size_mm
    }
    /// Returns source-ordered glyph placements.
    pub fn glyphs(&self) -> &[PositionedGlyph] {
        &self.glyphs
    }
    /// Returns stable fallback and missing-glyph diagnostics.
    pub fn diagnostics(&self) -> &[FontDiagnostic] {
        &self.diagnostics
    }
}

/// Thread-safe font selection used by the backend-neutral glyph positioner.
pub trait FontResolver: Send + Sync {
    /// Resolves embedded bytes first, then the declared system family or font name.
    fn resolve_primary(&self, resource: &FontResource) -> Result<Option<ResolvedFont>>;
    /// Resolves the first configured fallback family containing `character`.
    fn resolve_fallback(&self, character: char) -> Result<Option<ResolvedFont>>;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
enum CacheKey {
    Embedded(u64, Arc<[u8]>),
    System(ID),
}

/// A deterministic resolver over one immutable `fontdb` snapshot.
///
/// Misses are never cached globally. Constructing a new resolver creates a new
/// lookup scope and is how callers observe fonts installed after this immutable
/// database snapshot was built.
pub struct SystemFontResolver {
    database: Database,
    fallback_faces: Vec<Vec<ID>>,
    max_font_bytes: u64,
    cache: Mutex<HashMap<CacheKey, ResolvedFont>>,
}

impl SystemFontResolver {
    /// Uses an empty database, useful for hermetic callers and tests.
    pub fn empty(fallback_families: Vec<String>, max_font_bytes: u64) -> Self {
        Self::from_database(Database::new(), fallback_families, max_font_bytes)
    }

    /// Takes ownership of an already configured database without scanning the host.
    pub fn from_database(
        database: Database,
        fallback_families: Vec<String>,
        max_font_bytes: u64,
    ) -> Self {
        let fallback_faces = fallback_families
            .iter()
            .map(|family| face_ids_for_name(&database, family))
            .collect();
        Self {
            database,
            fallback_faces,
            max_font_bytes,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Scans currently installed system fonts once and retains that immutable snapshot.
    pub fn with_system_fonts(fallback_families: Vec<String>, max_font_bytes: u64) -> Self {
        let mut database = Database::new();
        database.load_system_fonts();
        Self::from_database(database, fallback_families, max_font_bytes)
    }

    fn cached(
        &self,
        key: CacheKey,
        loader: impl FnOnce() -> Result<ResolvedFont>,
    ) -> Result<ResolvedFont> {
        let mut cache = self.cache.lock().map_err(|_| Error::FontCache {
            message: "initialization lock is poisoned".to_owned(),
        })?;
        if let Some(font) = cache.get(&key) {
            return Ok(font.clone());
        }
        let font = loader()?;
        cache.insert(key, font.clone());
        Ok(font)
    }

    fn face_for_name(&self, name: &str) -> Option<ID> {
        face_ids_for_name(&self.database, name).into_iter().next()
    }

    fn load_system_face(&self, id: ID) -> Result<ResolvedFont> {
        self.cached(CacheKey::System(id), || {
            let info = self.database.face(id).ok_or_else(|| Error::InvalidFont {
                identity: "configured-system-face".to_owned(),
                message: "fontdb face disappeared".to_owned(),
            })?;
            let identity = format!("system:{}:{}", info.post_script_name, info.index);
            let declared_bytes = match &info.source {
                Source::Binary(data) => {
                    u64::try_from(data.as_ref().as_ref().len()).unwrap_or(u64::MAX)
                }
                Source::File(path) => std::fs::metadata(path)
                    .map(|metadata| metadata.len())
                    .map_err(|error| Error::InvalidFont {
                        identity: identity.clone(),
                        message: format!("font bytes could not be inspected: {error}"),
                    })?,
                Source::SharedFile(_, data) => {
                    u64::try_from(data.as_ref().as_ref().len()).unwrap_or(u64::MAX)
                }
            };
            if declared_bytes > self.max_font_bytes {
                return Err(Error::FontBytesExceeded {
                    identity,
                    actual_bytes: declared_bytes,
                    max_bytes: self.max_font_bytes,
                });
            }
            let (actual_bytes, bytes, face_index) = self
                .database
                .with_face_data(id, |bytes, index| {
                    let length = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
                    let owned = (length <= self.max_font_bytes).then(|| bytes.to_vec());
                    (length, owned, index)
                })
                .ok_or_else(|| Error::InvalidFont {
                    identity: identity.clone(),
                    message: "font bytes could not be read".to_owned(),
                })?;
            if actual_bytes > self.max_font_bytes {
                return Err(Error::FontBytesExceeded {
                    identity,
                    actual_bytes,
                    max_bytes: self.max_font_bytes,
                });
            }
            let bytes = bytes.ok_or_else(|| Error::FontBytesExceeded {
                identity: identity.clone(),
                actual_bytes,
                max_bytes: self.max_font_bytes,
            })?;
            ResolvedFont::from_bytes(
                Arc::from(bytes),
                face_index,
                identity.clone(),
                FontSource::System { identity },
            )
        })
    }
}

impl FontResolver for SystemFontResolver {
    fn resolve_primary(&self, resource: &FontResource) -> Result<Option<ResolvedFont>> {
        if let Some(bytes) = resource.encoded_bytes_arc() {
            let fingerprint = fingerprint(&bytes);
            let identity = format!("embedded:{}:{fingerprint:016x}", resource.id());
            return self
                .cached(
                    CacheKey::Embedded(resource.id(), Arc::clone(&bytes)),
                    || {
                        ResolvedFont::from_bytes(
                            bytes,
                            0,
                            identity,
                            FontSource::Embedded {
                                resource_id: resource.id(),
                            },
                        )
                    },
                )
                .map(Some);
        }
        for name in resource
            .family_name()
            .into_iter()
            .chain(std::iter::once(resource.font_name()))
        {
            if let Some(id) = self.face_for_name(name) {
                return self.load_system_face(id).map(Some);
            }
        }
        Ok(None)
    }

    fn resolve_fallback(&self, character: char) -> Result<Option<ResolvedFont>> {
        for faces in &self.fallback_faces {
            for id in faces {
                let font = self.load_system_face(*id)?;
                if font.glyph_for(character)?.is_some() {
                    return Ok(Some(font));
                }
            }
        }
        Ok(None)
    }
}

fn face_ids_for_name(database: &Database, name: &str) -> Vec<ID> {
    let requested = match name.to_ascii_lowercase().as_str() {
        "serif" => Family::Serif,
        "sans-serif" => Family::SansSerif,
        "monospace" => Family::Monospace,
        "cursive" => Family::Cursive,
        "fantasy" => Family::Fantasy,
        _ => Family::Name(name),
    };
    let families = [requested];
    let mut ids = database
        .query(&Query {
            families: &families,
            ..Query::default()
        })
        .into_iter()
        .collect::<Vec<_>>();
    for face in database.faces().filter(|face| {
        face.post_script_name.eq_ignore_ascii_case(name)
            || face
                .families
                .iter()
                .any(|(family, _)| family.eq_ignore_ascii_case(name))
    }) {
        if !ids.contains(&face.id) {
            ids.push(face.id);
        }
    }
    ids
}

fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

struct Selection {
    glyph_id: u32,
    font: Option<ResolvedFont>,
    source: FontSource,
}

/// Resolves and positions the exact source-ordered glyph stream for one text object.
///
/// This does not shape, reorder, or render text. Explicit CG glyph identifiers
/// replace covered scalar positions. Explicit delta attributes, including zero,
/// are authoritative; only a genuinely absent axis uses the selected glyph advance.
/// When a CG mapping emits multiple glyphs, they retain declared glyph order and
/// advance consecutively without shaping; every emitted glyph records the whole
/// mapped scalar range. A mapping may span `TextCode` boundaries and is emitted
/// by the run containing its first scalar.
pub fn position_glyph_runs(
    resolver: &dyn FontResolver,
    resource: &FontResource,
    text: &TextObject,
    limits: &ResourceLimits,
) -> Result<Vec<GlyphRun>> {
    if !text.font_size().is_finite() || text.font_size() <= 0.0 {
        return Err(layout_error(text, "Size", "must be finite and positive"));
    }
    let primary = resolver.resolve_primary(resource)?;
    let total_scalars = text.runs().iter().try_fold(0usize, |total, run| {
        total
            .checked_add(run.text().chars().count())
            .ok_or_else(|| layout_error(text, "text characters", "character count overflow"))
    })?;
    if total_scalars > limits.max_text_characters_per_page {
        return Err(layout_error(
            text,
            "max_text_characters_per_page",
            &format!(
                "{total_scalars} exceeds {}",
                limits.max_text_characters_per_page
            ),
        ));
    }
    let mapped_codes = text
        .glyph_maps()
        .iter()
        .try_fold(0usize, |sum, map| sum.checked_add(map.code_count()))
        .ok_or_else(|| layout_error(text, "CGTransform", "mapped character count overflow"))?;
    let mapped_glyphs = text
        .glyph_maps()
        .iter()
        .try_fold(0usize, |sum, map| sum.checked_add(map.glyphs().len()))
        .ok_or_else(|| layout_error(text, "Glyphs", "mapped glyph count overflow"))?;
    let glyph_count = total_scalars
        .checked_sub(mapped_codes)
        .and_then(|count| count.checked_add(mapped_glyphs))
        .ok_or_else(|| layout_error(text, "Glyphs", "positioned glyph count overflow"))?;
    if glyph_count > limits.max_glyphs_per_page {
        return Err(layout_error(
            text,
            "max_glyphs_per_page",
            &format!("{glyph_count} exceeds {}", limits.max_glyphs_per_page),
        ));
    }
    let expansion = text
        .runs()
        .len()
        .checked_add(text.glyph_maps().len())
        .and_then(|count| count.checked_add(glyph_count))
        .ok_or_else(|| layout_error(text, "text expansion", "entry count overflow"))?;
    if expansion > limits.max_text_expansion_entries {
        return Err(layout_error(
            text,
            "max_text_expansion_entries",
            &format!("{expansion} exceeds {}", limits.max_text_expansion_entries),
        ));
    }

    let mut map_starts = HashMap::with_capacity(text.glyph_maps().len());
    for map in text.glyph_maps() {
        map_starts.insert(map.code_position(), map);
    }
    let requested = resource
        .family_name()
        .unwrap_or_else(|| resource.font_name())
        .to_owned();
    let mut runs = Vec::with_capacity(text.runs().len());
    let mut global_scalar = 0usize;
    let mut covered_until = 0usize;
    for run in text.runs() {
        let characters = run.text().char_indices().collect::<Vec<_>>();
        let run_start = global_scalar;
        let mut glyphs = Vec::new();
        let mut diagnostics = Vec::new();
        let mut local = 0usize;
        let mut x = run.x();
        let mut y = run.y();
        while local < characters.len() {
            let scalar_index = global_scalar + local;
            if scalar_index < covered_until {
                local += 1;
                continue;
            }
            if let Some(map) = map_starts.get(&scalar_index) {
                let selected = primary.as_ref().ok_or_else(|| {
                    layout_error(
                        text,
                        "CGTransform",
                        "explicit glyph IDs require a resolved primary font",
                    )
                })?;
                let consumed = map.code_count().min(characters.len() - local);
                covered_until = scalar_index.checked_add(map.code_count()).ok_or_else(|| {
                    layout_error(text, "CGTransform", "mapped scalar range overflow")
                })?;
                let start_byte = characters[local].0;
                let end_local = local + consumed;
                let end_byte = characters
                    .get(end_local)
                    .map_or(run.text().len(), |(offset, _)| *offset);
                let mut inferred_x = 0.0;
                let mut inferred_y = 0.0;
                for (glyph_offset, glyph_id) in map.glyphs().iter().copied().enumerate() {
                    validate_glyph(selected, glyph_id)?;
                    let character =
                        (glyph_offset < consumed).then(|| characters[local + glyph_offset].1);
                    glyphs.push(PositionedGlyph {
                        glyph_id,
                        x: finite_coordinate(text, "glyph x", x + inferred_x)?,
                        y: finite_coordinate(text, "glyph y", y + inferred_y)?,
                        character,
                        source_range: start_byte..end_byte,
                        scalar_index,
                        source_scalar_range: scalar_index..covered_until,
                        font: Some(selected.clone()),
                        font_source: selected.source().clone(),
                        transform: None,
                    });
                    let advance = selected.advance_mm(glyph_id, text.font_size())?;
                    inferred_x = finite_coordinate(text, "advance x", inferred_x + advance.0)?;
                    inferred_y = finite_coordinate(text, "advance y", inferred_y + advance.1)?;
                }
                let delta_x = if run.has_explicit_delta_x() {
                    run.delta_x()[local..end_local].iter().sum()
                } else {
                    inferred_x
                };
                let delta_y = if run.has_explicit_delta_y() {
                    run.delta_y()[local..end_local].iter().sum()
                } else {
                    inferred_y
                };
                x = finite_coordinate(text, "glyph x", x + delta_x)?;
                y = finite_coordinate(text, "glyph y", y + delta_y)?;
                local = end_local;
                continue;
            }

            let (byte_start, character) = characters[local];
            let byte_end = characters
                .get(local + 1)
                .map_or(run.text().len(), |(offset, _)| *offset);
            let selection = select_character(
                resolver,
                primary.as_ref(),
                character,
                scalar_index,
                text.object_id(),
                &requested,
                &mut diagnostics,
            )?;
            glyphs.push(PositionedGlyph {
                glyph_id: selection.glyph_id,
                x: finite_coordinate(text, "glyph x", x)?,
                y: finite_coordinate(text, "glyph y", y)?,
                character: Some(character),
                source_range: byte_start..byte_end,
                scalar_index,
                source_scalar_range: scalar_index..scalar_index + 1,
                font: selection.font.clone(),
                font_source: selection.source,
                transform: None,
            });
            let advance = selection
                .font
                .as_ref()
                .map(|font| font.advance_mm(selection.glyph_id, text.font_size()))
                .transpose()?
                .unwrap_or((text.font_size(), 0.0));
            let delta_x = if run.has_explicit_delta_x() {
                run.delta_x()[local]
            } else {
                advance.0
            };
            let delta_y = if run.has_explicit_delta_y() {
                run.delta_y()[local]
            } else {
                advance.1
            };
            x = finite_coordinate(text, "glyph x", x + delta_x)?;
            y = finite_coordinate(text, "glyph y", y + delta_y)?;
            local += 1;
        }
        global_scalar = global_scalar
            .checked_add(characters.len())
            .ok_or_else(|| layout_error(text, "text characters", "character index overflow"))?;
        runs.push(GlyphRun {
            text: run.text().to_owned(),
            source_range: run_start..global_scalar,
            size_mm: text.font_size(),
            glyphs,
            diagnostics,
        });
    }
    Ok(runs)
}

fn select_character(
    resolver: &dyn FontResolver,
    primary: Option<&ResolvedFont>,
    character: char,
    scalar_index: usize,
    object_id: u64,
    requested: &str,
    diagnostics: &mut Vec<FontDiagnostic>,
) -> Result<Selection> {
    if let Some(font) = primary {
        if let Some(glyph_id) = font.glyph_for(character)? {
            return Ok(Selection {
                glyph_id,
                font: Some(font.clone()),
                source: font.source().clone(),
            });
        }
    }
    if let Some(font) = resolver.resolve_fallback(character)? {
        if let Some(glyph_id) = font.glyph_for(character)? {
            diagnostics.push(FontDiagnostic::FamilyFallback {
                character,
                scalar_index,
                requested: requested.to_owned(),
                selected: font.identity().to_owned(),
            });
            return Ok(Selection {
                glyph_id,
                source: FontSource::ConfiguredFallback {
                    identity: font.identity().to_owned(),
                },
                font: Some(font),
            });
        }
    }
    for replacement in ['\u{25a1}', '\u{fffd}'] {
        if let Some(font) = primary {
            if let Some(glyph_id) = font.glyph_for(replacement)? {
                diagnostics.push(FontDiagnostic::MissingGlyph {
                    character,
                    scalar_index,
                    object_id,
                    used_visible_replacement: true,
                });
                return Ok(Selection {
                    glyph_id,
                    source: font.source().clone(),
                    font: Some(font.clone()),
                });
            }
        }
        if let Some(font) = resolver.resolve_fallback(replacement)? {
            if let Some(glyph_id) = font.glyph_for(replacement)? {
                diagnostics.push(FontDiagnostic::MissingGlyph {
                    character,
                    scalar_index,
                    object_id,
                    used_visible_replacement: true,
                });
                return Ok(Selection {
                    glyph_id,
                    source: FontSource::ConfiguredFallback {
                        identity: font.identity().to_owned(),
                    },
                    font: Some(font),
                });
            }
        }
    }
    diagnostics.push(FontDiagnostic::MissingGlyph {
        character,
        scalar_index,
        object_id,
        used_visible_replacement: false,
    });
    Ok(Selection {
        glyph_id: 0,
        font: None,
        source: FontSource::Missing,
    })
}

fn validate_glyph(font: &ResolvedFont, glyph_id: u32) -> Result<()> {
    if glyph_id >= font.glyph_count() {
        return Err(Error::InvalidGlyph {
            identity: font.identity().to_owned(),
            glyph_id,
            glyph_count: font.glyph_count(),
        });
    }
    Ok(())
}

fn finite_coordinate(text: &TextObject, field: &'static str, value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(layout_error(
            text,
            field,
            "arithmetic produced a non-finite value",
        ))
    }
}

fn layout_error(text: &TextObject, field: &'static str, message: &str) -> Error {
    Error::InvalidTextLayout {
        object_id: text.object_id(),
        field,
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::thread;

    use super::*;

    const FONT: &[u8] = include_bytes!("../tests/fixtures/fonts/phase3-subset.ttf");

    fn fixture_font(bytes: Arc<[u8]>) -> Result<ResolvedFont> {
        ResolvedFont::from_bytes(
            bytes,
            0,
            "unit-fixture".to_owned(),
            FontSource::Embedded { resource_id: 1 },
        )
    }

    #[test]
    fn cache_is_single_flight_and_does_not_publish_failures() {
        let resolver = Arc::new(SystemFontResolver::empty(Vec::new(), 1 << 20));
        let bytes: Arc<[u8]> = Arc::from(FONT);
        let initializations = Arc::new(AtomicUsize::new(0));
        let threads = (0..12)
            .map(|_| {
                let resolver = Arc::clone(&resolver);
                let bytes = Arc::clone(&bytes);
                let initializations = Arc::clone(&initializations);
                thread::spawn(move || {
                    resolver.cached(CacheKey::Embedded(1, Arc::clone(&bytes)), || {
                        initializations.fetch_add(1, Ordering::SeqCst);
                        fixture_font(bytes)
                    })
                })
            })
            .collect::<Vec<_>>();
        for thread in threads {
            thread.join().unwrap().unwrap();
        }
        assert_eq!(initializations.load(Ordering::SeqCst), 1);

        let failed_key = CacheKey::Embedded(2, Arc::from(FONT));
        let attempts = AtomicUsize::new(0);
        assert!(resolver
            .cached(failed_key.clone(), || {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err(Error::InvalidFont {
                    identity: "retry".to_owned(),
                    message: "injected failure".to_owned(),
                })
            })
            .is_err());
        resolver
            .cached(failed_key, || {
                attempts.fetch_add(1, Ordering::SeqCst);
                fixture_font(Arc::from(FONT))
            })
            .unwrap();
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }
}
