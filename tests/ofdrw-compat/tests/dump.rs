//! Manual debugging utility: dump rofd's rendering of fixtures to PNG files.
//!
//! Run with e.g.
//! `ROFD_DUMP="converter/y.ofd:0,1,2 converter/pattern类型.ofd:0" cargo test -p ofdrw-compat --test dump -- --ignored --nocapture`
//! PNGs are written to `target/ofdrw-compat-dump/` so they never interfere
//! with the committed references.

mod support;

#[test]
#[ignore = "manual debugging utility"]
fn dump_rendered_pages() {
    let spec = std::env::var("ROFD_DUMP").expect("set ROFD_DUMP=fixture:pages,...");
    let output_root =
        std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("ofdrw-compat-dump");
    for entry in spec.split_whitespace() {
        let (relative, pages) = entry.split_once(':').expect("fixture:page,page");
        let document = support::open_document(&support::fixture(relative));
        for page_index in pages
            .split(',')
            .map(|index| index.parse::<usize>().unwrap())
        {
            let page = document.page(page_index).unwrap();
            match support::render_page(&page) {
                Ok(surface) => {
                    let output_dir = output_root.join(relative);
                    std::fs::create_dir_all(&output_dir).unwrap();
                    let output = output_dir.join(format!("{page_index}.png"));
                    let mut file = std::fs::File::create(&output).unwrap();
                    surface.write_to_png(&mut file).unwrap();
                    eprintln!("wrote {}", output.display());
                }
                Err(error) => eprintln!("{relative} page {page_index}: render failed: {error}"),
            }
        }
    }
}
