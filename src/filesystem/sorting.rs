use crate::filesystem::FileEntry;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortMode {
    Name,
    Type,
    Size,
}

pub fn sort_entries(entries: &mut [FileEntry], mode: SortMode) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| match mode {
                SortMode::Name => a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()),
                SortMode::Type => a.kind.indicator().cmp(b.kind.indicator()),
                SortMode::Size => b.size.cmp(&a.size),
            })
    });
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::filesystem::FileKind;

    fn entry(name: &str, is_dir: bool, size: u64) -> FileEntry {
        FileEntry {
            path: PathBuf::from(name),
            name: name.to_string(),
            display_name: name.to_string(),
            kind: if is_dir { FileKind::Directory } else { FileKind::Unknown },
            size,
            is_dir,
            is_symlink: false,
        }
    }

    #[test]
    fn directories_sort_before_files_in_every_mode() {
        for mode in [SortMode::Name, SortMode::Type, SortMode::Size] {
            let mut entries = vec![entry("file-a", false, 100), entry("folder-a", true, 0), entry("file-b", false, 200), entry("folder-b", true, 0)];
            sort_entries(&mut entries, mode);
            assert!(entries[..2].iter().all(|entry| entry.is_dir));
            assert!(entries[2..].iter().all(|entry| !entry.is_dir));
        }
    }
}
