use serde::Deserialize;

use crate::elements::*;

/* Document.xml
<?xml version="1.0" encoding="UTF-8"?><ofd:Document xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:CommonData>
    <ofd:PageArea>
      <ofd:PhysicalBox>0 0 210 297</ofd:PhysicalBox>
      <ofd:ApplicationBox>0 0 210 297</ofd:ApplicationBox>
    </ofd:PageArea>
    <ofd:PublicRes>PublicRes.xml</ofd:PublicRes>
    <ofd:DocumentRes>DocumentRes.xml</ofd:DocumentRes>
    <ofd:MaxUnitID>176</ofd:MaxUnitID>
  </ofd:CommonData>
  <ofd:Pages>
    <ofd:Page BaseLoc="Pages/Page_0/Content.xml" ID="1"/>
  </ofd:Pages>
  <ofd:Annotations>Annots/Annotations.xml</ofd:Annotations>
  <ofd:CustomTags>Tags/CustomTags.xml</ofd:CustomTags>
</ofd:Document>
*/

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Document {
    pub common_data: CommonData,
    pub pages: PageList,
    pub annotations: Option<String>,
    pub custom_tags: Option<String>,

    #[serde(skip)]
    pub doc_res: DocumentRes,
    #[serde(skip)]
    pub public_res: PublicRes,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct CommonData {
    pub page_area: PageArea,
    pub public_res: String,
    pub document_res: String,
    #[serde(rename = "MaxUnitID")]
    pub max_unit_id: u32,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PageArea {
    pub physical_box: String,
    pub application_box: String,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PageList {
    pub page: Vec<PageElement>, // TODO(hualet): don't know why, same in ofd.rs CustomData:custom_data.
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PageElement {
    #[serde(rename = "BaseLoc")]
    pub base_loc: Option<String>, // TODO(hualet): don't know why the field should be Option.
    #[serde(rename = "ID")]
    pub id: Option<u32>,
}

impl Document {
    pub fn from_xml(xml: &str) -> Result<Document, serde_xml_rs::Error> {
        serde_xml_rs::from_str(xml)
    }
}

/*  DocumentRes.xml
<?xml version="1.0" encoding="UTF-8"?><ofd:Res xmlns:ofd="http://www.ofdspec.org/2016" BaseLoc="Res">
  <ofd:MultiMedias>
    <ofd:MultiMedia Format="PNG" ID="36" Type="Image">
      <ofd:MediaFile>b7cdce6106634283a4c851ae1e3347717795273356109011760_ewm.png</ofd:MediaFile>
    </ofd:MultiMedia>
    <ofd:MultiMedia Format="PNG" ID="174" Type="Image">
      <ofd:MediaFile>signature.png</ofd:MediaFile>
    </ofd:MultiMedia>
  </ofd:MultiMedias>
</ofd:Res>
*/

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DocumentRes {
    pub base_loc: String,
    pub multi_medias: MultiMedias,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct MultiMedias {
    pub multi_media: Vec<MultiMedia>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct MultiMedia {
    #[serde(rename = "ID")]
    pub id: u32,
    pub format: String,
    pub media_file: String,
}

impl DocumentRes {
    pub fn from_xml(xml: &str) -> Result<DocumentRes, serde_xml_rs::Error> {
        serde_xml_rs::from_str(xml)
    }
}

/* PublicRes.xml
<?xml version="1.0" encoding="UTF-8"?><ofd:Res xmlns:ofd="http://www.ofdspec.org/2016" BaseLoc="Res">
  <ofd:Fonts>
    <ofd:Font FamilyName="宋体" FontName="宋体" ID="3"/>
    <ofd:Font FamilyName="楷体" FontName="楷体" ID="5"/>
    <ofd:Font FamilyName="Courier New" FontName="Courier New" ID="7"/>
  </ofd:Fonts>
</ofd:Res>
*/

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PublicRes {
    pub base_loc: String,
    pub fonts: Fonts,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Fonts {
    pub font: Vec<Font>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Font {
    #[serde(rename = "ID")]
    pub id: u32,
    pub family_name: String,
    pub font_name: String,
}

impl PublicRes {
    pub fn from_xml(xml: &str) -> Result<PublicRes, serde_xml_rs::Error> {
        serde_xml_rs::from_str(xml)
    }
}

/* Annotations.xml
<?xml version="1.0" encoding="UTF-8"?>
<ofd:Annotations xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Page PageID="1">
    <ofd:FileLoc>Page_0/Annot_0.xml</ofd:FileLoc>
  </ofd:Page>
</ofd:Annotations>
*/

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct Annotations {
    pub page: Vec<AnnotationPageNode>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct AnnotationPageNode {
    #[serde(rename = "PageID")]
    pub page_id: u32,
    #[serde(rename = "$value")]
    pub file_loc: String,
}

impl Annotations {
    pub fn from_xml(xml: &str) -> Result<Annotations, serde_xml_rs::Error> {
        serde_xml_rs::from_str(xml)
    }
}

/* Annot_0.xml
<?xml version="1.0" encoding="UTF-8"?>
<ofd:PageAnnot xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Annot Type="Stamp" Creator="OFD R&amp;W" LastModDate="2024-10-22" ID="173">
    <ofd:Appearance Boundary="87.50 8.50 30 20">
      <ofd:ImageObject ID="175" ResourceID="174" Boundary="0 0 30 20" CTM="30 0 0 20 0 0"/>
    </ofd:Appearance>
  </ofd:Annot>
</ofd:PageAnnot>
*/

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "PascalCase")]
pub struct PageAnnot {
    pub annot: Vec<Annot>,
}

impl PageAnnot {
    pub fn from_xml(xml: &str) -> Result<PageAnnot, serde_xml_rs::Error> {
        serde_xml_rs::from_str(xml)
    }
}
