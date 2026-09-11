//! Capture action XML without interpreting it or attaching navigation errors to
//! page rendering. Graphic owners retain only cheap shared handles.

use super::xml::{native, NavigationXml, XmlCapture};
use crate::{raw, Error, ResourceLimits, Result};
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};
use xml::reader::{EventReader, XmlEvent};

pub(crate) type Actions = Option<Arc<DeferredActions>>;

#[derive(Debug, PartialEq)]
pub(crate) struct DeferredActions {
    pub(crate) source: Arc<ActionSource>,
    pub(crate) containers: Vec<usize>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ActionSource {
    pub(crate) path: String,
    pub(crate) xml: Option<NavigationXml>,
}

#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq)]
enum Owner {
    Page,
    Layer,
    Group,
    Path,
    Text,
    Image,
    Composite,
    Annotation,
    Appearance,
    VectorContent,
}

pub(crate) struct ExtractedActions {
    pub(crate) sanitized: Vec<u8>,
    owners: BTreeMap<Owner, VecDeque<(usize, Arc<DeferredActions>)>>,
    next_owner: BTreeMap<Owner, usize>,
    failed: Actions,
}

#[derive(Clone, Copy)]
enum Scope {
    Page,
    PageContent,
    Layer,
    Group,
    Leaf,
    Annotations,
    AnnotationPage,
    PageAnnot,
    Annotation,
    Appearance,
    Resource,
    GraphicUnits,
    VectorUnit,
    VectorContent,
    Other,
}

fn scope(parent: Option<Scope>, name: &str) -> (Scope, Option<Owner>) {
    use Scope::*;
    match (parent, name) {
        (None, "Page") => (Page, Some(Owner::Page)),
        (None, "Annotations") => (Annotations, None),
        (None, "PageAnnot") => (PageAnnot, None),
        (None, "Res") => (Resource, None),
        (Some(Page), "Content") => (PageContent, None),
        (Some(PageContent), "Layer") => (Layer, Some(Owner::Layer)),
        (Some(Annotations), "Page") => (AnnotationPage, None),
        (Some(PageAnnot | AnnotationPage), "Annot") => (Annotation, Some(Owner::Annotation)),
        (Some(Annotation), "Appearance") => (Appearance, Some(Owner::Appearance)),
        (Some(Resource), "CompositeGraphicUnits") => (GraphicUnits, None),
        (Some(GraphicUnits), "CompositeGraphicUnit") => (VectorUnit, None),
        (Some(VectorUnit), "Content") => (VectorContent, Some(Owner::VectorContent)),
        (Some(Layer | Group | Appearance | VectorContent), "PageBlock") => {
            (Group, Some(Owner::Group))
        }
        (Some(Layer | Group | Appearance | VectorContent), "PathObject") => {
            (Leaf, Some(Owner::Path))
        }
        (Some(Layer | Group | Appearance | VectorContent), "TextObject") => {
            (Leaf, Some(Owner::Text))
        }
        (Some(Layer | Group | Appearance | VectorContent), "ImageObject") => {
            (Leaf, Some(Owner::Image))
        }
        (Some(Layer | Group | Appearance | VectorContent), "CompositeObject") => {
            (Leaf, Some(Owner::Composite))
        }
        _ => (Other, None),
    }
}

pub(crate) fn extract(
    bytes: &[u8],
    path: &str,
    limits: &ResourceLimits,
) -> Result<ExtractedActions> {
    let mut writer = xml::EventWriter::new(Vec::new());
    let mut scopes = Vec::new();
    let mut native_ancestors: Vec<bool> = Vec::new();
    let mut historical_ancestors: Vec<bool> = Vec::new();
    let mut owner_stack: Vec<Option<(Owner, usize, bool, bool)>> = Vec::new();
    let mut owner_counts: BTreeMap<Owner, usize> = BTreeMap::new();
    // Keep only action-bearing positions. Empty resource catalog owners must
    // not consume a page's object budget or allocate one retained slot each.
    let mut owners: BTreeMap<(Owner, usize), Vec<usize>> = BTreeMap::new();
    let mut arena = Some(XmlCapture::new(limits));
    let mut capture_depth = 0usize;
    let mut capture_enabled = false;
    for event in EventReader::new(bytes) {
        let event = event.map_err(|e| Error::Xml {
            path: path.to_owned(),
            message: e.to_string(),
        })?;
        let mut suppressed = capture_depth > 0;
        match &event {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                let action_owner = if capture_depth == 0 && name.local_name == "Actions" {
                    owner_stack.last().copied().flatten()
                } else {
                    None
                };
                if let Some((kind, ordinal, owner_native, owner_historical)) = action_owner {
                    suppressed = true;
                    capture_depth = 1;
                    capture_enabled = owner_native && native(name);
                    if capture_enabled {
                        let captured = arena.as_mut().map(|arena| {
                            arena.xml.historical_namespace |= owner_historical;
                            arena.start(name.clone(), attributes.clone())
                        });
                        match captured {
                            Some(Ok(node)) => owners.entry((kind, ordinal)).or_default().push(node),
                            _ => arena = None,
                        }
                    }
                } else if capture_depth > 0 {
                    capture_depth += 1;
                    if capture_enabled
                        && arena.as_mut().is_some_and(|arena| {
                            arena.start(name.clone(), attributes.clone()).is_err()
                        })
                    {
                        arena = None;
                    }
                }
                let (current_scope, owner) = if !suppressed {
                    scope(scopes.last().copied(), &name.local_name)
                } else {
                    (Scope::Other, None)
                };
                let native_owner = native(name) && native_ancestors.last().copied().unwrap_or(true);
                let historical_owner = historical_ancestors.last().copied().unwrap_or(false)
                    || name.namespace.as_deref() == Some("http://www.ofdspec.org")
                    || attributes.iter().any(|attribute| {
                        attribute.name.namespace.as_deref() == Some("http://www.ofdspec.org")
                    });
                let owner_index = owner.map(|kind| {
                    let next = owner_counts.entry(kind).or_default();
                    let ordinal = *next;
                    // There cannot be more owners than bytes in this already
                    // bounded source slice, so this ordinal cannot overflow.
                    *next += 1;
                    (kind, ordinal, native_owner, historical_owner)
                });
                scopes.push(current_scope);
                native_ancestors.push(native_owner);
                historical_ancestors.push(historical_owner);
                owner_stack.push(owner_index);
            }
            XmlEvent::EndElement { .. } => {
                if capture_depth > 0 {
                    capture_depth -= 1;
                    if capture_enabled {
                        if let Some(arena) = &mut arena {
                            arena.end();
                        }
                    }
                    if capture_depth == 0 {
                        capture_enabled = false;
                    }
                }
                scopes.pop();
                native_ancestors.pop();
                historical_ancestors.pop();
                owner_stack.pop();
            }
            XmlEvent::Characters(value) | XmlEvent::CData(value) | XmlEvent::Whitespace(value)
                if capture_depth > 0
                    && capture_enabled
                    && arena
                        .as_mut()
                        .is_some_and(|arena| arena.text(value).is_err()) =>
            {
                arena = None;
            }
            _ => {}
        }
        if !suppressed {
            if let Some(event) = event.as_writer_event() {
                writer.write(event).map_err(|e| Error::Xml {
                    path: path.to_owned(),
                    message: e.to_string(),
                })?;
            }
        }
    }
    let source = Arc::new(ActionSource {
        path: path.to_owned(),
        xml: arena.map(|arena| arena.xml),
    });
    // Once this source's arena limit is reached, retain one shared failure
    // marker, not one marker per remaining XML action or owner.
    let failed = source.xml.is_none().then(|| {
        Arc::new(DeferredActions {
            source: source.clone(),
            containers: vec![0],
        })
    });
    let mut queues: BTreeMap<Owner, VecDeque<(usize, Arc<DeferredActions>)>> = BTreeMap::new();
    for ((kind, ordinal), containers) in owners {
        queues.entry(kind).or_default().push_back((
            ordinal,
            Arc::new(DeferredActions {
                source: source.clone(),
                containers,
            }),
        ));
    }
    Ok(ExtractedActions {
        sanitized: writer.into_inner(),
        owners: queues,
        next_owner: BTreeMap::new(),
        failed,
    })
}

impl ExtractedActions {
    fn take(&mut self, kind: Owner) -> Actions {
        if self.failed.is_some() {
            return self.failed.clone();
        }
        let next = self.next_owner.entry(kind).or_default();
        let ordinal = *next;
        *next += 1;
        let queue = self.owners.get_mut(&kind)?;
        if queue.front().is_some_and(|(index, _)| *index == ordinal) {
            queue.pop_front().map(|(_, actions)| actions)
        } else {
            None
        }
    }
    pub(crate) fn page(&mut self, page: &mut raw::PageRoot) {
        page.actions = self.take(Owner::Page);
        if let Some(content) = &mut page.content {
            for layer in &mut content.layers {
                layer.actions = self.take(Owner::Layer);
                self.objects(&mut layer.objects);
            }
        }
    }
    fn objects(&mut self, objects: &mut [raw::GraphicUnit]) {
        for object in objects {
            match object {
                raw::GraphicUnit::Path(envelope) => {
                    let actions = self.take(Owner::Path);
                    if let Some(object) = &mut envelope.object {
                        object.actions = actions;
                    }
                }
                raw::GraphicUnit::Text(envelope) => {
                    let actions = self.take(Owner::Text);
                    if let Some(object) = &mut envelope.object {
                        object.actions = actions;
                    }
                }
                raw::GraphicUnit::Image(envelope) => {
                    let actions = self.take(Owner::Image);
                    if let Some(object) = &mut envelope.object {
                        object.actions = actions;
                    }
                }
                raw::GraphicUnit::Group(group) => {
                    group.actions = self.take(Owner::Group);
                    self.objects(&mut group.objects);
                }
                raw::GraphicUnit::Composite(object) => object.actions = self.take(Owner::Composite),
            }
        }
    }
    pub(crate) fn annotations(&mut self, annotations: &mut [raw::AnnotEntry]) {
        for annotation in annotations {
            annotation.actions = self.take(Owner::Annotation);
            if let Some(appearance) = &mut annotation.appearance {
                appearance.actions = self.take(Owner::Appearance);
                self.objects(&mut appearance.objects);
            }
        }
    }
    pub(crate) fn resources(&mut self, root: &mut raw::ResourceRoot) {
        for unit in root
            .composite_graphic_units
            .iter_mut()
            .flat_map(|units| units.entries.iter_mut())
        {
            if let Some(content) = &mut unit.content {
                content.actions = self.take(Owner::VectorContent);
                self.objects(&mut content.objects);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_capture_retains_one_failure_marker_not_unbounded_container_indices() {
        let limits = ResourceLimits {
            max_page_objects: 1,
            ..ResourceLimits::default()
        };
        let xml = format!("<Page>{}</Page>", "<Actions/>".repeat(64));
        let mut extracted = extract(xml.as_bytes(), "page.xml", &limits).unwrap();
        let actions = extracted.take(Owner::Page).unwrap();
        assert!(actions.source.xml.is_none());
        assert_eq!(actions.containers.len(), 1);
    }
}
