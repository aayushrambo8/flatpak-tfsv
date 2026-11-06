use std::fs;
use std::path::{Path, PathBuf};

use super::{FileEntry, FileKind};

pub fn load_directory(path: &Path) -> std::io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    if !path.exists() {
        return Ok(entries);
    }
    let mut dir_entries = fs::read_dir(path)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .collect::<Vec<PathBuf>>();
    dir_entries.sort();
    for item in dir_entries {
        let entry = FileEntry::from_path(item);
        entries.push(entry);
    }
    Ok(entries)
}

pub fn parent_directory(path: &Path) -> Option<PathBuf> {
    path.parent().map(Path::to_path_buf)
}

pub fn is_directory(path: &Path) -> bool {
    path.is_dir()
}

pub fn file_kind_label(path: &Path) -> &'static str {
    if !path.exists() {
        return FileKind::Unknown.indicator();
    }
    let kind = super::metadata::detect_file_kind(path, path.file_name().and_then(|s| s.to_str()).unwrap_or(""));
    kind.indicator()
}
