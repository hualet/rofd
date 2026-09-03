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
# Ok::<(), rofd_core::Error>(())
```

The v0.2 foundation supports one `DocBody`. Multiple document bodies return an
explicit `UnsupportedFeature` error. Rendering, text extraction, annotations,
signatures, and the C ABI are delivered by the following implementation phases.
