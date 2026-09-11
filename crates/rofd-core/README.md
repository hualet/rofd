# rofd-core

`rofd-core` provides a safe, read-only OFD document model without GUI or
rendering dependencies.

```no_run
use rofd_core::{Document, LoadOptions};

let document = Document::open("document.ofd", LoadOptions::default())?;
println!("{} pages", document.page_count());
let first_page = document.page(0)?;
println!(
    "{} mm × {} mm",
    first_page.size().width,
    first_page.size().height
);
for layer in first_page.layers() {
    println!("layer {} from {:?}", layer.object_id(), layer.source());
}
# Ok::<(), rofd_core::Error>(())
```

The v0.2 foundation supports one `DocBody`. Multiple document bodies return an
explicit `UnsupportedFeature` error. Page access resolves referenced template
pages lazily, merges background/page/foreground content in effective paint
order, and exposes paths, positioned-text inputs, image references, clips, and
their `LayerSource`. Font, image, and draw-parameter resources are indexed
lazily; archive, XML, encoded-byte, decoded-pixel, glyph, and expansion limits
remain enforced on cached and uncached paths. `rofd-core` never loads system
fonts or image codecs. Rendering lives in `rofd-render`; text extraction,
annotations, signatures, and the C ABI remain later phases.

`Document::outline()` lazily returns a shared, immutable preorder outline tree.
Parent, first-child, and next-sibling indices describe the actual XML hierarchy;
advisory `Count` attributes are ignored. Ordered actions expose destinations,
named bookmarks, URIs, attachments, and unsupported type/event names as inert
data. Nothing opens a URI, launches an attachment, or executes an action.
Page identifiers resolve to zero-based indices; missing coordinates stay absent
and zoom is preserved without conversion. Strict mode rejects malformed or
unresolved navigation on query, while lenient mode retains usable nodes and
reports unavailable destinations with warnings, never a fabricated page zero.
Document opening and page parsing remain independent of navigation semantics.

Navigation accepts the official `http://www.ofdspec.org/2016` namespace and
unqualified XML. The historical `http://www.ofdspec.org` namespace used by
existing ofdrw compatibility fixtures is explicitly accepted with a compatibility
warning. Other namespaces cannot impersonate native action types. Nonstandard
numeric `Dest` child elements are a warned compatibility fallback; attributes
always take priority. Successful queries cache both the tree and bookmark
resolver across document clones, and commit their warnings only once.

Only direct `Document/Outlines` and `Document/Bookmarks` subtrees are retained in
the bounded navigation XML arena. Their aggregate element count (including
unknown extensions and action children) uses `max_page_objects`; unrelated
document elements do not consume that budget. Outline depth uses
`max_page_block_depth`. The existing global XML well-formedness and
`max_xml_depth` checks still apply when opening a document.
Independent `max_entry_size` byte budgets bound retained XML arena strings
(including expanded namespace names and entity values) and cumulative expanded
model strings and diagnostics (including warning paths and escaped messages).
Named bookmark destination strings are charged before copying, so repeated
references cannot amplify a small XML entry into unbounded memory. Budget
errors are fatal in both strict and lenient modes and publish neither partial
cache nor warnings.

`Page::links()` returns a shared immutable list of inert `PageLink` values, one
per source action. Page-root actions come first, followed by effective content
in background/page/foreground order. Template-root actions precede their
content, including empty templates; graphic owners precede their children.
Visible annotations and their appearances follow the page layers, regardless
of annotation type. Events and unknown action types remain inspectable; links
do not decode images, load fonts, open URIs, or execute actions.

An explicit `Region` yields a separate conservative page-millimetre rectangle
for each `Area`, transformed through its owner's CTM, boundary translation,
and parent content transforms. Bezier bounds include control points; arc bounds
include the full corrected ellipse, not only the swept arc. OFD radius
normalization, `Move`, and `Close` semantics are preserved. These rectangles
are not exact clipping/occlusion hit masks. Without an explicit region, only
the parent transform applies to the owner boundary (not the owner's own CTM);
an absent boundary uses the physical page. Invalid explicit regions never
become page-sized fallbacks: strict queries fail; lenient queries skip the
affected link or invalid transformed subtree and emit `NavigationInvalid`.

Action XML is captured in bounded shared arenas before content conversion.
Template and composite expansion copy only `Arc` handles to this data. Link
queries independently charge effective layers, graphic objects, visible
annotations, and all selected action XML nodes to one `max_page_objects`
budget; region segments share one `max_path_commands` budget. Repeated source
references consume both budgets on every use. Expanded action/destination
strings and diagnostics share one `max_entry_size` byte budget per query.
Successful results and navigation warnings are committed once across page
clones; failed queries publish neither a link cache nor navigation warnings.
The existing annotation loader can separately publish `AnnotationSkipped`.
Its tolerant public annotation result never hides resource limits from links:
the first original limit failure is retained alongside the annotation cache and
returned as `LimitExceeded` by links even if annotations were queried first.
Bookmark resolution is cached independently from
outlines, so unrelated malformed outlines do not block links, and links without
named bookmark references do not parse the bookmark table.
