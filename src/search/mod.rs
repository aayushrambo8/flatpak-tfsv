pub fn search_files(root: &std::path::Path, query: &str) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(root) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.to_ascii_lowercase().contains(&query.to_ascii_lowercase()) {
                results.push(path);
            }
        }
    }
    results
}
