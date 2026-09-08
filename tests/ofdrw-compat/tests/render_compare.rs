//! Rendering comparisons against reference PNGs produced by ofdrw itself.
//!
//! The ofdrw-converter test suite (`ofdrw-converter/src/test/java/OFD2IMGTest.java`
//! and friends) renders fixtures to PNG for manual inspection. This test makes
//! that comparison automatic: `tools/render-references.sh` renders every
//! migrated fixture with ofdrw's `ImageMaker` at 96 DPI into `references/`,
//! and each page rendered by rofd is diffed against the corresponding PNG
//! within a per-fixture mismatch threshold (see `support::max_mismatch_fraction`).

mod support;

use support::{fixture, known_load_failure, open_document, references_dir, render_page};

/// Every reference directory corresponds to one migrated fixture; rofd must
/// render each reference page within the fixture's mismatch threshold.
#[test]
fn rendered_pages_match_ofdrw_references() {
    let references = references_dir();
    assert!(
        references.join("manifest.json").exists(),
        "references missing; generate them with tools/render-references.sh"
    );

    let mut compared = 0usize;
    let mut skipped = 0usize;
    let mut failures = Vec::new();

    let mut modules: Vec<_> = std::fs::read_dir(&references)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect();
    modules.sort();
    for module_dir in modules {
        let module = module_dir.file_name().unwrap().to_string_lossy();
        let mut fixture_dirs: Vec<_> = std::fs::read_dir(&module_dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_dir())
            .collect();
        fixture_dirs.sort();
        for fixture_dir in fixture_dirs {
            let name = fixture_dir.file_name().unwrap().to_string_lossy();
            let relative = format!("{module}/{name}");
            if let Some(reason) = known_load_failure(&relative) {
                eprintln!("skip {relative}: known parser gap ({reason})");
                skipped += 1;
                continue;
            }
            if let Some(reason) = support::unusable_reference(&relative) {
                eprintln!("skip {relative}: ofdrw reference unusable ({reason})");
                skipped += 1;
                continue;
            }

            let mut pages: Vec<_> = std::fs::read_dir(&fixture_dir)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.extension().is_some_and(|ext| ext == "png")
                        && path
                            .file_stem()
                            .unwrap()
                            .to_string_lossy()
                            .parse::<usize>()
                            .is_ok()
                })
                .collect();
            pages.sort_by_key(|path| {
                path.file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .parse::<usize>()
                    .unwrap()
            });

            let document = open_document(&fixture(&relative));
            let known_render = support::known_render_failure(&relative);
            let mut render_failed = false;
            for reference in pages {
                let page_index: usize = reference
                    .file_stem()
                    .unwrap()
                    .to_string_lossy()
                    .parse()
                    .unwrap();
                let label = format!("{relative} page {page_index}");
                let Some(page) = document.page(page_index).ok() else {
                    failures.push(format!("{label}: rofd has no such page"));
                    continue;
                };
                let mut surface = match render_page(&page) {
                    Ok(surface) => surface,
                    Err(error) => {
                        if let Some(reason) = known_render {
                            eprintln!("skip {relative}: known render gap ({reason}): {error}");
                            render_failed = true;
                            break;
                        }
                        failures.push(format!("{label}: render failed: {error}"));
                        continue;
                    }
                };
                let report = support::diff_against_reference(&mut surface, &reference);
                let threshold = support::max_mismatch_fraction(&relative);
                compared += 1;
                if report.mismatch_fraction > threshold {
                    failures.push(format!(
                        "{label}: mismatch {:.2}% exceeds {:.2}% (actual {:?}, reference {:?})",
                        report.mismatch_fraction * 100.0,
                        threshold * 100.0,
                        report.actual_size,
                        report.reference_size
                    ));
                } else {
                    eprintln!(
                        "ok {label}: mismatch {:.2}% (threshold {:.2}%)",
                        report.mismatch_fraction * 100.0,
                        threshold * 100.0
                    );
                }
            }
            if let Some(reason) = known_render {
                if !render_failed {
                    failures.push(format!(
                        "{relative}: now renders; remove its KNOWN_RENDER_FAILURES entry ({reason})"
                    ));
                }
            }
        }
    }

    eprintln!("compared {compared} pages, skipped {skipped} fixtures");
    assert!(compared > 0, "no reference pages compared");
    assert!(
        failures.is_empty(),
        "rendering mismatches:\n{}",
        failures.join("\n")
    );
}
