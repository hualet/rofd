//! Inert navigation data, parsed lazily and independently of page content.

pub(crate) mod actions;
mod budget;
pub(crate) mod deferred;
pub(crate) mod region;
pub(crate) mod xml;

use std::collections::HashMap;

use crate::{Error, ResourceLimits, Result, Strictness, Warning, WarningCode};
use actions::ActionParser;
use xml::NavigationXml;

/// Event that activates an action. The library never executes actions.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ActionEvent {
    /// Document open (`DO`).
    DocumentOpen,
    /// Page open (`PO`).
    PageOpen,
    /// User click (`CLICK`).
    Click,
    /// An unrecognized event, preserved verbatim.
    Unknown(String),
}

/// OFD destination view mode.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DestinationMode {
    /// Position and zoom (`XYZ`).
    Xyz,
    /// Fit the page.
    Fit,
    /// Fit horizontally.
    FitH,
    /// Fit vertically.
    FitV,
    /// Fit the specified rectangle.
    FitR,
    /// An unrecognized mode, preserved verbatim.
    Unknown(String),
}

/// A document destination. Coordinates are absolute page millimetres.
///
/// Omitted coordinates and zoom remain absent; a reader chooses its defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct Destination {
    /// Original OFD page object identifier, not a page index.
    pub page_id: u64,
    /// Resolved zero-based page index, or absent for an unresolved identifier.
    pub page_index: Option<usize>,
    /// Requested view mode.
    pub mode: DestinationMode,
    /// Optional left coordinate in millimetres.
    pub left: Option<f64>,
    /// Optional top coordinate in millimetres.
    pub top: Option<f64>,
    /// Optional right coordinate in millimetres.
    pub right: Option<f64>,
    /// Optional bottom coordinate in millimetres.
    pub bottom: Option<f64>,
    /// Raw optional zoom value, including zero when explicitly specified.
    pub zoom: Option<f64>,
}

/// Action payload, exposed as data only. URIs and attachments are never opened.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ActionKind {
    /// Navigate within this document.
    Goto {
        /// Resolved destination, absent when malformed or unavailable.
        destination: Option<Destination>,
        /// Original named bookmark reference, when present.
        bookmark: Option<String>,
    },
    /// A URI and optional base, both preserved without interpretation.
    Uri {
        /// Original URI string, which can include non-network schemes.
        uri: String,
        /// Optional base URI.
        base: Option<String>,
    },
    /// Open an attachment (`GotoA`).
    Attachment {
        /// Original string attachment IDREF.
        attachment_id: String,
        /// Whether a new window was requested; defaults to true.
        new_window: bool,
    },
    /// An unsupported or unusable action.
    Unknown {
        /// Element name; foreign namespaces use `{namespace}local` notation.
        type_name: String,
    },
}

/// One ordered action and its activating event.
#[derive(Clone, Debug, PartialEq)]
pub struct Action {
    /// Activating event.
    pub event: ActionEvent,
    /// Inert action payload.
    pub kind: ActionKind,
}

/// One outline node in a document-wide preorder array.
#[derive(Clone, Debug, PartialEq)]
pub struct OutlineNode {
    /// Display title.
    pub title: String,
    /// Parent index in the same outline array, absent for roots.
    pub parent: Option<usize>,
    /// First child index in the same outline array.
    pub first_child: Option<usize>,
    /// Next sibling index in the same outline array.
    pub next_sibling: Option<usize>,
    /// Initial expansion state; defaults to true.
    pub expanded: bool,
    /// Actions in their original document order.
    pub actions: Vec<Action>,
}

/// Shared with future page-link parsing so named destinations resolve once.
#[derive(Debug)]
pub(crate) struct NavigationData {
    pub(crate) outlines: Vec<OutlineNode>,
}

pub(crate) fn parse_navigation(
    bytes: &[u8],
    path: &str,
    limits: &ResourceLimits,
    strictness: Strictness,
    pages: &[(u64, usize)],
    bookmarks: &BookmarkData,
) -> Result<(NavigationData, Vec<Warning>)> {
    let mut remaining = limits.clone();
    remaining.max_page_objects = limits
        .max_page_objects
        .checked_sub(bookmarks.xml_nodes)
        .ok_or_else(|| Error::LimitExceeded("navigation XML node budget exceeded".into()))?;
    remaining.max_entry_size = limits
        .max_entry_size
        .checked_sub(bookmarks.arena_bytes)
        .ok_or_else(|| Error::LimitExceeded("navigation XML string budget exceeded".into()))?;
    let xml = NavigationXml::read(bytes, path, &remaining, true, false)?;
    let string_limit = limits
        .max_entry_size
        .checked_sub(bookmarks.string_bytes)
        .ok_or_else(|| Error::LimitExceeded("expanded navigation string budget exceeded".into()))?;
    let mut parser = ActionParser::new(path, strictness, pages, string_limit);
    if xml.historical_namespace {
        parser.warn(
            WarningCode::NavigationCompatibility,
            format_args!("accepted historical OFD navigation namespace http://www.ofdspec.org"),
        )?;
    }
    let mut data = NavigationData {
        outlines: Vec::new(),
    };
    let mut pending = Vec::new();
    for root in xml
        .roots
        .iter()
        .copied()
        .rev()
        .filter(|&i| xml.nodes[i].is("Outlines"))
    {
        for node in xml.children(root, "OutlineElem").rev() {
            pending.push((node, None, 1usize));
        }
    }
    let mut last_children: Vec<Option<usize>> = Vec::new();
    let mut last_root: Option<usize> = None;
    while let Some((node, parent, depth)) = pending.pop() {
        if depth > limits.max_page_block_depth {
            return Err(Error::LimitExceeded(format!(
                "navigation depth exceeds {} in {path}",
                limits.max_page_block_depth
            )));
        }
        let required = parser.required(&xml.nodes[node], "Title");
        let title = parser.recover(required)?.unwrap_or_default();
        let title = parser.string(title)?;
        let expanded = parser.boolean(&xml.nodes[node], "Expanded", true)?;
        let mut actions = Vec::new();
        for container in xml.children(node, "Actions") {
            actions.extend(parser.actions(&xml, container, &bookmarks.bookmarks)?);
        }
        let index = data.outlines.len();
        data.outlines.push(OutlineNode {
            title,
            parent,
            first_child: None,
            next_sibling: None,
            expanded,
            actions,
        });
        last_children.push(None);
        let previous = if let Some(parent) = parent {
            if data.outlines[parent].first_child.is_none() {
                data.outlines[parent].first_child = Some(index);
            }
            last_children[parent].replace(index)
        } else {
            last_root.replace(index)
        };
        if let Some(previous) = previous {
            data.outlines[previous].next_sibling = Some(index);
        }
        for child in xml.children(node, "OutlineElem").rev() {
            pending.push((child, Some(index), depth + 1));
        }
    }
    Ok((data, parser.warnings))
}

#[derive(Debug)]
pub(crate) struct BookmarkData {
    pub(crate) bookmarks: HashMap<String, Option<Destination>>,
    pub(crate) warnings: Vec<Warning>,
    xml_nodes: usize,
    arena_bytes: u64,
    string_bytes: u64,
}

pub(crate) fn parse_bookmarks(
    bytes: &[u8],
    path: &str,
    limits: &ResourceLimits,
    strictness: Strictness,
    pages: &[(u64, usize)],
) -> Result<BookmarkData> {
    let xml = NavigationXml::read(bytes, path, limits, false, true)?;
    let mut parser = ActionParser::new(path, strictness, pages, limits.max_entry_size);
    if xml.historical_namespace {
        parser.warn(
            WarningCode::NavigationCompatibility,
            format_args!("accepted historical OFD navigation namespace http://www.ofdspec.org"),
        )?;
    }
    let mut data = BookmarkData {
        bookmarks: HashMap::new(),
        warnings: Vec::new(),
        xml_nodes: xml.nodes.len(),
        arena_bytes: xml.retained_bytes,
        string_bytes: 0,
    };
    for root in xml
        .roots
        .iter()
        .copied()
        .filter(|&i| xml.nodes[i].is("Bookmarks"))
    {
        for node in xml.children(root, "Bookmark") {
            let required = parser.required(&xml.nodes[node], "Name");
            let Some(name) = parser.recover(required)? else {
                continue;
            };
            let name = parser.string(name)?;
            let destinations: Vec<_> = xml.children(node, "Dest").collect();
            let destination = if destinations.len() == 1 {
                parser.destination(&xml, destinations[0])?
            } else {
                parser.invalid(format_args!(
                    "Bookmark requires exactly one direct Dest child"
                ))?;
                None
            };
            match data.bookmarks.entry(name) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    parser.invalid(format_args!("duplicate bookmark name {:?}", entry.key()))?;
                    entry.insert(None);
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(destination);
                }
            }
        }
    }

    data.string_bytes = parser.string_bytes();
    data.warnings = parser.warnings;
    Ok(data)
}
