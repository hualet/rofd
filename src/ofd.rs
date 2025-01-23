use std::fs::File;
use std::io::BufReader;

use zip::ZipArchive;
use serde::{Deserialize, Serialize};

/*
<?xml version="1.0" encoding="UTF-8"?><ofd:OFD xmlns:ofd="http://www.ofdspec.org/2016" DocType="OFD" Version="1.0">
  <ofd:DocBody>
    <ofd:DocInfo>
      <ofd:DocID>2195d5df959c419cb575dab5eeabb065</ofd:DocID>
      <ofd:CreationDate>2025-01-06</ofd:CreationDate>
      <ofd:Creator>nuonuo</ofd:Creator>
      <ofd:CreatorVersion>1.0.0</ofd:CreatorVersion>
      <ofd:ModDate>2024-10-22</ofd:ModDate>
      <ofd:CustomDatas>
        <ofd:CustomData Name="native-producer">SuwellFormSDK</ofd:CustomData>
        <ofd:CustomData Name="producer-version">1.1.22.0112</ofd:CustomData>
        <ofd:CustomData Name="发票号码">25422000000003570205</ofd:CustomData>
        <ofd:CustomData Name="合计税额">2.61</ofd:CustomData>
        <ofd:CustomData Name="合计金额">261.39</ofd:CustomData>
        <ofd:CustomData Name="开票日期">2025年01月06日</ofd:CustomData>
        <ofd:CustomData Name="购买方纳税人识别号">91110302MA01NP925M</ofd:CustomData>
        <ofd:CustomData Name="销售方纳税人识别号">92420100MA4JMP6D4P</ofd:CustomData>
      </ofd:CustomDatas>
    </ofd:DocInfo>
    <ofd:DocRoot>Doc_0/Document.xml</ofd:DocRoot>
  </ofd:DocBody>
</ofd:OFD>
*/

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct OfdNode {
    #[serde(rename = "DocBody")]
    pub doc_body: DocBody,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DocBody {
    pub doc_info: DocInfo,
    pub doc_root: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct DocInfo {
    #[serde(rename = "DocID")]
    pub doc_id: String,
    #[serde(rename = "CreationDate")]
    pub creation_date: String,
    #[serde(rename = "Creator")]
    pub creator: String,
    #[serde(rename = "CreatorVersion")]
    pub creator_version: String,
    #[serde(rename = "ModDate")]
    pub mod_date: String,
    #[serde(rename = "CustomDatas")]
    pub custom_datas: CustomDataList,
}


#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct CustomDataList {
    #[serde(rename = "CustomData")]
    pub custom_data: Vec<CustomData>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub struct CustomData {
    #[serde(rename = "Name")]
    pub name: Option<String>, // XML attribute
    #[serde(rename = "$value")]
    pub value: String, // XML node's text content
}

#[derive(Debug)]
pub struct Ofd {
    pub node: OfdNode,
    pub zip_archive: ZipArchive<BufReader<File>>,
}

impl OfdNode {
    pub fn from_xml(xml: &str) -> Result<OfdNode, serde_xml_rs::Error> {
        serde_xml_rs::from_str(xml)
    }
}
