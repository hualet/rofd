# Poppler-style C Text Semantic API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add idiomatic Rust page-text semantics and expose text extraction, layout, search, and rectangular selection through a Poppler-like page-level C API without changing any published ABI v1 symbol.

**Architecture:** `rofd-core` owns a renderer-independent immutable `PageText` index built lazily from validated page objects, with deterministic UTF-8 text, source identity, and conservative millimetre geometry. `rofd-ffi` keeps Poppler's page-query mental model while translating core values into independently owned opaque result handles and versioned C records. Existing transaction, overlap, panic, lifetime, and symbol-allowlist rules remain mandatory at every entry point.

**Tech Stack:** Rust 2021, `std::sync::{Mutex, OnceLock}`, existing `rofd-core` geometry/content model, stable C11/C++17 ABI, Cargo integration tests, shell-based dynamic-link smoke tests.

---

## File map

- Create `crates/rofd-core/src/semantic.rs`: public Rust semantic value types, index construction, geometry, search, area extraction, and selection.
- Modify `crates/rofd-core/src/document.rs`: lazy per-page semantic cache and `Page::text()` entry point.
- Modify `crates/rofd-core/src/lib.rs`: export the idiomatic Rust semantic API.
- Create `crates/rofd-core/tests/semantic_text.rs`: focused package-level semantic and cache tests.
- Modify `crates/rofd-core/tests/real_fixture.rs`: repository invoice semantic assertions.
- Modify `crates/rofd-ffi/src/abi.rs`: search flags, selection constants, versioned records, and permanent v1 size boundaries.
- Modify `crates/rofd-ffi/src/handles.rs`: opaque token mappings and owned semantic result storage.
- Create `crates/rofd-ffi/src/semantic.rs`: Poppler-style page-level C text functions and matching accessors/free functions.
- Modify `crates/rofd-ffi/src/lib.rs`: export the new semantic symbols and opaque types.
- Modify `crates/rofd-ffi/include/rofd.h`: public C declarations and ownership/safety contracts.
- Create `crates/rofd-ffi/tests/semantic.rs`: ABI behavior, lifetime, transaction, overlap, and concurrency regressions.
- Modify `crates/rofd-ffi/tests/abi.rs`: numeric values and record-layout assertions.
- Modify `crates/rofd-ffi/tests/c/header_compile.c`: C11 declarations and record prefix checks.
- Modify `crates/rofd-ffi/tests/c/header_compile.cpp`: C++17 standard-layout checks.
- Modify `crates/rofd-ffi/tests/c/ffi_smoke.c`: real invoice extraction/search/layout/selection and post-page lifetime checks.
- Modify `crates/rofd-ffi/tests/c/expected-symbols.txt`: exact additive exported-symbol allowlist.
- Modify `crates/rofd-ffi/README.md`: C consumer example and ownership notes.

### Task 1: Define the idiomatic Rust semantic value model

**Files:**
- Create: `crates/rofd-core/src/semantic.rs`
- Modify: `crates/rofd-core/src/lib.rs`
- Test: `crates/rofd-core/tests/semantic_text.rs`

- [ ] **Step 1: Write the public-model test first**

Create `crates/rofd-core/tests/semantic_text.rs` with a synthetic page containing ASCII, CJK, explicit displacement, and two source text objects:

```rust
mod support;

use rofd_core::{Document, LoadOptions, TextCharFlags, TextGeometryPrecision};

fn page_with_text(content: &str) -> rofd_core::Page {
    let page = format!(
        r#"<ofd:Page xmlns:ofd="http://www.ofdspec.org/2016">
  <ofd:Area><ofd:PhysicalBox>0 0 100 100</ofd:PhysicalBox></ofd:Area>
  <ofd:Content><ofd:Layer ID="1">{content}</ofd:Layer></ofd:Content>
</ofd:Page>"#
    );
    Document::from_bytes(support::minimal_ofd(&page), LoadOptions::default())
        .unwrap()
        .page(0)
        .unwrap()
}

#[test]
fn page_text_exposes_utf8_ranges_source_ids_and_conservative_geometry() {
    let page = page_with_text(
        r#"<ofd:TextObject ID="2" Boundary="10 20 30 8" Font="10" Size="4">
             <ofd:TextCode X="1" Y="5" DeltaX="3 4">A中B</ofd:TextCode>
           </ofd:TextObject>
           <ofd:TextObject ID="3" Boundary="10 35 30 8" Font="10" Size="4">
             <ofd:TextCode X="1" Y="5">尾</ofd:TextCode>
           </ofd:TextObject>"#,
    );

    let text = page.text().unwrap();
    assert_eq!(text.as_str(), "A中B\n尾");
    assert_eq!(text.characters().len(), 5);
    assert_eq!(text.characters()[0].utf8_range(), 0..1);
    assert_eq!(text.characters()[1].utf8_range(), 1..4);
    assert_eq!(text.characters()[2].utf8_range(), 4..5);
    assert_eq!(text.characters()[3].utf8_range(), 5..6);
    assert_eq!(text.characters()[4].utf8_range(), 6..9);
    assert_eq!(text.characters()[1].object_id(), Some(2));
    assert_eq!(
        text.characters()[1].geometry_precision(),
        TextGeometryPrecision::Conservative
    );
    assert!(text.characters()[1].rect_mm().is_some());
    assert!(text.characters()[3]
        .flags()
        .contains(TextCharFlags::SYNTHESIZED_SEPARATOR));
    assert_eq!(text.characters()[3].object_id(), None);
    assert_eq!(text.characters()[3].rect_mm(), None);
}
```

- [ ] **Step 2: Run the focused test and confirm the API is absent**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-core --test semantic_text page_text_exposes_utf8_ranges_source_ids_and_conservative_geometry -- --exact
```

Expected: compilation fails because `Page::text`, `TextCharFlags`, and `TextGeometryPrecision` do not exist.

- [ ] **Step 3: Add focused semantic value types**

Create `crates/rofd-core/src/semantic.rs`. Keep fields private and expose immutable queries:

```rust
use std::ops::Range;

use crate::Rect;

/// Precision of a semantic character rectangle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextGeometryPrecision {
    /// Geometry came directly from exact source bounds.
    Exact,
    /// Geometry is a finite conservative estimate from validated OFD metrics.
    Conservative,
}

/// Stable flags describing one semantic character.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextCharFlags(u32);

impl TextCharFlags {
    /// The character was inserted to separate source runs or blocks.
    pub const SYNTHESIZED_SEPARATOR: Self = Self(1 << 0);
    /// The source character is Unicode whitespace.
    pub const WHITESPACE: Self = Self(1 << 1);

    /// Returns true when every bit in `other` is set.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }

    /// Returns the stable numeric representation used by the C adapter.
    pub const fn bits(self) -> u32 {
        self.0
    }
}

/// One logical Unicode scalar and its page semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct TextChar {
    pub(crate) utf8_range: Range<usize>,
    pub(crate) rect_mm: Option<Rect>,
    pub(crate) flags: TextCharFlags,
    pub(crate) precision: TextGeometryPrecision,
    pub(crate) object_id: Option<u64>,
}

impl TextChar {
    /// Returns the byte range in [`PageText::as_str`].
    pub fn utf8_range(&self) -> Range<usize> {
        self.utf8_range.clone()
    }

    /// Returns the page-coordinate rectangle, or none for a synthesized separator.
    pub fn rect_mm(&self) -> Option<Rect> {
        self.rect_mm
    }

    /// Returns stable character flags.
    pub fn flags(&self) -> TextCharFlags {
        self.flags
    }

    /// Returns the precision classification for the rectangle.
    pub fn geometry_precision(&self) -> TextGeometryPrecision {
        self.precision
    }

    /// Returns the source text object identifier.
    pub fn object_id(&self) -> Option<u64> {
        self.object_id
    }
}

/// Immutable semantic text for one page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageText {
    pub(crate) text: String,
    pub(crate) characters: Vec<TextChar>,
}

impl PageText {
    /// Returns flattened UTF-8 page text.
    pub fn as_str(&self) -> &str {
        &self.text
    }

    /// Returns logical characters in the same UTF-8 order.
    pub fn characters(&self) -> &[TextChar] {
        &self.characters
    }
}
```

Add `mod semantic;` and these exports to `crates/rofd-core/src/lib.rs`:

```rust
pub use semantic::{PageText, TextChar, TextCharFlags, TextGeometryPrecision};
```

- [ ] **Step 4: Compile the public types before adding index construction**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo check -p rofd-core
```

Expected: PASS; the test still fails only because `Page::text()` is absent.

- [ ] **Step 5: Commit the value-model slice**

```bash
git add crates/rofd-core/src/semantic.rs crates/rofd-core/src/lib.rs crates/rofd-core/tests/semantic_text.rs
git commit -m "feat(core): define page text semantic model"
```

### Task 2: Build and cache semantic text with deterministic geometry

**Files:**
- Modify: `crates/rofd-core/src/semantic.rs`
- Modify: `crates/rofd-core/src/document.rs`
- Test: `crates/rofd-core/tests/semantic_text.rs`

- [ ] **Step 1: Add traversal, transform, and single-flight cache regressions**

Append tests that exercise page/group/composite/annotation traversal, transformed rectangles, singular-object omission, and shared-page caching:

```rust
#[test]
fn semantic_text_recurses_in_effective_paint_order_and_maps_to_page_mm() {
    let page = page_with_text(
        r#"<ofd:PageBlock ID="2">
             <ofd:TextObject ID="3" Boundary="10 20 20 8" CTM="1 0 0 1 2 3" Font="10" Size="4">
               <ofd:TextCode X="1" Y="5" DeltaX="4">甲乙</ofd:TextCode>
             </ofd:TextObject>
           </ofd:PageBlock>"#,
    );
    let text = page.text().unwrap();
    assert_eq!(text.as_str(), "甲乙");
    let first = text.characters()[0].rect_mm().unwrap();
    assert_eq!(first.x, 13.0);
    assert_eq!(first.y, 24.0);
    assert!(first.width > 0.0 && first.height > 0.0);
}

#[test]
fn repeated_page_handles_share_one_published_semantic_index() {
    let page = page_with_text(
        r#"<ofd:TextObject ID="2" Boundary="0 0 20 8" Font="10" Size="4">
             <ofd:TextCode X="1" Y="5">cache</ofd:TextCode>
           </ofd:TextObject>"#,
    );
    let clone = page.clone();
    assert!(std::ptr::eq(page.text().unwrap(), clone.text().unwrap()));
}
```

- [ ] **Step 2: Run both tests and verify failure**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-core --test semantic_text
```

Expected: compilation fails because index construction and `Page::text()` are absent.

- [ ] **Step 3: Add the fallible lazy cache to page data**

In `crates/rofd-core/src/document.rs`, extend `PageData`, initialize both fields where page data is created, and expose a borrowed cached index:

```rust
struct PageData {
    size: crate::Rect,
    layers: Vec<crate::Layer>,
    semantic_text: OnceLock<crate::PageText>,
    semantic_text_initialization: Mutex<()>,
}
```

```rust
let parsed = Arc::new(PageData {
    size,
    layers,
    semantic_text: OnceLock::new(),
    semantic_text_initialization: Mutex::new(()),
});
```

Add this method to `impl Page`:

```rust
/// Returns the lazily built immutable semantic text index.
///
/// Failed construction is not cached, so a transient internal failure cannot
/// poison later queries.
pub fn text(&self) -> crate::Result<&crate::PageText> {
    if let Some(text) = self.data.semantic_text.get() {
        return Ok(text);
    }
    let _initialization = self
        .data
        .semantic_text_initialization
        .lock()
        .map_err(|_| crate::Error::Internal(
            "semantic text initialization lock is poisoned".to_owned(),
        ))?;
    if let Some(text) = self.data.semantic_text.get() {
        return Ok(text);
    }
    let built = crate::semantic::build_page_text(self)?;
    Ok(self.data.semantic_text.get_or_init(|| built))
}
```

- [ ] **Step 4: Implement deterministic source traversal and conservative boxes**

In `crates/rofd-core/src/semantic.rs`, implement `build_page_text` with these concrete rules:

```rust
pub(crate) fn build_page_text(page: &crate::Page) -> crate::Result<PageText> {
    let mut builder = PageTextBuilder::default();
    for layer in page.layers() {
        builder.visit_objects(layer.objects(), crate::Transform::IDENTITY)?;
    }
    for annotation in page.annotations().into_iter().filter(|item| item.visible()) {
        let transform = object_to_parent(annotation.boundary(), crate::Transform::IDENTITY)?;
        builder.visit_objects(annotation.objects(), transform)?;
    }
    Ok(builder.finish())
}

#[derive(Default)]
struct PageTextBuilder {
    page_text: PageText,
}

impl PageTextBuilder {
    fn visit_objects(
        &mut self,
        objects: &[crate::PageObject],
        parent_to_page: crate::Transform,
    ) -> crate::Result<()> {
        for object in objects {
            match object {
                crate::PageObject::Text(text) if !text.transform().is_singular() => {
                    let local_to_parent = object_to_parent(text.boundary(), text.transform())?;
                    self.push_text_object(text, local_to_parent.then(parent_to_page)?)?;
                }
                crate::PageObject::Group(group) => {
                    self.visit_objects(group.objects(), parent_to_page)?;
                }
                crate::PageObject::Composite(composite)
                    if !composite.transform().is_singular() =>
                {
                    let local_to_parent =
                        object_to_parent(composite.boundary(), composite.transform())?;
                    self.visit_objects(
                        composite.objects(),
                        local_to_parent.then(parent_to_page)?,
                    )?;
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn object_to_parent(boundary: Rect, transform: crate::Transform) -> crate::Result<crate::Transform> {
    let translation = crate::Transform::new(1.0, 0.0, 0.0, 1.0, boundary.x, boundary.y)?;
    transform.then(translation)
}
```

`push_text_object` must walk `TextCode::text().char_indices()` in source order, use explicit `DeltaX`/`DeltaY` when present and `font_size` as the deterministic horizontal fallback advance, map all four corners of `(x, y - font_size, advance_width, font_size)`, and store their axis-aligned union. Insert exactly one `\n` before a nonempty later text object; create a synthesized `TextChar` with `None` geometry and `None` object ID so UTF-8 offsets stay total and contiguous.

Use these helpers so semantic positioning does not depend on font discovery:

```rust
fn push_text_object(
    &mut self,
    object: &crate::TextObject,
    local_to_page: crate::Transform,
) -> crate::Result<()> {
    if object.runs().iter().all(|run| run.text().is_empty()) {
        return Ok(());
    }
    if !self.page_text.text.is_empty() {
        let start = self.page_text.text.len();
        self.page_text.text.push('\n');
        self.page_text.characters.push(TextChar {
            utf8_range: start..start + 1,
            rect_mm: None,
            flags: TextCharFlags::SYNTHESIZED_SEPARATOR,
            precision: TextGeometryPrecision::Conservative,
            object_id: None,
        });
    }
    for run in object.runs() {
        let mut x = run.x();
        let mut y = run.y();
        let chars = run.text().chars().collect::<Vec<_>>();
        for (index, character) in chars.into_iter().enumerate() {
            let start = self.page_text.text.len();
            self.page_text.text.push(character);
            let end = self.page_text.text.len();
            let advance_x = if run.has_explicit_delta_x() {
                run.delta_x()[index]
            } else {
                object.font_size()
            };
            let advance_y = if run.has_explicit_delta_y() {
                run.delta_y()[index]
            } else {
                0.0
            };
            let width = advance_x.abs().max(object.font_size() * 0.5);
            let local = Rect {
                x: x.min(x + advance_x),
                y: y.min(y + advance_y) - object.font_size(),
                width,
                height: object.font_size() + advance_y.abs(),
            };
            self.page_text.characters.push(TextChar {
                utf8_range: start..end,
                rect_mm: Some(map_rect(local_to_page, local)?),
                flags: if character.is_whitespace() {
                    TextCharFlags::WHITESPACE
                } else {
                    TextCharFlags::default()
                },
                precision: TextGeometryPrecision::Conservative,
                object_id: Some(object.object_id()),
            });
            x = finite_add(x, advance_x, object.object_id(), "text x")?;
            y = finite_add(y, advance_y, object.object_id(), "text y")?;
        }
    }
    Ok(())
}

fn map_rect(transform: crate::Transform, rect: Rect) -> crate::Result<Rect> {
    let corners = [
        crate::Point::new(rect.x, rect.y)?,
        crate::Point::new(rect.x + rect.width, rect.y)?,
        crate::Point::new(rect.x, rect.y + rect.height)?,
        crate::Point::new(rect.x + rect.width, rect.y + rect.height)?,
    ]
    .map(|point| transform.apply(point))
    .into_iter()
    .collect::<crate::Result<Vec<_>>>()?;
    let min_x = corners.iter().map(|p| p.x()).fold(f64::INFINITY, f64::min);
    let min_y = corners.iter().map(|p| p.y()).fold(f64::INFINITY, f64::min);
    let max_x = corners.iter().map(|p| p.x()).fold(f64::NEG_INFINITY, f64::max);
    let max_y = corners.iter().map(|p| p.y()).fold(f64::NEG_INFINITY, f64::max);
    Ok(Rect { x: min_x, y: min_y, width: max_x - min_x, height: max_y - min_y })
}

fn finite_add(value: f64, delta: f64, object_id: u64, field: &'static str) -> crate::Result<f64> {
    let result = value + delta;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(crate::Error::InvalidValue {
            field,
            value: format!("object {object_id} coordinate overflow"),
            path: None,
        })
    }
}
```

- [ ] **Step 5: Run semantic and existing content regressions**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-core --test semantic_text
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-core --test text_content
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-core --test templates
```

Expected: all tests PASS with no renderer dependency introduced into `rofd-core`.

- [ ] **Step 6: Commit cached construction**

```bash
git add crates/rofd-core/src/semantic.rs crates/rofd-core/src/document.rs crates/rofd-core/tests/semantic_text.rs
git commit -m "feat(core): build cached page text semantics"
```

### Task 3: Add Rust extraction, search, and selection operations

**Files:**
- Modify: `crates/rofd-core/src/semantic.rs`
- Test: `crates/rofd-core/tests/semantic_text.rs`
- Modify: `crates/rofd-core/tests/real_fixture.rs`

- [ ] **Step 1: Write behavior tests for every Rust query**

Append tests covering area extraction, UTF-8-safe literal search, case and whole-word options, result limits, and the three selection styles:

```rust
use rofd_core::{FindOptions, Rect, SelectionStyle};

#[test]
fn search_is_utf8_safe_case_configurable_and_bounded() {
    let page = page_with_text(
        r#"<ofd:TextObject ID="2" Boundary="0 0 60 8" Font="10" Size="4">
             <ofd:TextCode X="1" Y="5" DeltaX="4 4 4 4 4 4 4 4">Foo 发票 foo</ofd:TextCode>
           </ofd:TextObject>"#,
    );
    let text = page.text().unwrap();
    let default_matches = text.find("foo", FindOptions::default()).unwrap();
    assert_eq!(default_matches.len(), 2);
    assert_eq!(&text.as_str()[default_matches[0].utf8_range()], "Foo");
    assert_eq!(
        text.find(
            "foo",
            FindOptions {
                case_sensitive: true,
                whole_words: true,
                max_results: 1,
            },
        )
        .unwrap()
        .len(),
        1
    );
    assert_eq!(text.find("发票", FindOptions::default()).unwrap().len(), 1);
}

#[test]
fn area_and_styled_selection_share_the_flattened_text_contract() {
    let page = page_with_text(
        r#"<ofd:TextObject ID="2" Boundary="10 20 60 8" Font="10" Size="4">
             <ofd:TextCode X="1" Y="5" DeltaX="4 4 4 4 4 4 4 4 4 4">alpha beta</ofd:TextCode>
           </ofd:TextObject>"#,
    );
    let text = page.text().unwrap();
    let area = Rect { x: 10.0, y: 20.0, width: 24.0, height: 10.0 };
    assert!(text.text_for_area(area).contains("alpha"));
    assert_eq!(text.select(area, SelectionStyle::Word).text(), "alpha");
    assert_eq!(text.select(area, SelectionStyle::Line).text(), "alpha beta");
    assert!(!text.select(area, SelectionStyle::Glyph).regions().is_empty());
}
```

Extend `crates/rofd-core/tests/real_fixture.rs`:

```rust
let text = document.page(0).unwrap().text().unwrap();
assert!(text.as_str().contains("电子发票（普通发票）"));
assert_eq!(text.find("发票号码", Default::default()).unwrap().len(), 1);
```

- [ ] **Step 2: Run the query tests and verify the types are absent**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-core --test semantic_text
```

Expected: compilation fails on `FindOptions`, `SelectionStyle`, and the new `PageText` methods.

- [ ] **Step 3: Implement the public Rust query types**

Add these public types and export them from `crates/rofd-core/src/lib.rs`:

```rust
/// Options for a literal page-text search.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FindOptions {
    /// Match Unicode scalar case exactly.
    pub case_sensitive: bool,
    /// Require Unicode-alphanumeric word boundaries.
    pub whole_words: bool,
    /// Maximum results returned by one query.
    pub max_results: usize,
}

impl Default for FindOptions {
    fn default() -> Self {
        Self { case_sensitive: false, whole_words: false, max_results: 10_000 }
    }
}

/// Expansion policy for rectangular text selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionStyle { Glyph, Word, Line }

/// One literal match in flattened page text.
#[derive(Clone, Debug, PartialEq)]
pub struct TextMatch {
    pub(crate) utf8_range: Range<usize>,
    pub(crate) rect_mm: Option<Rect>,
}

/// Owned text and page regions produced by a selection.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextSelection {
    pub(crate) text: String,
    pub(crate) regions: Vec<Rect>,
}
```

Provide documented getters for `TextMatch::utf8_range`, `TextMatch::rect_mm`, `TextSelection::text`, and `TextSelection::regions`.

- [ ] **Step 4: Implement query algorithms over one canonical UTF-8 index**

Add `PageText::text_for_area`, `PageText::find`, and `PageText::select`. All range boundaries must come from `TextChar::utf8_range`; never increment byte offsets manually. Use rectangle intersection for glyph selection, expand through non-whitespace characters for word selection, and through synthesized newline boundaries for line selection. Search must reject an empty query and `max_results == 0` with `Error::InvalidOption`, use `to_lowercase()` only for matching, and translate matches back to original UTF-8 character ranges before returning rectangles.

For rectangle aggregation, use one helper that returns `None` if no visible character participates and otherwise computes finite min/max coordinates. For selection regions, merge adjacent participating character rectangles on the same logical line and preserve line order.

Implement matching through a folded-string boundary map so lowercase expansion never produces invalid original UTF-8 offsets:

```rust
fn folded_with_boundaries(value: &str) -> (String, Vec<(usize, usize)>) {
    let mut folded = String::new();
    let mut boundaries = Vec::new();
    for (start, character) in value.char_indices() {
        let end = start + character.len_utf8();
        for lowered in character.to_lowercase() {
            let folded_start = folded.len();
            folded.push(lowered);
            boundaries.extend((folded_start..folded.len()).map(|_| (start, end)));
        }
    }
    (folded, boundaries)
}

pub fn find(&self, query: &str, options: FindOptions) -> crate::Result<Vec<TextMatch>> {
    if query.is_empty() || options.max_results == 0 {
        return Err(crate::Error::InvalidOption {
            field: "text search",
            value: "query must be nonempty and max_results must be positive".to_owned(),
        });
    }
    let (haystack, boundaries) = if options.case_sensitive {
        let map = self.text.char_indices().flat_map(|(start, ch)| {
            std::iter::repeat_n((start, start + ch.len_utf8()), ch.len_utf8())
        }).collect();
        (self.text.clone(), map)
    } else {
        folded_with_boundaries(&self.text)
    };
    let needle = if options.case_sensitive { query.to_owned() } else { query.to_lowercase() };
    let mut matches = Vec::new();
    for (folded_start, _) in haystack.match_indices(&needle) {
        let folded_end = folded_start + needle.len();
        let start = boundaries[folded_start].0;
        let end = boundaries[folded_end - 1].1;
        if options.whole_words && !is_whole_word(&self.text, start, end) {
            continue;
        }
        matches.push(TextMatch {
            utf8_range: start..end,
            rect_mm: union_rects(self.characters.iter().filter(|character| {
                character.utf8_range.start < end && start < character.utf8_range.end
            }).filter_map(|character| character.rect_mm)),
        });
        if matches.len() == options.max_results {
            break;
        }
    }
    Ok(matches)
}
```

Replace `std::iter::repeat_n` with `std::iter::repeat(...).take(...)` if the repository's minimum Rust version predates stabilization. `is_whole_word` treats a boundary as valid when the adjacent scalar is absent or not alphanumeric and not `_`.

- [ ] **Step 5: Run core semantic, real-fixture, and documentation gates**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-core --test semantic_text
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-core --test real_fixture
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-core
```

Expected: all tests and the `rofd-core` doctest PASS.

- [ ] **Step 6: Commit the complete Rust semantic slice**

```bash
git add crates/rofd-core/src/semantic.rs crates/rofd-core/src/lib.rs crates/rofd-core/tests/semantic_text.rs crates/rofd-core/tests/real_fixture.rs
git commit -m "feat(core): query page text semantics"
```

### Task 4: Declare the additive C ABI and owned handle storage

**Files:**
- Modify: `crates/rofd-ffi/src/abi.rs`
- Modify: `crates/rofd-ffi/src/handles.rs`
- Modify: `crates/rofd-ffi/src/lib.rs`
- Modify: `crates/rofd-ffi/include/rofd.h`
- Modify: `crates/rofd-ffi/tests/abi.rs`
- Modify: `crates/rofd-ffi/tests/c/header_compile.c`
- Modify: `crates/rofd-ffi/tests/c/header_compile.cpp`

- [ ] **Step 1: Add failing ABI layout and constant assertions**

Extend `crates/rofd-ffi/tests/abi.rs` with assertions for:

```rust
assert_eq!(ROFD_FIND_CASE_SENSITIVE, 1 << 0);
assert_eq!(ROFD_FIND_WHOLE_WORDS, 1 << 1);
assert_eq!(ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR, 1 << 0);
assert_eq!(ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY, 1 << 1);
assert_eq!(ROFD_SELECTION_GLYPH, 0);
assert_eq!(ROFD_SELECTION_WORD, 1);
assert_eq!(ROFD_SELECTION_LINE, 2);
assert_eq!(offset_of!(rofd_find_options_t, struct_size), 0);
assert_eq!(offset_of!(rofd_text_char_t, struct_size), 0);
assert_eq!(offset_of!(rofd_text_match_t, struct_size), 0);
```

Add `_Static_assert`/`static_assert` checks for standard C/C++ layout and first-field `struct_size` in both header compilation tests.

- [ ] **Step 2: Run ABI tests and confirm declarations are absent**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-ffi --test abi
```

Expected: compilation fails on the new constants and records.

- [ ] **Step 3: Add versioned records and constants in Rust and C**

Declare identical layouts in `abi.rs` and `rofd.h`:

```c
#define ROFD_FIND_CASE_SENSITIVE (1u << 0)
#define ROFD_FIND_WHOLE_WORDS (1u << 1)
#define ROFD_TEXT_CHAR_SYNTHESIZED_SEPARATOR (1u << 0)
#define ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY (1u << 1)
#define ROFD_SELECTION_GLYPH 0u
#define ROFD_SELECTION_WORD 1u
#define ROFD_SELECTION_LINE 2u

typedef struct rofd_find_options {
    uint32_t struct_size;
    uint32_t flags;
    size_t max_results;
} rofd_find_options_t;

typedef struct rofd_text_char {
    uint32_t struct_size;
    size_t utf8_offset;
    size_t utf8_length;
    rofd_rect_t rect_mm;
    uint32_t flags;
    uint64_t object_id;
} rofd_text_char_t;

typedef struct rofd_text_match {
    uint32_t struct_size;
    size_t utf8_offset;
    size_t utf8_length;
    rofd_rect_t rect_mm;
} rofd_text_match_t;
```

Add `ROFD_FIND_OPTIONS_V1_SIZE`, `ROFD_TEXT_CHAR_V1_SIZE`, and `ROFD_TEXT_MATCH_V1_SIZE` using the existing `c_record_size` pattern. Add `rofd_find_options_init`; null and undersized buffers stay untouched, supported/oversized buffers receive the permanent v1 boundary, zero flags, and `max_results = 10_000`.

- [ ] **Step 4: Add opaque token mappings with independent owned storage**

In `handles.rs`, add opaque tokens and exact storage:

```rust
opaque_handle!(rofd_string_t, "Opaque owned UTF-8 string handle.");
opaque_handle!(rofd_text_layout_t, "Opaque owned page text layout handle.");
opaque_handle!(rofd_text_search_t, "Opaque owned text-search result handle.");
opaque_handle!(rofd_text_selection_t, "Opaque owned text-selection result handle.");

pub(crate) struct StringHandle { pub(crate) bytes: std::ffi::CString }
pub(crate) struct TextLayoutHandle { pub(crate) characters: Vec<rofd_core::TextChar> }
pub(crate) struct TextSearchHandle { pub(crate) matches: Vec<rofd_core::TextMatch> }
pub(crate) struct TextSelectionHandle {
    pub(crate) text: std::ffi::CString,
    pub(crate) regions: Vec<rofd_core::Rect>,
}
```

Seal each token to exactly one storage type and extend the Send/Sync compile-time test to all four storage types.

- [ ] **Step 5: Finish the header ownership contracts and exports**

In `rofd.h`, declare all opaque types before function declarations. Document that page queries return caller-owned handles; borrowed string data remains valid until its string/selection owner is freed; a result remains valid after freeing the page/document; input records and strings must be readable and disjoint from output/error slots.

Export the new ABI records, constants, opaque tokens, and `rofd_find_options_init` from `src/lib.rs`, without changing `ROFD_ABI_VERSION` or any existing record size.

- [ ] **Step 6: Run ABI and header checks**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-ffi --test abi
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-ffi --test contract_docs
```

Expected: PASS; existing v1 sizes and numeric constants remain unchanged.

- [ ] **Step 7: Commit declarations separately from behavior**

```bash
git add crates/rofd-ffi/src/abi.rs crates/rofd-ffi/src/handles.rs crates/rofd-ffi/src/lib.rs crates/rofd-ffi/include/rofd.h crates/rofd-ffi/tests/abi.rs crates/rofd-ffi/tests/c/header_compile.c crates/rofd-ffi/tests/c/header_compile.cpp
git commit -m "feat(ffi): declare page text semantic ABI"
```

### Task 5: Implement page text and layout C entry points

**Files:**
- Create: `crates/rofd-ffi/src/semantic.rs`
- Modify: `crates/rofd-ffi/src/lib.rs`
- Test: `crates/rofd-ffi/tests/semantic.rs`

- [ ] **Step 1: Write FFI tests for strings, layout, and lifetime**

Create `crates/rofd-ffi/tests/semantic.rs` using the existing document-test fixture helpers. Cover:

```rust
#[test]
fn page_text_and_layout_survive_source_handles_and_share_utf8_offsets() {
    let (document, page) = open_semantic_fixture("A中B");
    let mut string = std::ptr::null_mut();
    let mut layout = std::ptr::null_mut();
    assert_eq!(unsafe { rofd_page_get_text(page, &mut string, std::ptr::null_mut()) }, ROFD_STATUS_OK);
    assert_eq!(unsafe { rofd_page_get_text_layout(page, &mut layout, std::ptr::null_mut()) }, ROFD_STATUS_OK);
    unsafe { rofd_page_free(page); rofd_document_free(document); }

    let bytes = unsafe {
        std::slice::from_raw_parts(
            rofd_string_get_data(string).cast::<u8>(),
            rofd_string_get_length(string),
        )
    };
    assert_eq!(std::str::from_utf8(bytes).unwrap(), "A中B");
    let mut count = usize::MAX;
    assert_eq!(unsafe { rofd_text_layout_get_count(layout, &mut count, std::ptr::null_mut()) }, ROFD_STATUS_OK);
    assert_eq!(count, 3);
    let mut character = rofd_text_char_t { struct_size: std::mem::size_of::<rofd_text_char_t>() as u32, ..zero_text_char() };
    assert_eq!(unsafe { rofd_text_layout_get_char(layout, 1, &mut character, std::ptr::null_mut()) }, ROFD_STATUS_OK);
    assert_eq!((character.utf8_offset, character.utf8_length), (1, 3));
    assert_ne!(character.flags & ROFD_TEXT_CHAR_CONSERVATIVE_GEOMETRY, 0);

    unsafe { rofd_text_layout_free(layout); rofd_string_free(string); }
}
```

Also test `rofd_page_get_text_for_area`, null page, null required output, out-of-range layout index, undersized output record, output/error aliasing, input/output overlap, all four null-safe free functions, and concurrent read calls on one page.

- [ ] **Step 2: Run the semantic FFI test and verify symbols are absent**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-ffi --test semantic
```

Expected: compilation fails on the page text/layout entry points.

- [ ] **Step 3: Implement owned strings and page extraction**

In `src/semantic.rs`, reuse `document::page_ref`, `error::{boundary, boundary_with_inputs, HandleOutput, InputRanges, ScalarOutput, StructOutput}`, and the sealed handle helpers. Implement:

```rust
#[no_mangle]
pub unsafe extern "C" fn rofd_page_get_text(
    page: *const rofd_page_t,
    text: *mut *mut rofd_string_t,
    error: *mut *mut rofd_error_t,
) -> rofd_status_t {
    unsafe {
        boundary(error, HandleOutput::required(text), || {
            let value = page_ref(page)?.inner.text()?.as_str();
            Ok(Box::new(StringHandle { bytes: ffi_string(value)? }))
        })
    }
}
```

`ffi_string` must replace embedded NUL with the same visible escaped representation used by error/report handles. Implement `rofd_page_get_text_for_area` after validating the input rectangle is finite with non-negative width/height and preflighting its complete readable range against `text` and `error` through `boundary_with_inputs`.

Implement `rofd_string_get_data`, `rofd_string_get_length`, and `rofd_string_free` with `catch_unwind`; null getters return null/zero and null free is a no-op.

- [ ] **Step 4: Implement layout snapshot and indexed access**

`rofd_page_get_text_layout` clones the immutable `TextChar` slice into `TextLayoutHandle`. `rofd_text_layout_get_count` uses `ScalarOutput`; `rofd_text_layout_get_char` uses `StructOutput`, preserves the caller's supported `struct_size`, maps absent geometry to a zero rectangle, sets synthesized/conservative flags, and writes `object_id = 0` when absent. Reject out-of-range indices without publishing a partially filled record.

Implement `rofd_text_layout_free` with the established panic-contained null-safe pattern.

- [ ] **Step 5: Export and run focused FFI tests**

Add `mod semantic;` and explicit `pub use semantic::{...};` entries in `src/lib.rs`. Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-ffi --test semantic page_text_and_layout_survive_source_handles_and_share_utf8_offsets -- --exact
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-ffi --test semantic
```

Expected: all semantic FFI tests PASS.

- [ ] **Step 6: Commit extraction and layout**

```bash
git add crates/rofd-ffi/src/semantic.rs crates/rofd-ffi/src/lib.rs crates/rofd-ffi/tests/semantic.rs
git commit -m "feat(ffi): expose page text and layout"
```

### Task 6: Implement Poppler-style C search and selection

**Files:**
- Modify: `crates/rofd-ffi/src/semantic.rs`
- Modify: `crates/rofd-ffi/src/lib.rs`
- Modify: `crates/rofd-ffi/include/rofd.h`
- Test: `crates/rofd-ffi/tests/semantic.rs`

- [ ] **Step 1: Add search and selection behavior/hostility tests**

Add tests that call both `rofd_page_find_text` and `rofd_page_find_text_with_options`; assert default case-insensitive behavior, exact case flag, whole-word flag, UTF-8 offsets, finite match rectangles, `max_results`, and independent lifetime after page free. Add selection tests for glyph/word/line expansion, owned UTF-8 text, indexed regions, and lifetime after page free.

Add invalid-input cases for null/non-UTF-8/empty queries, unknown option flags, zero `max_results`, undersized/oversized options, unknown selection style, non-finite/negative rectangles, null outputs, record/result index out of range, detectable query/options/rectangle overlap with outputs, and output/error aliasing. In every failure test, prefill outputs and assert the boundary resets or preserves them according to the existing transaction contract.

- [ ] **Step 2: Run the new tests and verify the functions are absent**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-ffi --test semantic
```

Expected: compilation fails on search/selection entry points.

- [ ] **Step 3: Translate search options once at the boundary**

Add `FindOptionsInput::from_ffi` that accepts null as `FindOptions::default()`, requires at least the permanent v1 boundary, ignores a larger tail, rejects bits outside `ROFD_FIND_CASE_SENSITIVE | ROFD_FIND_WHOLE_WORDS`, rejects zero `max_results`, and clamps the request to `page.resource_limits().max_text_characters_per_page` before calling core.

Parse query bytes with `CStr::from_ptr(...).to_str()`, reject null, invalid UTF-8, and empty values, and include the readable query byte range plus options prefix in `InputRanges` before initializing outputs.

- [ ] **Step 4: Implement search functions and result access**

`rofd_page_find_text` delegates to the same internal implementation with default options. `rofd_page_find_text_with_options` stores `Vec<TextMatch>` in `TextSearchHandle`. Implement `get_count`, versioned `get_match`, and `free`; convert `None` geometry to the zero rectangle and preserve the caller's `struct_size` boundary.

- [ ] **Step 5: Implement styled selection and region access**

Map the three stable `ROFD_SELECTION_*` values to core `SelectionStyle`. Store the selected text as owned `CString` and regions as owned core rectangles. Implement:

```c
const char *rofd_text_selection_get_text(const rofd_text_selection_t *selection);
size_t rofd_text_selection_get_text_length(const rofd_text_selection_t *selection);
rofd_status_t rofd_text_selection_get_region_count(
    const rofd_text_selection_t *selection, size_t *count, rofd_error_t **error);
rofd_status_t rofd_text_selection_get_region(
    const rofd_text_selection_t *selection, size_t index,
    rofd_rect_t *region_mm, rofd_error_t **error);
void rofd_text_selection_free(rofd_text_selection_t *selection);
```

Borrowed getters return null/zero for null handles. Fallible indexed access remains transactional and rejects out-of-range indices.

- [ ] **Step 6: Run the complete Rust FFI suite**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-ffi
```

Expected: all unit, ABI, contract, document, renderer, and semantic tests PASS.

- [ ] **Step 7: Commit search and selection**

```bash
git add crates/rofd-ffi/src/semantic.rs crates/rofd-ffi/src/lib.rs crates/rofd-ffi/include/rofd.h crates/rofd-ffi/tests/semantic.rs
git commit -m "feat(ffi): add page search and text selection"
```

### Task 7: Prove the public ABI from real C and C++ consumers

**Files:**
- Modify: `crates/rofd-ffi/tests/c/header_compile.c`
- Modify: `crates/rofd-ffi/tests/c/header_compile.cpp`
- Modify: `crates/rofd-ffi/tests/c/ffi_smoke.c`
- Modify: `crates/rofd-ffi/tests/c/expected-symbols.txt`
- Modify: `crates/rofd-ffi/README.md`

- [ ] **Step 1: Extend the real C smoke test before updating symbols**

In `ffi_smoke.c`, after obtaining page zero:

```c
rofd_string_t *text = NULL;
rofd_text_layout_t *layout = NULL;
rofd_text_search_t *search = NULL;
rofd_text_selection_t *selection = NULL;
rofd_find_options_t find_options;
size_t count = 0;

CHECK(rofd_page_get_text(page, &text, &error) == ROFD_STATUS_OK);
CHECK(rofd_string_get_data(text) != NULL);
CHECK(strstr(rofd_string_get_data(text), "电子发票（普通发票）") != NULL);
CHECK(rofd_page_get_text_layout(page, &layout, &error) == ROFD_STATUS_OK);
CHECK(rofd_text_layout_get_count(layout, &count, &error) == ROFD_STATUS_OK);
CHECK(count > 0);

rofd_find_options_init(&find_options, sizeof(find_options));
find_options.flags = ROFD_FIND_WHOLE_WORDS;
CHECK(rofd_page_find_text_with_options(page, "发票号码", &find_options,
                                       &search, &error) == ROFD_STATUS_OK);
CHECK(rofd_text_search_get_count(search, &count, &error) == ROFD_STATUS_OK);
CHECK(count == 1);
```

Fetch the first match, use its rectangle for `rofd_page_get_selected_text(..., ROFD_SELECTION_WORD, ...)`, free page/document, and then verify the search match and selected text remain readable before freeing all four semantic handles.

- [ ] **Step 2: Run the C harness and observe the symbol allowlist failure**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target crates/rofd-ffi/tests/run_c_tests.sh
```

Expected: header and runtime smoke behavior PASS, then the final symbol diff fails only because the new `rofd_*` exports are not yet listed.

- [ ] **Step 3: Update the exact sorted symbol allowlist**

Insert every new public symbol in bytewise sorted order in `expected-symbols.txt`, including option initialization, page query functions, borrowed getters, indexed accessors, and matching free functions. Do not remove or rename any existing symbol.

- [ ] **Step 4: Document the Poppler-style C workflow**

Update `crates/rofd-ffi/README.md` with one compilable example that opens a document, gets a page, calls `rofd_page_get_text`, `rofd_page_find_text_with_options`, and `rofd_page_get_text_layout`, checks every fallible return, and frees results with their matching functions. State explicitly that Rust consumers should use `rofd-core::Page::text()` and its idiomatic value types rather than imitate C handles.

- [ ] **Step 5: Run C11/C++17 dynamic-link and symbol checks**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target crates/rofd-ffi/tests/run_c_tests.sh
```

Expected: header compilation, real invoice smoke test, dynamic loading of the library built by that run, and exact symbol diff all PASS.

- [ ] **Step 6: Commit consumer proof and documentation**

```bash
git add crates/rofd-ffi/tests/c/header_compile.c crates/rofd-ffi/tests/c/header_compile.cpp crates/rofd-ffi/tests/c/ffi_smoke.c crates/rofd-ffi/tests/c/expected-symbols.txt crates/rofd-ffi/README.md
git commit -m "test(ffi): prove semantic API from C consumers"
```

### Task 8: Complete quality and compatibility verification

**Files:**
- Modify only files required by failures found in this task.

- [ ] **Step 1: Format and prove formatting is clean**

Run:

```bash
cargo fmt --all
cargo fmt --all -- --check
```

Expected: the check exits successfully with no diff.

- [ ] **Step 2: Run strict lints for all affected targets**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo clippy -p rofd-core -p rofd-ffi --all-targets -- -D warnings
```

Expected: PASS with no warnings, including missing public documentation and unsafe-operation checks.

- [ ] **Step 3: Run all affected Rust tests**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target cargo test -p rofd-core -p rofd-ffi
```

Expected: every unit, integration, and doctest PASS.

- [ ] **Step 4: Run the independent C ABI suite again**

Run:

```bash
CARGO_TARGET_DIR=/tmp/rofd-semantic-api-target crates/rofd-ffi/tests/run_c_tests.sh
```

Expected: C11/C++17 headers, real-fixture runtime checks, dynamic provenance, and symbol allowlist PASS.

- [ ] **Step 5: Inspect compatibility and scope**

Run:

```bash
git diff 04ea071 --check
git diff 04ea071 --stat
git status --short
```

Expected: no whitespace errors; changes stay inside core semantics, C ABI, tests, and documentation; no renderer WIP from the main checkout appears.

- [ ] **Step 6: Commit any verification-only corrections**

If formatting or linting changed tracked files, commit only those focused corrections:

```bash
git add crates/rofd-core crates/rofd-ffi
git commit -m "chore: finish semantic API verification"
```

If no file changed, skip this commit and record the successful command outputs in the final handoff.
