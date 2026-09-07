use std::path::PathBuf;

use rofd_core::{Document, LoadOptions};

#[test]
fn package_structure_conformance_files_open_and_load_all_pages() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/ofd_files");
    let mut entries: Vec<_> = std::fs::read_dir(&directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "ofd"))
        .collect();
    entries.sort();
    assert_eq!(entries.len(), 13);

    for path in entries {
        let document = Document::open(&path, LoadOptions::default())
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert!(document.page_count() > 0, "{}", path.display());
        for index in 0..document.page_count() {
            document
                .page(index)
                .unwrap_or_else(|error| panic!("{} page {index}: {error}", path.display()));
        }
    }
}
