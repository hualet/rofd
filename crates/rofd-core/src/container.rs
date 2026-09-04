use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read};
use std::sync::Mutex;

use zip::ZipArchive;

use crate::path::PackagePath;
use crate::{Error, ResourceLimits, Result};

pub(crate) struct Container {
    archive: Mutex<ZipArchive<Cursor<Vec<u8>>>>,
    indexes: HashMap<PackagePath, usize>,
    sizes: HashMap<PackagePath, u64>,
    limits: ResourceLimits,
}

impl std::fmt::Debug for Container {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Container")
            .field("entry_count", &self.indexes.len())
            .field("limits", &self.limits)
            .finish_non_exhaustive()
    }
}

impl Container {
    pub(crate) fn from_bytes(bytes: Vec<u8>, limits: ResourceLimits) -> Result<Self> {
        let mut archive = ZipArchive::new(Cursor::new(bytes))
            .map_err(|error| Error::Container(error.to_string()))?;
        if archive.len() > limits.max_entries {
            return Err(Error::LimitExceeded(format!(
                "{} entries exceeds {}",
                archive.len(),
                limits.max_entries
            )));
        }

        let mut indexes = HashMap::new();
        let mut sizes = HashMap::new();
        let mut names = HashSet::new();
        let mut total_size = 0_u64;
        for index in 0..archive.len() {
            let entry = archive
                .by_index(index)
                .map_err(|error| Error::Container(error.to_string()))?;
            let path = PackagePath::new(entry.name())?;
            if !names.insert(path.clone()) {
                return Err(Error::Container(format!(
                    "duplicate normalized entry {}",
                    path.as_str()
                )));
            }
            if entry.size() > limits.max_entry_size {
                return Err(Error::LimitExceeded(format!(
                    "entry {} is {} bytes",
                    path.as_str(),
                    entry.size()
                )));
            }
            total_size = total_size
                .checked_add(entry.size())
                .ok_or_else(|| Error::LimitExceeded("declared ZIP size overflow".to_owned()))?;
            if total_size > limits.max_total_size {
                return Err(Error::LimitExceeded(format!(
                    "declared total size {total_size} exceeds {}",
                    limits.max_total_size
                )));
            }
            sizes.insert(path.clone(), entry.size());
            indexes.insert(path, index);
        }

        Ok(Self {
            archive: Mutex::new(archive),
            indexes,
            sizes,
            limits,
        })
    }

    pub(crate) fn read(&self, path: &PackagePath) -> Result<Vec<u8>> {
        self.read_with_limit(path, self.limits.max_entry_size, "entry")
    }

    pub(crate) fn read_with_limit(
        &self,
        path: &PackagePath,
        max_bytes: u64,
        resource_name: &str,
    ) -> Result<Vec<u8>> {
        let index = self
            .indexes
            .get(path)
            .copied()
            .ok_or_else(|| Error::MissingEntry(path.as_str().to_owned()))?;
        let declared_size = *self
            .sizes
            .get(path)
            .ok_or_else(|| Error::Container("ZIP size index is inconsistent".to_owned()))?;
        if declared_size > max_bytes {
            return Err(Error::LimitExceeded(format!(
                "{resource_name} {} is {declared_size} bytes, exceeding limit {max_bytes}",
                path.as_str()
            )));
        }
        let capacity = usize::try_from(declared_size).map_err(|_| {
            Error::LimitExceeded(format!(
                "{resource_name} {} is too large for this platform",
                path.as_str()
            ))
        })?;
        let mut archive = self
            .archive
            .lock()
            .map_err(|_| Error::Container("ZIP archive lock is poisoned".to_owned()))?;
        let mut entry = archive
            .by_index(index)
            .map_err(|error| Error::Container(error.to_string()))?;
        let mut bytes = Vec::with_capacity(capacity);
        let read_limit = max_bytes.saturating_add(1);
        entry
            .by_ref()
            .take(read_limit)
            .read_to_end(&mut bytes)
            .map_err(|error| Error::Container(error.to_string()))?;
        let actual_size = u64::try_from(bytes.len())
            .map_err(|_| Error::LimitExceeded(format!("{resource_name} size exceeds u64")))?;
        if actual_size > max_bytes {
            return Err(Error::LimitExceeded(format!(
                "{resource_name} {} exceeded {max_bytes} bytes while reading",
                path.as_str(),
            )));
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use zip::{write::SimpleFileOptions, ZipWriter};

    use super::Container;
    use crate::path::PackagePath;
    use crate::{Error, ResourceLimits};

    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        for (name, contents) in entries {
            writer
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    #[test]
    fn reads_a_normalized_entry() {
        let container = Container::from_bytes(
            archive(&[("OFD.xml", b"<OFD/>")]),
            ResourceLimits::default(),
        )
        .unwrap();
        let contents = container
            .read(&PackagePath::new("./OFD.xml").unwrap())
            .unwrap();
        assert_eq!(contents, b"<OFD/>");
    }

    #[test]
    fn rejects_duplicate_normalized_names() {
        let error = Container::from_bytes(
            archive(&[("OFD.xml", b"one"), ("./OFD.xml", b"two")]),
            ResourceLimits::default(),
        )
        .unwrap_err();
        assert!(matches!(error, Error::Container(_)));
    }

    #[test]
    fn rejects_declared_total_size_over_limit() {
        let limits = ResourceLimits {
            max_entries: 10,
            max_entry_size: 4,
            max_total_size: 4,
            ..ResourceLimits::default()
        };
        let error = Container::from_bytes(archive(&[("OFD.xml", b"12345")]), limits).unwrap_err();
        assert!(matches!(error, Error::LimitExceeded(_)));
    }
}
