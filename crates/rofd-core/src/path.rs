use crate::{Error, Result};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PackagePath(String);

impl PackagePath {
    pub(crate) fn new(value: &str) -> Result<Self> {
        normalize(value, &[]).map(Self)
    }

    pub(crate) fn resolve(&self, value: &str) -> Result<Self> {
        let mut base = self.0.split('/').collect::<Vec<_>>();
        base.pop();
        normalize(value, &base).map(Self)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

fn normalize(value: &str, base: &[&str]) -> Result<String> {
    let replaced = value.replace('\\', "/");
    // A leading slash denotes a package-root-absolute path. Real-world
    // producers use it liberally (e.g. `/Doc_0/Document.xml`), and ofdrw
    // resolves it against the package root, so do the same instead of
    // rejecting it. Normalization below still forbids escaping the root
    // via `..`, so this cannot turn into a zip-slip primitive.
    let (replaced, base) = match replaced.strip_prefix('/') {
        Some(rest) => (rest, &[][..]),
        None => (replaced.as_str(), base),
    };

    let mut parts = base.iter().map(|part| (*part).to_owned()).collect::<Vec<_>>();
    for part in replaced.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(invalid_path(value));
                }
            }
            component if component.contains(':') || component.contains('\0') => {
                return Err(invalid_path(value));
            }
            component => parts.push(component.to_owned()),
        }
    }

    if parts.is_empty() {
        return Err(invalid_path(value));
    }
    Ok(parts.join("/"))
}

fn invalid_path(value: &str) -> Error {
    Error::InvalidValue {
        field: "package path",
        value: value.to_owned(),
        path: None,
    }
}

#[cfg(test)]
mod tests {
    use super::PackagePath;

    #[test]
    fn resolves_paths_relative_to_declaring_file() {
        let document = PackagePath::new("Doc_0/Document.xml").unwrap();
        assert_eq!(
            document
                .resolve("Pages/Page_0/Content.xml")
                .unwrap()
                .as_str(),
            "Doc_0/Pages/Page_0/Content.xml"
        );
    }

    #[test]
    fn treats_leading_slash_as_package_root_absolute() {
        assert_eq!(
            PackagePath::new("/Doc_0/Document.xml").unwrap().as_str(),
            "Doc_0/Document.xml"
        );
        let page = PackagePath::new("Doc_0/Pages/Page_0/Content.xml").unwrap();
        assert_eq!(
            page.resolve("/Doc_0/Res/2.gif").unwrap().as_str(),
            "Doc_0/Res/2.gif"
        );
    }

    #[test]
    fn rejects_root_escapes_and_bare_separators() {
        assert!(PackagePath::new("../OFD.xml").is_err());
        assert!(PackagePath::new("/").is_err());
        assert!(PackagePath::new("/../OFD.xml").is_err());
        let document = PackagePath::new("Doc_0/Document.xml").unwrap();
        assert!(document.resolve("../../outside").is_err());
        assert!(document.resolve("/../outside").is_err());
    }

    #[test]
    fn normalizes_dot_and_backslash_separators() {
        assert_eq!(
            PackagePath::new(r"./Doc_0\Pages/Page_0/Content.xml")
                .unwrap()
                .as_str(),
            "Doc_0/Pages/Page_0/Content.xml"
        );
    }

    #[test]
    fn collapses_empty_segments_but_preserves_whitespace_in_names() {
        assert_eq!(
            PackagePath::new("Doc_0//Pages/Page_0/Content.xml")
                .unwrap()
                .as_str(),
            "Doc_0/Pages/Page_0/Content.xml"
        );
        assert_eq!(
            PackagePath::new("Doc_0/Images Fonts/font_4.ttf")
                .unwrap()
                .as_str(),
            "Doc_0/Images Fonts/font_4.ttf"
        );
        // Whitespace-only segments are kept verbatim: entries are matched
        // exactly, so a padded segment simply misses at lookup time.
        assert_eq!(
            PackagePath::new("Doc_0/   /Content.xml").unwrap().as_str(),
            "Doc_0/   /Content.xml"
        );
    }

    #[test]
    fn resolves_parent_references_that_stay_inside_package() {
        let page = PackagePath::new("Doc_0/Pages/Page_0/Content.xml").unwrap();
        assert_eq!(
            page.resolve("../../Signs/Signatures.xml").unwrap().as_str(),
            "Doc_0/Signs/Signatures.xml"
        );
    }
}
