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
