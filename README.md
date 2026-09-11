<p align="center"><img width="128" height="128" src="./resources/logo.png" alt="logo"></p>

<p align="center">rofd </br> OFD parser and renderer library, in Rust.</p>
<p align="center"></p>


# Introduction

OFD (Open Form Document) is an open standard for electronic documents, which is widely used in China. Unlike PDF, which is a layout-based format, OFD is a semantic-based format, which means it stores the document structure and text information separately. This makes OFD documents more flexible and easier to edit than PDFs. OFD also supports more features than PDF, such as form filling and digital signatures.


# Project status

- `rofd-core`: safe OFD container, metadata/DocID/keywords, document outlines,
  page link mappings, page content, clipping, and recursively resolved template layers.
- `rofd-render`: backend-neutral display lists plus bounded Cairo rendering for
  paths, positioned text, PNG/JPEG images, clipping, transforms, rotation, and
  pixel viewports without a full-page raster allocation.
- `rofd-ffi`: stable, versioned C ABI for `rofd-core` and Cairo rendering, with
  independently owned metadata, warning, outline, and page-link snapshots.
- Root `rofd` package: legacy Cairo rendering prototype kept during migration.
- Qt/QML reader: design approved; implementation follows the C ABI phase.

The target architecture and phased roadmap are documented in
[`docs/superpowers/specs/2026-09-03-rofd-library-reader-design.md`](docs/superpowers/specs/2026-09-03-rofd-library-reader-design.md).
The active text-and-image plan is in
[`docs/superpowers/plans/2026-09-05-rofd-text-image-rendering.md`](docs/superpowers/plans/2026-09-05-rofd-text-image-rendering.md).
The deepin-reader integration API contracts are described in
[`crates/rofd-ffi/README.md`](crates/rofd-ffi/README.md).

# Test the reusable libraries

```bash
cargo fmt --all -- --check
cargo clippy -p rofd-core -p rofd-render -p rofd-ffi --all-targets -- -D warnings
cargo test -p rofd-core -p rofd-render -p rofd-ffi
cargo test -p rofd-ffi
crates/rofd-ffi/tests/run_c_tests.sh
```

`rofd-render` requires Cairo and FreeType/fontconfig for configured system-font
fallback. `rofd-core` remains Cairo-, image-codec-, Qt-, and QML-free. Detailed
content support and compatibility limits are documented in the
[`rofd-core`](crates/rofd-core/README.md) and
[`rofd-render`](crates/rofd-render/README.md) READMEs.

# Run the Qt prototype

```bash
cargo run -p rofd --features qt-reader --bin rofd [file.ofd]
```

The Qt command requires the system Qt6 development dependencies used by
`qmetaobject`. The prototype opens OFD documents via the "Open…" button (or a
path passed on the command line), renders pages through `rofd-core` and
`rofd-render`, and supports page navigation. Note the `-p rofd` flag: the root
package is not part of the workspace's `default-members`.

# Project structure

This project is organized into the following directories and files:

- `crates/rofd-core/`: independent document parsing and query crate.
- `crates/rofd-render/`: display-list lowering and bounded Cairo renderer.
- `crates/rofd-ffi/`: stable C header, Rust implementation, and independent
  C/C++ consumer tests.
- `crates/rofd-render/tests/real_fixture.rs`: end-to-end invoice fixture
  regression without generated artifacts.
- `src/`: legacy parser and Cairo renderer retained during migration.
- `src/bin/rofd`: legacy Qt/QML prototype.
- `src/lib.rs`: legacy library crate.
    - `src/document.rs`: document parsing and rendering.
    - `src/page.rs`: page parsing and rendering.
    - `src/render.rs`: rendering to Cairo surface.
    - `src/types.rs`: types used in OFD spec.
    - `src/elements.rs`: OFD elements.
    - `src/ofd.rs`: OFD file parsing.
- `learning/`: learning notes and examples.
- `docs/superpowers/`: architecture specifications and implementation plans.
- `resources/`: resources, such as the logo.
- `LICENSE`: license file.
- `Cargo.toml`: cargo configuration file.
- `README.md`: this readme file.


# Reference projects

- [ofdrw](https://github.com/ofdrw/ofdrw)
- [ofd-parser](https://github.com/jyh2012/ofd-parser)
- [poppler](https://gitlab.freedesktop.org/poppler/poppler)

# License

This project is under the terms of the [GNU Lesser General Public License v2.1 or later](https://github.com/hualet/rofd/blob/main/LICENSE).
