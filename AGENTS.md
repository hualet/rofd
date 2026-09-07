# Repository Guidelines

## Project Structure & Module Organization

This Rust 2021 workspace is migrating from a legacy renderer to a reusable OFD library. Active development belongs in `crates/rofd-core/`; its public API is exported from `src/lib.rs`, implementation modules live beside it, and integration tests are in `crates/rofd-core/tests/`. The root `src/` tree contains the legacy parser and Cairo renderer. The optional Qt/QML prototype lives under `src/bin/rofd/`. Repository fixtures are under `tests/fixtures/`, visual assets under `resources/`, and design or implementation notes under `docs/superpowers/`. Treat `learning/` as reference material, not production code.

## Build, Test, and Development Commands

- `cargo test -p rofd-core` runs the primary library test suite and matches CI.
- `cargo fmt --all -- --check` verifies formatting without modifying files; run `cargo fmt --all` to apply it.
- `cargo clippy -p rofd-core --all-targets -- -D warnings` enforces the CI lint policy.
- `cargo test -p rofd` exercises the legacy root package when changing legacy code.
- `cargo run --features qt-reader --bin rofd` launches the prototype; it requires Qt development packages and Cairo.
- `dpkg-buildpackage -us -uc -b` builds the Debian packages (`rofd` Qt/QML app, `librofd-ffi0`, `librofd-ffi-dev`) from `debian/`; it needs a Rust toolchain newer than the distro's (image 0.25 requires Rust 1.88), e.g. via rustup.

CI runs in the `docker.io/hualet/deepin:25.1-builder` container: `.github/workflows/build.yml` verifies formatting, Clippy, tests, and the release build on every push and pull request, and `.github/workflows/deb.yml` builds the Debian packages and attaches them to the GitHub release when a `v*` tag is pushed.

## Coding Style & Naming Conventions

Use standard `rustfmt` output (four-space indentation). Name modules, functions, and test cases in `snake_case`; types and traits use `UpperCamelCase`; constants use `SCREAMING_SNAKE_CASE`. Keep public `rofd-core` APIs documented: the crate denies missing documentation and forbids unsafe code. Prefer small modules, explicit errors, and resource-bounded parsing for untrusted OFD archives.

## Testing Guidelines

Add unit tests beside private implementation details and integration tests in `crates/rofd-core/tests/` for public behavior. Use behavior-focused names such as `multiple_doc_bodies_are_explicitly_unsupported_in_v02`. Reuse helpers from `tests/support/` and repository fixtures when realistic packages matter. There is no stated coverage threshold; every bug fix should include a regression test.

## Commit & Pull Request Guidelines

Follow the repository's Conventional Commit pattern: `feat(core): ...`, `fix(render): ...`, `test(core): ...`, `docs: ...`, or `build: ...`. Keep commits focused and use an imperative, concise subject. Pull requests should explain motivation and behavior changes, link relevant issues or design notes, and list verification commands. Include screenshots for Qt/QML or rendered-output changes, and ensure formatting, Clippy, and affected tests pass before review.
