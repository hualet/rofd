# Repository Guidelines

## Scope and Layout

This Rust 2021 workspace is migrating from a legacy renderer to reusable OFD crates. Put production work in `crates/rofd-core/`, `crates/rofd-render/`, and `crates/rofd-ffi/`; keep `rofd-core` independent of GUI and rendering dependencies. The root `src/` tree is the legacy parser/Cairo renderer, and `src/bin/rofd/` is the optional Qt/QML reader. Fixtures live in `tests/fixtures/`, assets in `resources/`, plans in `docs/superpowers/`, and provenance-tracked ofdrw compatibility tests in `tests/ofdrw-compat/`. Treat `learning/` as reference material only.

## Build and Test

- `cargo fmt --all -- --check`
- `cargo clippy -p rofd-core -p rofd-render -p rofd-ffi --all-targets -- -D warnings`
- `cargo test -p rofd-core` for the primary suite; `cargo test --workspace --all-targets` for the full workspace.
- `cargo test -p ofdrw-compat` for real-world parsing/rendering checks; it is outside `default-members` and needs Noto CJK fonts.
- `crates/rofd-ffi/tests/run_c_tests.sh` for C11/C++17 ABI and dynamic-link checks.
- `cargo test -p rofd` when changing legacy code.
- `cargo build --release -p rofd-ffi -p rofd --features rofd/qt-reader` for the shipped FFI library and Qt reader.
- `cargo run -p rofd --features qt-reader --bin rofd [file.ofd]` to launch the reader; Qt6 and Cairo development packages are required.
- `dpkg-buildpackage -us -uc -b` builds Debian packages. A rustup toolchain may be needed because `image 0.25` requires Rust 1.88.

`rofd-render` enables system `jbig2dec` support by default; use `--no-default-features` only when deliberately building without it. Regenerate ofdrw reference PNGs only with `tests/ofdrw-compat/tools/render-references.sh`, then inspect them before committing.

## Versioning and Releases

For a release, update the root and crate versions in `Cargo.toml`, `crates/rofd-core/Cargo.toml`, `crates/rofd-render/Cargo.toml`, and `crates/rofd-ffi/Cargo.toml`; update `crates/rofd-core/tests/public_api.rs`, run Cargo to refresh `Cargo.lock`, prepend `debian/changelog`, and make the `debian/rules` library filename match the `rofd-ffi` crate version. The `vX.Y.Z` tag must match the Debian version.

Keep the C ABI SONAME in `crates/rofd-ffi/build.rs` at `librofd_ffi.so.0` unless the ABI major changes. Never force-push, delete, or move a published tag; publish a new tag if correction is required.

## Code and API Rules

Use standard `rustfmt` naming and formatting. Keep public `rofd-core` APIs documented; the crate denies missing docs and forbids unsafe code. Prefer small modules, explicit errors, and resource-bounded parsing of untrusted archives.

Use Poppler GLib only as a usability reference for `rofd.h`: document and page handles are independently owned, and page queries have matching free functions. Do not expose GLib types or PDF-only concepts. Preserve all published v1 symbols and structures, opaque handles, `struct_size` versioning, transactional outputs, overlap checks, panic containment, bounded inputs, and exact status/error reporting. Extend the ABI additively; Rust APIs remain idiomatic and need not mirror C layouts.

## Tests and Reviews

Add unit tests beside private code and integration tests for public behavior. Every bug fix needs a regression test; use repository fixtures when package realism matters. PRs should explain motivation and behavior, list verification commands, and include screenshots for Qt/QML or rendered-output changes.
