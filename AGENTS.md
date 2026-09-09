# Repository Guidelines

## Project Structure & Module Organization

This Rust 2021 workspace is migrating from a legacy renderer to a reusable OFD library. Active development belongs in `crates/rofd-core/`; its public API is exported from `src/lib.rs`, implementation modules live beside it, and integration tests are in `crates/rofd-core/tests/`. The root `src/` tree contains the legacy parser and Cairo renderer. The optional Qt/QML prototype lives under `src/bin/rofd/`. Repository fixtures are under `tests/fixtures/`, visual assets under `resources/`, and design or implementation notes under `docs/superpowers/`. The ofdrw-migrated compatibility tests live in `tests/ofdrw-compat/`: real-world `.ofd` fixtures copied from ofdrw, ported parsing assertions, and rendering comparisons against ofdrw-rendered reference PNGs (see its `README.md` and `PROVENANCE.md`). Treat `learning/` as reference material, not production code.

## Build, Test, and Development Commands

- `cargo test -p rofd-core` runs the primary library test suite and matches CI.
- `cargo test -p ofdrw-compat` runs the ofdrw-migrated compatibility suite (not in `default-members`); rendering comparisons need system Noto CJK fonts, and the reference PNGs are regenerated only via `tests/ofdrw-compat/tools/render-references.sh` (requires JDK and Maven).
- `cargo fmt --all -- --check` verifies formatting without modifying files; run `cargo fmt --all` to apply it.
- `cargo clippy -p rofd-core --all-targets -- -D warnings` enforces the CI lint policy.
- `cargo test -p rofd` exercises the legacy root package when changing legacy code.
- `cargo run -p rofd --features qt-reader --bin rofd [file.ofd]` launches the prototype; it requires Qt6 development packages and Cairo.
- `rofd-render`'s default `jbig2` feature decodes JBIG2 images through the system jbig2dec library (Debian: `libjbig2dec0-dev`), located via pkg-config like Cairo and FreeType; build with `--no-default-features` to compile without it. The `-p rofd` flag is needed because the root package is not in the workspace's `default-members`.
- `dpkg-buildpackage -us -uc -b` builds the Debian packages (`rofd` Qt/QML app, `librofd-ffi0`, `librofd-ffi-dev`) from `debian/`; it needs a Rust toolchain newer than the distro's (image 0.25 requires Rust 1.88), e.g. via rustup.

CI runs in the `docker.io/hualet/deepin:25.1-builder` container: `.github/workflows/build.yml` verifies formatting, Clippy, tests, and the release build on every push and pull request, and `.github/workflows/deb.yml` builds the Debian packages and attaches them to the GitHub release when a `v*` tag is pushed.

## Version Bumps & Releases

Keep these places in sync when bumping the version; the tag `v<X.Y.Z>` must match the version in `debian/changelog`:

- `Cargo.toml` (root package): the reader application version.
- `crates/rofd-core/Cargo.toml`, `crates/rofd-render/Cargo.toml`, `crates/rofd-ffi/Cargo.toml`: each crate versions independently (`rofd-core` is ahead of the others). Run any cargo command afterwards so `Cargo.lock` picks up the new versions.
- `debian/changelog`: add a new `rofd (X.Y.Z-1) unstable; urgency=medium` entry on top (e.g. with `dch -v X.Y.Z-1`).
- `debian/rules`: the installed library filename `librofd_ffi.so.0.1.0` mirrors the `rofd-ffi` crate version.
- `crates/rofd-ffi/build.rs`: the SONAME `librofd_ffi.so.0` is the ABI major version; bump the trailing number only when the C ABI breaks, not on every release.

## Coding Style & Naming Conventions

Use standard `rustfmt` output (four-space indentation). Name modules, functions, and test cases in `snake_case`; types and traits use `UpperCamelCase`; constants use `SCREAMING_SNAKE_CASE`. Keep public `rofd-core` APIs documented: the crate denies missing documentation and forbids unsafe code. Prefer small modules, explicit errors, and resource-bounded parsing for untrusted OFD archives.

## Testing Guidelines

Add unit tests beside private implementation details and integration tests in `crates/rofd-core/tests/` for public behavior. Use behavior-focused names such as `multiple_doc_bodies_are_explicitly_unsupported_in_v02`. Reuse helpers from `tests/support/` and repository fixtures when realistic packages matter. There is no stated coverage threshold; every bug fix should include a regression test.

## Commit & Pull Request Guidelines

Follow the repository's Conventional Commit pattern: `feat(core): ...`, `fix(render): ...`, `test(core): ...`, `docs: ...`, or `build: ...`. Keep commits focused and use an imperative, concise subject. Pull requests should explain motivation and behavior changes, link relevant issues or design notes, and list verification commands. Include screenshots for Qt/QML or rendered-output changes, and ensure formatting, Clippy, and affected tests pass before review.
