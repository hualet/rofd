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
order, and exposes each layer's `LayerSource`. Concurrent page loads publish
only complete immutable template cache entries; they may safely duplicate work
while a shared template is still being parsed. Rendering, text extraction,
annotations, signatures, and the C ABI are delivered by the following
implementation phases.
