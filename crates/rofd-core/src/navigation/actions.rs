//! Shared, inert action and destination parsing for outlines and page links.

use std::{collections::HashMap, fmt};

use super::{
    budget::StringBudget,
    xml::{native, NavigationXml, XmlNode},
    Action, ActionEvent, ActionKind, Destination, DestinationMode,
};
use crate::{Error, Result, Strictness, Warning, WarningCode};

pub(crate) struct ActionParser<'a> {
    path: &'a str,
    strictness: Strictness,
    pages: HashMap<u64, Option<usize>>,
    budget: StringBudget,
    pub(crate) warnings: Vec<Warning>,
}

impl<'a> ActionParser<'a> {
    pub(crate) fn new(
        path: &'a str,
        strictness: Strictness,
        pages: &[(u64, usize)],
        string_limit: u64,
    ) -> Self {
        let mut indices = HashMap::new();
        for &(id, index) in pages {
            indices
                .entry(id)
                .and_modify(|entry| *entry = None)
                .or_insert(Some(index));
        }
        Self {
            path,
            strictness,
            pages: indices,
            budget: StringBudget::new(string_limit),
            warnings: Vec::new(),
        }
    }

    pub(crate) fn string(&mut self, value: &str) -> Result<String> {
        self.budget.copy(value)
    }

    pub(crate) fn required<'n>(&mut self, node: &'n XmlNode, field: &str) -> Result<&'n str> {
        node.attribute(field).ok_or_else(|| {
            self.error(format_args!(
                "{} is missing required {field} attribute",
                node.name.local_name
            ))
        })
    }

    fn type_name(&mut self, node: &XmlNode) -> Result<String> {
        if native(&node.name) {
            self.string(&node.name.local_name)
        } else {
            self.budget.format(format_args!(
                "{{{}}}{}",
                node.name.namespace.as_deref().unwrap_or_default(),
                node.name.local_name
            ))
        }
    }

    fn error(&mut self, arguments: fmt::Arguments<'_>) -> Error {
        let built = (|| {
            let message = self.budget.format(arguments)?;
            let path = self.budget.copy(self.path)?;
            Ok(Error::InvalidStructure { path, message })
        })();
        match built {
            Ok(error) | Err(error) => error,
        }
    }

    pub(crate) fn warn(&mut self, code: WarningCode, arguments: fmt::Arguments<'_>) -> Result<()> {
        let message = self.budget.format(arguments)?;
        let path = self.budget.copy(self.path)?;
        self.warnings.push(Warning {
            code,
            path,
            message,
        });
        Ok(())
    }

    pub(crate) fn invalid(&mut self, arguments: fmt::Arguments<'_>) -> Result<()> {
        if self.strictness == Strictness::Strict {
            Err(self.error(arguments))
        } else {
            self.warn(WarningCode::NavigationInvalid, arguments)
        }
    }

    pub(crate) fn recover<T>(&mut self, result: Result<T>) -> Result<Option<T>> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error @ (Error::InvalidStructure { .. } | Error::InvalidValue { .. }))
                if self.strictness == Strictness::Lenient =>
            {
                self.warn(WarningCode::NavigationInvalid, format_args!("{error}"))?;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn boolean(&mut self, node: &XmlNode, field: &str, default: bool) -> Result<bool> {
        match node.attribute(field) {
            None => Ok(default),
            Some("true" | "1") => Ok(true),
            Some("false" | "0") => Ok(false),
            Some(value) => {
                self.invalid(format_args!("invalid {field} boolean {value:?}"))?;
                Ok(default)
            }
        }
    }

    pub(crate) fn actions(
        &mut self,
        xml: &NavigationXml,
        container: usize,
        bookmarks: &HashMap<String, Option<Destination>>,
    ) -> Result<Vec<Action>> {
        xml.children(container, "Action")
            .map(|node| self.action(xml, node, bookmarks))
            .collect()
    }

    fn action(
        &mut self,
        xml: &NavigationXml,
        index: usize,
        bookmarks: &HashMap<String, Option<Destination>>,
    ) -> Result<Action> {
        let node = &xml.nodes[index];
        let event = match node.attribute("Event") {
            Some("DO") => ActionEvent::DocumentOpen,
            Some("PO") => ActionEvent::PageOpen,
            Some("CLICK") => ActionEvent::Click,
            Some(other) => {
                self.warn(
                    WarningCode::NavigationUnsupported,
                    format_args!("unsupported action event {other:?}"),
                )?;
                ActionEvent::Unknown(self.string(other)?)
            }
            None => {
                self.invalid(format_args!("Action is missing required Event attribute"))?;
                ActionEvent::Unknown(String::new())
            }
        };
        // Only immediate children participate in the schema's choice. An
        // extension wrapper can never smuggle a nested native Goto into it.
        let choices: Vec<_> = node
            .children
            .iter()
            .copied()
            .filter(|&child| !xml.nodes[child].is("Region"))
            .collect();
        let kind = if choices.len() == 1 {
            let body = &xml.nodes[choices[0]];
            let result = if body.is("Goto") {
                self.goto(xml, choices[0], bookmarks)
            } else if body.is("URI") {
                self.uri(body)
            } else if body.is("GotoA") {
                self.attachment(body)
            } else {
                let type_name = self.type_name(body)?;
                self.warn(
                    WarningCode::NavigationUnsupported,
                    format_args!("unsupported action type {type_name:?}"),
                )?;
                Ok(ActionKind::Unknown { type_name })
            };
            match self.recover(result)? {
                Some(kind) => kind,
                None => ActionKind::Unknown {
                    type_name: self.type_name(body)?,
                },
            }
        } else {
            self.invalid(format_args!(
                "Action requires exactly one action-type child"
            ))?;
            ActionKind::Unknown {
                type_name: self.string("Action")?,
            }
        };
        Ok(Action { event, kind })
    }

    fn uri(&mut self, node: &XmlNode) -> Result<ActionKind> {
        let uri = self.required(node, "URI")?;
        let uri = self.string(uri)?;
        let base = node
            .attribute("Base")
            .map(|base| self.string(base))
            .transpose()?;
        Ok(ActionKind::Uri { uri, base })
    }

    fn attachment(&mut self, node: &XmlNode) -> Result<ActionKind> {
        let attachment_id = self.required(node, "AttachID")?;
        let attachment_id = self.string(attachment_id)?;
        let new_window = self.boolean(node, "NewWindow", true)?;
        Ok(ActionKind::Attachment {
            attachment_id,
            new_window,
        })
    }

    fn goto(
        &mut self,
        xml: &NavigationXml,
        index: usize,
        bookmarks: &HashMap<String, Option<Destination>>,
    ) -> Result<ActionKind> {
        let children = &xml.nodes[index].children;
        if children.len() != 1 {
            self.invalid(format_args!(
                "Goto requires exactly one Dest or Bookmark child"
            ))?;
            return Ok(ActionKind::Goto {
                destination: None,
                bookmark: None,
            });
        }
        let child = children[0];
        let node = &xml.nodes[child];
        if node.is("Dest") {
            Ok(ActionKind::Goto {
                destination: self.destination(xml, child)?,
                bookmark: None,
            })
        } else if node.is("Bookmark") {
            let required = self.required(node, "Name");
            let bookmark = self
                .recover(required)?
                .map(|name| self.string(name))
                .transpose()?;
            let destination = bookmark
                .as_ref()
                .and_then(|name| bookmarks.get(name))
                .and_then(Option::as_ref)
                .map(|destination| self.clone_destination(destination))
                .transpose()?;
            if destination.is_none() && bookmark.is_some() {
                self.invalid(format_args!(
                    "unresolved or ambiguous bookmark {:?}",
                    bookmark.as_deref().unwrap_or_default()
                ))?;
            }
            Ok(ActionKind::Goto {
                destination,
                bookmark,
            })
        } else {
            self.invalid(format_args!(
                "Goto requires a native Dest or Bookmark child"
            ))?;
            Ok(ActionKind::Goto {
                destination: None,
                bookmark: None,
            })
        }
    }

    fn clone_destination(&mut self, destination: &Destination) -> Result<Destination> {
        // A short bookmark reference can expand to a large unknown mode string.
        // Charge before copying; sharing only the raw arena cannot bound this.
        let mode = match &destination.mode {
            DestinationMode::Unknown(mode) => DestinationMode::Unknown(self.string(mode)?),
            known => known.clone(),
        };
        Ok(Destination {
            page_id: destination.page_id,
            page_index: destination.page_index,
            mode,
            left: destination.left,
            top: destination.top,
            right: destination.right,
            bottom: destination.bottom,
            zoom: destination.zoom,
        })
    }

    pub(crate) fn destination(
        &mut self,
        xml: &NavigationXml,
        index: usize,
    ) -> Result<Option<Destination>> {
        let result = self.parse_destination(xml, index);
        self.recover(result)
    }

    fn parse_destination(&mut self, xml: &NavigationXml, index: usize) -> Result<Destination> {
        let node = &xml.nodes[index];
        let mode = match self.required(node, "Type")? {
            "XYZ" => DestinationMode::Xyz,
            "Fit" => DestinationMode::Fit,
            "FitH" => DestinationMode::FitH,
            "FitV" => DestinationMode::FitV,
            "FitR" => DestinationMode::FitR,
            other => {
                self.warn(
                    WarningCode::NavigationUnsupported,
                    format_args!("unsupported destination mode {other:?}"),
                )?;
                DestinationMode::Unknown(self.string(other)?)
            }
        };
        let id = self
            .numeric_value(xml, index, "PageID")?
            .ok_or_else(|| self.error(format_args!("Dest is missing required PageID")))?;
        let page_id = id
            .trim()
            .parse::<u64>()
            .map_err(|_| self.error(format_args!("invalid destination PageID {id:?}")))?;
        let left = self.coordinate(xml, index, "Left")?;
        let top = self.coordinate(xml, index, "Top")?;
        let right = self.coordinate(xml, index, "Right")?;
        let bottom = self.coordinate(xml, index, "Bottom")?;
        let zoom = self.coordinate(xml, index, "Zoom")?;
        let page_index = self.pages.get(&page_id).copied().flatten();
        if page_index.is_none() {
            self.invalid(format_args!(
                "destination PageID {page_id} is unresolved or ambiguous"
            ))?;
        }
        Ok(Destination {
            page_id,
            page_index,
            mode,
            left,
            top,
            right,
            bottom,
            zoom,
        })
    }

    fn coordinate(
        &mut self,
        xml: &NavigationXml,
        index: usize,
        field: &str,
    ) -> Result<Option<f64>> {
        self.numeric_value(xml, index, field)?
            .map(|value| {
                value
                    .trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|n| n.is_finite())
                    .ok_or_else(|| {
                        self.error(format_args!("invalid finite destination {field} {value:?}"))
                    })
            })
            .transpose()
    }

    fn numeric_value<'x>(
        &mut self,
        xml: &'x NavigationXml,
        index: usize,
        field: &str,
    ) -> Result<Option<&'x str>> {
        let children: Vec<_> = xml.children(index, field).collect();
        let attribute = xml.nodes[index].attribute(field);
        if !children.is_empty() {
            self.warn(WarningCode::NavigationCompatibility, format_args!(
                "nonstandard Dest/{field} numeric child {}; the attribute takes priority when present",
                if attribute.is_some() { "ignored" } else { "used" }
            ))?;
        }
        if let Some(value) = attribute {
            return Ok(Some(value));
        }
        if children.len() > 1 {
            return Err(self.error(format_args!(
                "ambiguous repeated Dest/{field} numeric children"
            )));
        }
        children
            .first()
            .map(|&child| {
                let node = &xml.nodes[child];
                if !node.children.is_empty() {
                    Err(self.error(format_args!(
                        "Dest/{field} numeric child contains nested elements"
                    )))
                } else {
                    Ok(node.text.as_str())
                }
            })
            .transpose()
    }
}
