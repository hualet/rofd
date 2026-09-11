//! Inert page link extraction over the already expanded content model.

use crate::navigation::{actions::ActionParser, deferred::Actions, region};
use crate::{
    Action, Destination, Document, Error, Page, PageObject, Rect, Result, Transform, Warning,
    WarningCode,
};
use std::collections::{HashMap, HashSet};

/// One source action and its conservative clickable areas in page millimetres.
///
/// Areas are bounding rectangles, not exact curve or clipping hit masks. Each
/// source action receives its own entry; the action vector preserves room for
/// future grouping without changing the public model. Actions are never run.
#[derive(Clone, Debug, PartialEq)]
pub struct PageLink {
    /// Conservative independent region rectangles in physical-page coordinates.
    pub regions: Vec<Rect>,
    /// Inert actions associated with these regions, currently one per entry.
    pub actions: Vec<Action>,
}

pub(crate) fn build<'a>(
    page: &'a Page,
    document: &'a Document,
    anchors: &'a [(usize, Actions)],
) -> Result<(Vec<PageLink>, Vec<Warning>, bool)> {
    let mut builder = Builder {
        document,
        page_box: page.size(),
        parser: ActionParser::new(
            "page",
            document.strictness(),
            &document.page_indices(),
            page.resource_limits().max_entry_size,
        ),
        entries: page.resource_limits().max_page_objects,
        commands: page.resource_limits().max_path_commands,
        links: Vec::new(),
        historical: HashSet::new(),
        bookmarks_used: false,
        empty_bookmarks: HashMap::new(),
    };
    let mut anchors = anchors.iter().peekable();
    for index in 0..=page.layers().len() {
        while let Some((anchor, actions)) = anchors.peek() {
            if *anchor != index {
                break;
            }
            builder.actions(actions, None, Transform::IDENTITY, Transform::IDENTITY)?;
            anchors.next();
        }
        if let Some(layer) = page.layers().get(index) {
            builder.charge(1)?;
            builder.actions(
                &layer.actions,
                None,
                Transform::IDENTITY,
                Transform::IDENTITY,
            )?;
            builder.objects(layer.objects(), Transform::IDENTITY)?;
        }
    }
    for annotation in document
        .page_annotations_ref()?
        .iter()
        .filter(|a| a.page_ref == page.object_id() && a.visible())
    {
        builder.charge(1)?;
        builder.actions(
            &annotation.actions,
            annotation.action_boundary,
            Transform::IDENTITY,
            Transform::IDENTITY,
        )?;
        builder.actions(
            &annotation.appearance_actions,
            Some(annotation.boundary()),
            Transform::IDENTITY,
            Transform::IDENTITY,
        )?;
        builder.objects(
            annotation.objects(),
            region::translation(annotation.boundary())?,
        )?;
    }
    Ok((
        builder.links,
        builder.parser.warnings,
        builder.bookmarks_used,
    ))
}

struct Builder<'a> {
    document: &'a Document,
    page_box: Rect,
    parser: ActionParser<'a>,
    entries: usize,
    commands: usize,
    links: Vec<PageLink>,
    historical: HashSet<usize>,
    bookmarks_used: bool,
    empty_bookmarks: HashMap<String, Option<Destination>>,
}

impl<'a> Builder<'a> {
    fn charge(&mut self, count: usize) -> Result<()> {
        self.entries = self.entries.checked_sub(count).ok_or_else(|| {
            Error::LimitExceeded(
                "effective page link objects and XML nodes exceed max_page_objects".into(),
            )
        })?;
        Ok(())
    }
    fn objects(&mut self, objects: &'a [PageObject], parent: Transform) -> Result<()> {
        for object in objects {
            self.charge(1)?;
            match object {
                PageObject::Path(object) => self.actions(
                    &object.actions,
                    object.action_boundary,
                    object.transform(),
                    parent,
                )?,
                PageObject::Text(object) => self.actions(
                    &object.actions,
                    Some(object.boundary()),
                    object.transform(),
                    parent,
                )?,
                PageObject::Image(object) => self.actions(
                    &object.actions,
                    Some(object.boundary()),
                    object.transform(),
                    parent,
                )?,
                PageObject::Group(object) => {
                    self.actions(&object.actions, None, Transform::IDENTITY, parent)?;
                    self.objects(object.objects(), parent)?;
                }
                PageObject::Composite(object) => {
                    self.actions(
                        &object.actions,
                        object.action_boundary,
                        object.transform(),
                        parent,
                    )?;
                    let child = (|| {
                        object
                            .transform()
                            .then(region::translation(object.boundary())?)?
                            .then(parent)
                    })();
                    let Some(child) = self.parser.recover(child)? else {
                        continue;
                    };
                    self.actions(&object.content_actions, None, Transform::IDENTITY, child)?;
                    self.objects(object.objects(), child)?;
                }
                PageObject::Unsupported(_) => {}
            }
        }
        Ok(())
    }
    fn actions(
        &mut self,
        actions: &'a Actions,
        boundary: Option<Rect>,
        local: Transform,
        parent: Transform,
    ) -> Result<()> {
        let Some(actions) = actions else {
            return Ok(());
        };
        let source = &actions.source;
        let xml = source.xml.as_ref().ok_or_else(|| {
            Error::LimitExceeded("deferred action XML exceeded navigation limits".into())
        })?;
        self.parser.set_path(&source.path);
        if xml.historical_namespace
            && self
                .historical
                .insert(std::sync::Arc::as_ptr(source) as usize)
        {
            self.parser.warn(
                WarningCode::NavigationCompatibility,
                format_args!("accepted historical OFD navigation namespace http://www.ofdspec.org"),
            )?;
        }
        for &container in &actions.containers {
            let mut pending = vec![container];
            while let Some(index) = pending.pop() {
                self.charge(1)?;
                pending.extend(xml.nodes[index].children.iter().copied());
            }
            for index in xml.children(container, "Action") {
                let region_nodes: Vec<_> = xml.children(index, "Region").collect();
                let geometry = (|| {
                    if region_nodes.len() > 1 {
                        Err(self
                            .parser
                            .error(format_args!("Action has multiple Region children")))
                    } else if let Some(&region) = region_nodes.first() {
                        let transform = local
                            .then(
                                boundary
                                    .map(region::translation)
                                    .transpose()?
                                    .unwrap_or(Transform::IDENTITY),
                            )?
                            .then(parent)?;
                        region::areas(xml, region, transform, &mut self.commands, &mut self.parser)
                    } else {
                        boundary
                            .map(|boundary| region::map_rect(boundary, parent))
                            .unwrap_or_else(|| region::map_rect(self.page_box, Transform::IDENTITY))
                            .map(|rect| vec![rect])
                    }
                })();
                let Some(regions) = self.parser.recover(geometry)? else {
                    continue;
                };
                let needs_bookmark = xml
                    .children(index, "Goto")
                    .any(|goto| xml.children(goto, "Bookmark").next().is_some());
                let bookmarks = if needs_bookmark {
                    self.bookmarks_used = true;
                    &self.document.bookmarks()?.bookmarks
                } else {
                    &self.empty_bookmarks
                };
                let action = self.parser.action(xml, index, bookmarks)?;
                self.links.push(PageLink {
                    regions,
                    actions: vec![action],
                });
            }
        }
        Ok(())
    }
}
