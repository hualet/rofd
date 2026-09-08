use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct OfdRoot {
    #[serde(rename = "DocBody")]
    pub(crate) doc_bodies: Vec<DocBody>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DocBody {
    pub(crate) doc_info: DocInfo,
    pub(crate) doc_root: String,
    #[serde(rename = "Signatures")]
    pub(crate) signatures: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SignaturesRoot {
    #[serde(rename = "Signature", default)]
    pub(crate) signatures: Vec<SignatureEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SignatureEntry {
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
    #[serde(rename = "BaseLoc")]
    pub(crate) base_loc: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SignatureRoot {
    #[serde(rename = "SignedInfo")]
    pub(crate) signed_info: SignedInfo,
    #[serde(rename = "SignedValue")]
    pub(crate) signed_value: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SignedInfo {
    #[serde(rename = "StampAnnot", default)]
    pub(crate) stamp_annots: Vec<StampAnnotRaw>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StampAnnotRaw {
    #[serde(rename = "PageRef")]
    pub(crate) page_ref: String,
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
    #[serde(rename = "Boundary")]
    pub(crate) boundary: String,
    #[serde(rename = "Clip")]
    pub(crate) clip: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DocInfo {
    #[serde(rename = "DocID")]
    pub(crate) document_id: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) author: Option<String>,
    pub(crate) subject: Option<String>,
    #[serde(rename = "Abstract")]
    pub(crate) abstract_: Option<String>,
    pub(crate) creator: Option<String>,
    pub(crate) creator_version: Option<String>,
    pub(crate) creation_date: Option<String>,
    pub(crate) mod_date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct DocumentRoot {
    pub(crate) common_data: CommonData,
    pub(crate) pages: PageList,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct CommonData {
    /// Optional because real-world producers omit it; documents without it
    /// are only usable when every page declares its own Area.
    pub(crate) page_area: Option<PageArea>,
    pub(crate) public_res: Option<String>,
    pub(crate) document_res: Option<String>,
    #[serde(rename = "TemplatePage", default)]
    pub(crate) template_pages: Vec<TemplatePage>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ResourceRoot {
    #[serde(rename = "BaseLoc")]
    pub(crate) base_loc: Option<String>,
    /// Real-world producers occasionally emit several Fonts blocks (ofdrw's
    /// ano.ofd); every block contributes entries to the catalog.
    #[serde(rename = "Fonts", default)]
    pub(crate) fonts: Vec<Fonts>,
    #[serde(rename = "MultiMedias")]
    pub(crate) multi_medias: Option<MultiMedias>,
    #[serde(rename = "DrawParams")]
    pub(crate) draw_params: Option<DrawParams>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Fonts {
    #[serde(rename = "Font", default)]
    pub(crate) entries: Vec<FontEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct FontEntry {
    #[serde(rename = "ID")]
    pub(crate) id: String,
    #[serde(rename = "FontName")]
    pub(crate) font_name: Option<String>,
    #[serde(rename = "FamilyName")]
    pub(crate) family_name: Option<String>,
    #[serde(rename = "Charset")]
    pub(crate) charset: Option<String>,
    #[serde(rename = "FontFile")]
    pub(crate) font_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MultiMedias {
    #[serde(rename = "MultiMedia", default)]
    pub(crate) entries: Vec<MultiMediaEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MultiMediaEntry {
    #[serde(rename = "ID")]
    pub(crate) id: String,
    #[serde(rename = "Type")]
    pub(crate) kind: Option<String>,
    #[serde(rename = "Format")]
    pub(crate) format: Option<String>,
    #[serde(rename = "MediaFile")]
    pub(crate) media_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DrawParams {
    #[serde(rename = "DrawParam", default)]
    pub(crate) entries: Vec<DrawParamEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DrawParamEntry {
    #[serde(rename = "ID")]
    pub(crate) id: String,
    #[serde(rename = "Relative")]
    pub(crate) relative: Option<String>,
    #[serde(rename = "LineWidth")]
    pub(crate) line_width: Option<String>,
    #[serde(rename = "Join")]
    pub(crate) line_join: Option<String>,
    #[serde(rename = "Cap")]
    pub(crate) line_cap: Option<String>,
    #[serde(rename = "DashOffset")]
    pub(crate) dash_offset: Option<String>,
    #[serde(rename = "DashPattern")]
    pub(crate) dash_pattern: Option<String>,
    #[serde(rename = "MiterLimit")]
    pub(crate) miter_limit: Option<String>,
    #[serde(rename = "FillColor")]
    pub(crate) fill_color: Option<PaintColor>,
    #[serde(rename = "StrokeColor")]
    pub(crate) stroke_color: Option<PaintColor>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TemplatePage {
    #[serde(rename = "ID")]
    pub(crate) id: String,
    #[serde(rename = "BaseLoc")]
    pub(crate) base_loc: String,
    #[serde(rename = "ZOrder")]
    pub(crate) z_order: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct PageArea {
    pub(crate) physical_box: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct PageList {
    #[serde(rename = "Page")]
    pub(crate) pages: Vec<PageEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PageEntry {
    #[serde(rename = "ID")]
    pub(crate) id: u64,
    #[serde(rename = "BaseLoc")]
    pub(crate) base_loc: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct PageRoot {
    pub(crate) area: Option<PageArea>,
    #[serde(rename = "Template", default)]
    pub(crate) templates: Vec<TemplateReference>,
    pub(crate) content: Option<PageContent>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TemplateReference {
    #[serde(rename = "TemplateID")]
    pub(crate) template_id: String,
    #[serde(rename = "ZOrder")]
    pub(crate) z_order: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PageContent {
    #[serde(rename = "Layer", default)]
    pub(crate) layers: Vec<Layer>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Layer {
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
    #[serde(rename = "Type")]
    pub(crate) kind: Option<String>,
    #[serde(rename = "$value", default)]
    pub(crate) objects: Vec<GraphicUnit>,
}

#[derive(Debug, Deserialize)]
pub(crate) enum GraphicUnit {
    #[serde(rename = "PathObject")]
    Path(PathObjectEnvelope),
    #[serde(rename = "PageBlock")]
    Group(PageBlock),
    #[serde(rename = "TextObject")]
    Text(TextObjectEnvelope),
    #[serde(rename = "ImageObject")]
    Image(ImageObjectEnvelope),
    #[serde(rename = "CompositeObject")]
    Composite(ObjectReference),
}

#[derive(Debug, Deserialize)]
pub(crate) struct PageBlock {
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
    #[serde(rename = "$value", default)]
    pub(crate) objects: Vec<GraphicUnit>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ObjectReference {
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TextObjectEnvelope {
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
    #[serde(skip)]
    pub(crate) object: Option<Box<TextObject>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ImageObjectEnvelope {
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
    #[serde(skip)]
    pub(crate) object: Option<Box<ImageObject>>,
}

/// PathObject is parsed standalone like TextObject/ImageObject: serde-xml-rs
/// 0.6 mishandles its nested Clips vectors when a sibling graphic unit
/// follows, so the streaming pass extracts the payload separately.
#[derive(Debug, Deserialize)]
pub(crate) struct PathObjectEnvelope {
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
    #[serde(skip)]
    pub(crate) object: Option<Box<PathObject>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PathObject {
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
    #[serde(rename = "Boundary")]
    pub(crate) boundary: String,
    #[serde(rename = "CTM")]
    pub(crate) transform: Option<String>,
    #[serde(rename = "Stroke")]
    pub(crate) stroke: Option<String>,
    #[serde(rename = "Fill")]
    pub(crate) fill: Option<String>,
    #[serde(rename = "LineWidth")]
    pub(crate) line_width: Option<String>,
    #[serde(rename = "Rule")]
    pub(crate) fill_rule: Option<String>,
    #[serde(rename = "Alpha")]
    pub(crate) alpha: Option<String>,
    #[serde(rename = "DrawParam")]
    pub(crate) draw_param: Option<String>,
    #[serde(rename = "Join")]
    pub(crate) line_join: Option<String>,
    #[serde(rename = "Cap")]
    pub(crate) line_cap: Option<String>,
    #[serde(rename = "DashOffset")]
    pub(crate) dash_offset: Option<String>,
    #[serde(rename = "DashPattern")]
    pub(crate) dash_pattern: Option<String>,
    #[serde(rename = "MiterLimit")]
    pub(crate) miter_limit: Option<String>,
    #[serde(rename = "AbbreviatedData")]
    pub(crate) abbreviated_data: String,
    #[serde(rename = "StrokeColor")]
    pub(crate) stroke_color: Option<PaintColor>,
    #[serde(rename = "FillColor")]
    pub(crate) fill_color: Option<PaintColor>,
    #[serde(rename = "Clips")]
    pub(crate) clips: Option<Clips>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TextObject {
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
    #[serde(rename = "Boundary")]
    pub(crate) boundary: Option<String>,
    #[serde(rename = "CTM")]
    pub(crate) transform: Option<String>,
    #[serde(rename = "Font")]
    pub(crate) font: Option<String>,
    #[serde(rename = "Size")]
    pub(crate) size: Option<String>,
    #[serde(rename = "Stroke")]
    pub(crate) stroke: Option<String>,
    #[serde(rename = "Fill")]
    pub(crate) fill: Option<String>,
    #[serde(rename = "Alpha")]
    pub(crate) alpha: Option<String>,
    #[serde(rename = "DrawParam")]
    pub(crate) draw_param: Option<String>,
    #[serde(rename = "LineWidth")]
    pub(crate) line_width: Option<String>,
    #[serde(rename = "Join")]
    pub(crate) line_join: Option<String>,
    #[serde(rename = "Cap")]
    pub(crate) line_cap: Option<String>,
    #[serde(rename = "DashOffset")]
    pub(crate) dash_offset: Option<String>,
    #[serde(rename = "DashPattern")]
    pub(crate) dash_pattern: Option<String>,
    #[serde(rename = "MiterLimit")]
    pub(crate) miter_limit: Option<String>,
    #[serde(rename = "FillColor")]
    pub(crate) fill_color: Option<PaintColor>,
    #[serde(rename = "StrokeColor")]
    pub(crate) stroke_color: Option<PaintColor>,
    #[serde(rename = "Clips")]
    pub(crate) clips: Option<Clips>,
    #[serde(rename = "TextCode", default)]
    pub(crate) text_codes: Vec<TextCode>,
    #[serde(rename = "CGTransform", default)]
    pub(crate) cg_transforms: Vec<CgTransform>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TextCode {
    #[serde(rename = "$value", default)]
    pub(crate) text: String,
    #[serde(rename = "X")]
    pub(crate) x: Option<String>,
    #[serde(rename = "Y")]
    pub(crate) y: Option<String>,
    #[serde(rename = "DeltaX")]
    pub(crate) delta_x: Option<String>,
    #[serde(rename = "DeltaY")]
    pub(crate) delta_y: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CgTransform {
    #[serde(rename = "CodePosition")]
    pub(crate) code_position: Option<String>,
    #[serde(rename = "CodeCount")]
    pub(crate) code_count: Option<String>,
    #[serde(rename = "GlyphCount")]
    pub(crate) glyph_count: Option<String>,
    #[serde(rename = "Glyphs")]
    pub(crate) glyphs: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ImageObject {
    #[serde(rename = "ID")]
    pub(crate) id: Option<String>,
    #[serde(rename = "Boundary")]
    pub(crate) boundary: Option<String>,
    #[serde(rename = "CTM")]
    pub(crate) transform: Option<String>,
    #[serde(rename = "ResourceID")]
    pub(crate) resource_id: Option<String>,
    #[serde(rename = "Alpha")]
    pub(crate) alpha: Option<String>,
    #[serde(rename = "DrawParam")]
    pub(crate) draw_param: Option<String>,
    #[serde(rename = "Substitution")]
    pub(crate) substitution: Option<String>,
    #[serde(rename = "ImageMask")]
    pub(crate) image_mask: Option<String>,
    #[serde(rename = "Clips")]
    pub(crate) clips: Option<Clips>,
    #[serde(rename = "Border")]
    pub(crate) border: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Clips {
    #[serde(rename = "TransFlag")]
    pub(crate) trans_flag: Option<String>,
    #[serde(rename = "Clip", default)]
    pub(crate) clips: Vec<Clip>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Clip {
    #[serde(rename = "Area", default)]
    pub(crate) areas: Vec<ClipArea>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClipArea {
    #[serde(rename = "CTM")]
    pub(crate) transform: Option<String>,
    #[serde(rename = "$value", default)]
    pub(crate) children: Vec<ClipAreaChild>,
}

#[derive(Debug, Deserialize)]
pub(crate) enum ClipAreaChild {
    #[serde(rename = "Path")]
    Path(ClipPath),
    #[serde(rename = "Text")]
    Text(ClipText),
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClipPath {
    #[serde(rename = "Boundary")]
    pub(crate) boundary: Option<String>,
    #[serde(rename = "CTM")]
    pub(crate) transform: Option<String>,
    #[serde(rename = "Stroke")]
    pub(crate) stroke: Option<String>,
    #[serde(rename = "Fill")]
    pub(crate) fill: Option<String>,
    #[serde(rename = "Rule")]
    pub(crate) fill_rule: Option<String>,
    #[serde(rename = "AbbreviatedData")]
    pub(crate) abbreviated_data: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClipText {}

#[derive(Debug, Deserialize)]
pub(crate) struct PaintColor {
    #[serde(rename = "Value")]
    pub(crate) value: Option<String>,
    #[serde(rename = "Alpha")]
    pub(crate) alpha: Option<String>,
}
