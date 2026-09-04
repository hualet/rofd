# rofd-render

`rofd-render` lowers validated, GUI-independent `rofd-core` page objects into an
immutable backend-neutral display list. It does not read OFD archives or XML;
rendering backends consume its commands in order. Effective page layers already
include recursively resolved background and foreground templates, so display
commands and diagnostics preserve the merged paint order without introducing a
dependency from `rofd-core` back to the renderer.

## Display lists and Cairo rendering

```no_run
use cairo::{Context, Format, ImageSurface};
use rofd_core::{Document, LoadOptions};
use rofd_render::{CairoRenderer, DisplayList, RenderOptions};

let document = Document::open("document.ofd", LoadOptions::default())?;
let page = document.page(0)?;

let display_list = DisplayList::from_page(&page)?;
for diagnostic in display_list.diagnostics() {
    eprintln!(
        "deferred object {} ({:?}) from {:?}",
        diagnostic.object_id(),
        diagnostic.kind(),
        diagnostic.source()
    );
}

let options = RenderOptions::default();
let (width, height) = CairoRenderer::pixel_size(&page, &options)?;
let surface = ImageSurface::create(Format::ARgb32, width, height)?;
let context = Context::new(&surface)?;
let report = CairoRenderer.render_page(&page, &context, &options)?;
assert_eq!(report.diagnostics(), display_list.diagnostics());
# Ok::<(), Box<dyn std::error::Error>>(())
```

The phase-2 renderer supports path move, line, quadratic and cubic curves,
elliptical arcs, and close operations; solid RGBA fills and strokes; non-zero
and even-odd fill rules; affine object and clip transforms; true-union OFD clip
areas with intersection between separate clips; recursively merged background
and foreground templates; optional page-space clipping; and clockwise page
rotation by 0, 90, 180, or 270 degrees. Page coordinates are millimetres, page
boxes may have nonzero origins, and output dimensions follow the configured DPI
and scale.

`RenderOptions::default()` selects 96 DPI, scale 1, no rotation, an opaque white
background, no optional page-space clip, and a 256 MiB raster budget. The
`max_raster_bytes` budget conservatively covers the required ARGB32 target plus
the full-page clip mask and clipped-drawing intermediate. Invalid options,
unsupported Cairo dimensions, insufficient target surfaces, and budget excesses
are returned as structured errors. Rendering preserves the caller's Cairo
graphics state and current path on both success and recoverable failure.

Text and image objects are deliberately deferred to phase 3. They are omitted
from drawing and reported through display-list and render-report diagnostics,
including their object kind, object identifier, and page/template source.
Fonts, document resources, composite objects, annotations, and signatures are
also not rendered in this phase.
