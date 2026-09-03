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
}
