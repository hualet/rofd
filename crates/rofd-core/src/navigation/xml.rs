//! A flat, bounded arena containing only selected direct navigation subtrees.

use xml::{
    attribute::OwnedAttribute,
    name::OwnedName,
    reader::{EventReader, XmlEvent},
};

use super::budget::StringBudget;
use crate::{Error, ResourceLimits, Result};

fn retain_name(budget: &mut StringBudget, name: &OwnedName) -> Result<()> {
    budget.reserve(name.local_name.len())?;
    budget.reserve(name.namespace.as_deref().unwrap_or_default().len())?;
    budget.reserve(name.prefix.as_deref().unwrap_or_default().len())
}

pub(crate) fn native(name: &OwnedName) -> bool {
    matches!(
        name.namespace.as_deref(),
        None | Some("") | Some("http://www.ofdspec.org/2016") | Some("http://www.ofdspec.org")
    )
}

#[derive(Debug)]
pub(crate) struct XmlNode {
    pub(crate) name: OwnedName,
    attributes: Vec<OwnedAttribute>,
    pub(crate) text: String,
    pub(crate) children: Vec<usize>,
}

impl XmlNode {
    pub(crate) fn is(&self, local: &str) -> bool {
        native(&self.name) && self.name.local_name == local
    }

    pub(crate) fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| native(&a.name) && a.name.local_name == name)
            .map(|a| a.value.as_str())
    }
}

#[derive(Debug)]
pub(crate) struct NavigationXml {
    pub(crate) nodes: Vec<XmlNode>,
    pub(crate) roots: Vec<usize>,
    pub(crate) historical_namespace: bool,
}

impl NavigationXml {
    pub(crate) fn children<'a>(
        &'a self,
        node: usize,
        local: &'a str,
    ) -> impl DoubleEndedIterator<Item = usize> + 'a {
        self.nodes[node]
            .children
            .iter()
            .copied()
            .filter(move |&i| self.nodes[i].is(local))
    }

    pub(crate) fn read(bytes: &[u8], path: &str, limits: &ResourceLimits) -> Result<Self> {
        let mut result = Self {
            nodes: Vec::new(),
            roots: Vec::new(),
            historical_namespace: false,
        };
        let mut selected: Vec<usize> = Vec::new();
        let mut depth = 0usize;
        let mut document = false;
        let mut historical_document = false;
        let mut retained_strings = StringBudget::new(limits.max_entry_size);
        for event in EventReader::new(bytes) {
            match event.map_err(|e| Error::Xml {
                path: path.to_owned(),
                message: e.to_string(),
            })? {
                XmlEvent::StartElement {
                    name, attributes, ..
                } => {
                    depth += 1;
                    if depth > limits.max_xml_depth {
                        return Err(Error::LimitExceeded(format!(
                            "XML depth exceeds {} in {path}",
                            limits.max_xml_depth
                        )));
                    }
                    if depth == 1 {
                        document = native(&name) && name.local_name == "Document";
                        historical_document =
                            name.namespace.as_deref() == Some("http://www.ofdspec.org");
                    }
                    let start = document
                        && depth == 2
                        && native(&name)
                        && matches!(name.local_name.as_str(), "Outlines" | "Bookmarks");
                    if start || !selected.is_empty() {
                        if result.nodes.len() >= limits.max_page_objects {
                            return Err(Error::LimitExceeded(format!(
                                "navigation XML nodes exceed {} in {path}",
                                limits.max_page_objects
                            )));
                        }
                        // Namespace URIs and entity values are expanded by the
                        // XML reader. Repeated short names can otherwise retain
                        // many independent copies of one large declaration.
                        retain_name(&mut retained_strings, &name)?;
                        for attribute in &attributes {
                            retain_name(&mut retained_strings, &attribute.name)?;
                            retained_strings.reserve(attribute.value.len())?;
                        }
                        result.historical_namespace |= historical_document
                            || name.namespace.as_deref() == Some("http://www.ofdspec.org")
                            || attributes.iter().any(|a| {
                                a.name.namespace.as_deref() == Some("http://www.ofdspec.org")
                            });
                        let index = result.nodes.len();
                        result.nodes.push(XmlNode {
                            name,
                            attributes,
                            text: String::new(),
                            children: Vec::new(),
                        });
                        if let Some(&parent) = selected.last() {
                            result.nodes[parent].children.push(index);
                        } else {
                            result.roots.push(index);
                        }
                        selected.push(index);
                    }
                }
                XmlEvent::EndElement { .. } => {
                    selected.pop();
                    depth = depth.saturating_sub(1);
                }
                XmlEvent::Characters(text) | XmlEvent::CData(text) | XmlEvent::Whitespace(text) => {
                    if let Some(&node) = selected.last() {
                        retained_strings.reserve(text.len())?;
                        result.nodes[node].text.push_str(&text);
                    }
                }
                _ => {}
            }
        }
        Ok(result)
    }
}
