pub struct EditorState {
    pub path: std::path::PathBuf,
}

impl EditorState {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self { path }
    }
}
