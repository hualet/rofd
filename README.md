<p align="center"><img width="128" height="128" src="./resources/logo.png" alt="logo"></p>

<p align="center">rofd </br> OFD parser and renderer library, in Rust.</p>
<p align="center"></p>


# Introduction

OFD (Open Form Document) is an open standard for electronic documents, which is widely used in China. Unlike PDF, which is a layout-based format, OFD is a semantic-based format, which means it stores the document structure and text information separately. This makes OFD documents more flexible and easier to edit than PDFs. OFD also supports more features than PDF, such as form filling and digital signatures.


# Project status

- `rofd-core`: safe OFD container, metadata, and page access foundation.
- Root `rofd` package: legacy Cairo rendering prototype kept during migration.
- Qt/QML reader: design approved; implementation follows the C ABI phase.

The target architecture and phased roadmap are documented in
[`docs/superpowers/specs/2026-09-03-rofd-library-reader-design.md`](docs/superpowers/specs/2026-09-03-rofd-library-reader-design.md).
The active core-foundation plan is in
[`docs/superpowers/plans/2026-09-03-rofd-core-foundation.md`](docs/superpowers/plans/2026-09-03-rofd-core-foundation.md).

# Build the core library

```bash
cargo test -p rofd-core
```

# Run the legacy Qt prototype

```bash
cargo run --features qt-reader --bin rofd
```

The Qt command requires the system Qt development dependencies used by
`qmetaobject`. The prototype is not yet the planned OFD reader.

# Project structure

This project is organized into the following directories and files:

- `crates/rofd-core/`: independent document parsing and query crate.
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
- `resources/`: resources, such as the logo.
- `LICENSE`: license file.
- `Cargo.toml`: cargo configuration file.
- `README.md`: this readme file.


# Reference projects

- [ofdrw](https://github.com/ofdrw/ofdrw)
- [ofd-parser](https://github.com/jyh2012/ofd-parser)
- [poppler](https://gitlab.freedesktop.org/poppler/poppler)

# License

This project is under the terms of the [MIT License](https://github.com/rofd/rofd/blob/main/LICENSE).
