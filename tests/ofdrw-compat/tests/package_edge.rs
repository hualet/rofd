//! Smoke tests over the non-standard package fixtures carried by ofdrw.
//!
//! Sources (ofdrw commit 7459e35, Apache-2.0):
//! - `ofdrw-layout/src/test/resources/no_page_container.ofd`
//!   (used by `org.ofdrw.layout.cases.watermark.WatermarkTest`)
//! - `ofdrw-layout/src/test/resources/拿来主义_page6.ofd`
//! - `ofdrw-sign/src/test/resources/namespace_no_std.ofd`
//!   (used by `org.ofdrw.sign.OFDSignerTest`)
//! - `ofdrw-converter/src/test/resources/不规范资源路径.ofd`
//!   (manual repro file referenced from `OFD2IMGTest`/`OFD2SVGTest`)

mod support;

use support::{fixture, open_document};

/// Every listed fixture must open and yield a positive size for each page,
/// matching how ofdrw's tests treat these files as readable documents.
fn assert_package_loads(relative: &str) {
    let path = fixture(relative);
    let document = open_document(&path);
    assert!(
        document.page_count() >= 1,
        "{}: expected at least one page",
        path.display()
    );
    for index in 0..document.page_count() {
        let page = document
            .page(index)
            .unwrap_or_else(|error| panic!("{}: page {index} failed: {error}", path.display()));
        let size = page.size();
        assert!(
            size.width > 0.0 && size.height > 0.0,
            "{}: page {index} has non-positive size {size:?}",
            path.display()
        );
    }
}

#[test]
#[ignore = "known parser gap: JB2 image format unsupported (see KNOWN_LOAD_FAILURES)"]
fn no_page_container_loads() {
    assert_package_loads("layout/no_page_container.ofd");
}

#[test]
fn namespace_no_std_loads() {
    assert_package_loads("sign/namespace_no_std.ofd");
}

#[test]
fn nalazhuyi_page6_loads() {
    assert_package_loads("layout/拿来主义_page6.ofd");
}

#[test]
#[ignore = "known parser gap: leading-slash resource paths rejected (see KNOWN_LOAD_FAILURES)"]
fn non_standard_resource_paths_load() {
    assert_package_loads("converter/不规范资源路径.ofd");
}

/// Opens every migrated fixture and loads every page, so parser regressions
/// surface even for files without dedicated assertions yet. Fixtures listed
/// in `KNOWN_LOAD_FAILURES` are expected to fail; the test fails when an
/// unlisted fixture breaks or a listed one starts passing (remove the entry
/// once the parser handles it).
#[test]
fn all_migrated_fixtures_open_and_load_pages() {
    let mut failures = Vec::new();
    for module in ["reader", "converter", "layout", "sign"] {
        let directory = support::fixtures_dir().join(module);
        let mut entries: Vec<_> = std::fs::read_dir(&directory)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "ofd"))
            .collect();
        entries.sort();
        for path in entries {
            let relative = format!("{module}/{}", path.file_name().unwrap().to_string_lossy());
            match (
                support::try_load_all_pages(&path),
                support::known_load_failure(&relative),
            ) {
                (Ok(pages), None) => eprintln!("ok: {relative} ({pages} pages)"),
                (Err(error), Some(reason)) => {
                    eprintln!("known failure: {relative} ({reason}): {error}")
                }
                (Ok(pages), Some(reason)) => failures.push(format!(
                    "{relative}: now loads ({pages} pages); remove its KNOWN_LOAD_FAILURES entry ({reason})"
                )),
                (Err(error), None) => failures.push(format!("{relative}: {error}")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "unexpected fixture results:\n{}",
        failures.join("\n")
    );
}
