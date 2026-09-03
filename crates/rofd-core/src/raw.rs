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
    pub(crate) page_area: PageArea,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub(crate) struct PageArea {
    pub(crate) physical_box: String,
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
    pub(crate) content: Option<PageContent>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PageContent {
    #[serde(rename = "Layer", default)]
    pub(crate) layers: Vec<Layer>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Layer {
    #[serde(rename = "ID")]
    pub(crate) id: u64,
    #[serde(rename = "Type")]
    pub(crate) kind: Option<String>,
    #[serde(rename = "$value", default)]
    pub(crate) objects: Vec<GraphicUnit>,
}

#[derive(Debug, Deserialize)]
pub(crate) enum GraphicUnit {
    #[serde(rename = "PathObject")]
    Path(Box<PathObject>),
    #[serde(rename = "PageBlock")]
    Group(PageBlock),
    #[serde(rename = "TextObject")]
    Text(ObjectReference),
    #[serde(rename = "ImageObject")]
    Image(ObjectReference),
    #[serde(rename = "CompositeObject")]
    Composite(ObjectReference),
}

#[derive(Debug, Deserialize)]
pub(crate) struct PageBlock {
    #[serde(rename = "ID")]
    pub(crate) id: u64,
    #[serde(rename = "$value", default)]
    pub(crate) objects: Vec<GraphicUnit>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ObjectReference {
    #[serde(rename = "ID")]
    pub(crate) id: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PathObject {
    #[serde(rename = "ID")]
    pub(crate) id: u64,
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
    #[serde(rename = "AbbreviatedData")]
    pub(crate) abbreviated_data: String,
    #[serde(rename = "StrokeColor")]
    pub(crate) stroke_color: Option<PaintColor>,
    #[serde(rename = "FillColor")]
    pub(crate) fill_color: Option<PaintColor>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PaintColor {
    #[serde(rename = "Value")]
    pub(crate) value: String,
    #[serde(rename = "Alpha")]
    pub(crate) alpha: Option<String>,
}
