use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileKind {
    Directory,
    Code,
    Rust,
    C,
    Cpp,
    Python,
    Java,
    JavaScript,
    TypeScript,
    Html,
    Css,
    Json,
    Toml,
    Yaml,
    Markdown,
    Text,
    Pdf,
    Image,
    Video,
    Audio,
    Archive,
    Font,
    Shell,
    Executable,
    Symlink,
    Unknown,
}

impl FileKind {
    pub fn icon(self) -> &'static str {
        match self {
            Self::Directory => "\u{ea83}",
            Self::Code => "\u{eae9}",
            Self::Rust => "\u{e7a8}",
            Self::C => "\u{e61e}",
            Self::Cpp => "\u{e61d}",
            Self::Python => "\u{e73c}",
            Self::Java => "\u{e738}",
            Self::JavaScript => "\u{e74e}",
            Self::TypeScript => "\u{e8ca}",
            Self::Html => "\u{e736}",
            Self::Css => "\u{e749}",
            Self::Json => "\u{eb0f}",
            Self::Toml => "\u{e6b2}",
            Self::Yaml => "\u{e8eb}",
            Self::Markdown => "\u{e73e}",
            Self::Text => "\u{ec5e}",
            Self::Pdf => "\u{eaeb}",
            Self::Image => "\u{eaea}",
            Self::Video => "\u{ead9}",
            Self::Audio => "\u{ec1b}",
            Self::Archive => "\u{ea98}",
            Self::Font => "\u{ea7b}",
            Self::Shell => "\u{ea85}",
            Self::Executable => "\u{eb2c}",
            Self::Symlink => "\u{eb15}",
            Self::Unknown => "\u{ea7b}",
        }
    }

    pub fn indicator(self) -> &'static str {
        match self {
            Self::Directory => "[DIR]",
            Self::Code => "[CODE]",
            Self::Rust => "[RS]",
            Self::C => "[C]",
            Self::Cpp => "[CPP]",
            Self::Python => "[PY]",
            Self::Java => "[JAVA]",
            Self::JavaScript => "[JS]",
            Self::TypeScript => "[TS]",
            Self::Html => "[HTML]",
            Self::Css => "[CSS]",
            Self::Json => "[JSON]",
            Self::Toml => "[TOML]",
            Self::Yaml => "[YAML]",
            Self::Markdown => "[MD]",
            Self::Text => "[TXT]",
            Self::Pdf => "[PDF]",
            Self::Image => "[IMG]",
            Self::Video => "[VID]",
            Self::Audio => "[AUD]",
            Self::Archive => "[ARC]",
            Self::Font => "[FONT]",
            Self::Shell => "[SH]",
            Self::Executable => "[EXE]",
            Self::Symlink => "[LINK]",
            Self::Unknown => "[FILE]",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub display_name: String,
    pub kind: FileKind,
    pub size: u64,
    pub is_dir: bool,
    pub is_symlink: bool,
}

impl FileEntry {
    pub fn from_path(path: PathBuf) -> Self {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        let kind = detect_file_kind(&path, file_name.as_str());
        let metadata = std::fs::symlink_metadata(&path).ok();
        let len = metadata
            .as_ref()
            .and_then(|m| m.file_type().is_file().then_some(m.len()))
            .unwrap_or(0);
        Self {
            path: path.clone(),
            name: file_name,
            display_name: crate::filesystem::safety::ascii_safe_display(&path),
            kind,
            size: len,
            is_dir: metadata
                .as_ref()
                .map(|m| m.file_type().is_dir())
                .unwrap_or(false),
            is_symlink: metadata
                .as_ref()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false),
        }
    }

    pub fn extension_badge(&self) -> String {
        if self.is_dir {
            return self.kind.icon().to_string();
        }

        if self.kind != FileKind::Unknown {
            return self.kind.icon().to_string();
        }

        let ext = self
            .path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_uppercase();
        if ext.is_empty() {
            return self.kind.indicator().to_string();
        }
        format!("[{}]", ext)
    }
}

pub fn detect_file_kind(path: &Path, name: &str) -> FileKind {
    if path.is_dir() {
        return FileKind::Directory;
    }

    let lower = name.to_ascii_lowercase();
    let extension = Path::new(&lower)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if matches!(lower.as_str(), "dockerfile" | "makefile" | "justfile" | ".bashrc" | ".zshrc" | ".profile" | ".env") {
        if matches!(lower.as_str(), "dockerfile" | "makefile" | "justfile") {
            FileKind::Code
        } else {
            FileKind::Shell
        }
    } else if matches!(extension, "rs") {
        FileKind::Rust
    } else if matches!(extension, "c" | "h") {
        FileKind::C
    } else if matches!(extension, "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx") {
        FileKind::Cpp
    } else if matches!(extension, "py" | "pyw" | "pyi" | "ipynb") {
        FileKind::Python
    } else if matches!(extension, "java" | "class") {
        FileKind::Java
    } else if matches!(extension, "js" | "mjs" | "cjs") {
        FileKind::JavaScript
    } else if matches!(extension, "ts" | "tsx" | "mts" | "cts") {
        FileKind::TypeScript
    } else if matches!(extension, "html" | "htm" | "xhtml") {
        FileKind::Html
    } else if matches!(extension, "css" | "scss" | "sass" | "less") {
        FileKind::Css
    } else if matches!(extension, "go" | "rb" | "php" | "swift" | "kt" | "kts" | "scala" | "sc" | "cs" | "fs" | "fsx" | "vb" | "lua" | "pl" | "pm" | "r" | "rmd" | "jl" | "ex" | "exs" | "erl" | "hrl" | "hs" | "lhs" | "clj" | "cljs" | "cljc" | "dart" | "elm" | "vue" | "svelte" | "sql" | "graphql" | "gql" | "proto" | "asm" | "s" | "zig" | "nim" | "v" | "sol" | "m" | "mm" | "groovy" | "gradle" | "mk" | "wasm" | "wat") {
        FileKind::Code
    } else if matches!(extension, "json" | "jsonc" | "json5") {
        FileKind::Json
    } else if matches!(extension, "toml") {
        FileKind::Toml
    } else if matches!(extension, "yaml" | "yml") {
        FileKind::Yaml
    } else if matches!(extension, "md" | "markdown" | "mdx" | "rst" | "adoc") {
        FileKind::Markdown
    } else if matches!(extension, "txt" | "log" | "out" | "err" | "csv" | "tsv") {
        FileKind::Text
    } else if matches!(extension, "pdf") {
        FileKind::Pdf
    } else if matches!(extension, "png" | "jpg" | "jpeg" | "jpe" | "gif" | "bmp" | "webp" | "avif" | "heic" | "heif" | "tif" | "tiff" | "svg" | "ico" | "icns" | "raw" | "cr2" | "nef" | "arw" | "dng" | "psd" | "xcf" | "ai" | "eps") {
        FileKind::Image
    } else if matches!(extension, "mp4" | "m4v" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "mpeg" | "mpg" | "mpe" | "mpv" | "m2v" | "3gp" | "3g2" | "mts" | "m2ts" | "ts" | "vob" | "ogv" | "divx" | "asf" | "rm" | "rmvb" | "f4v") {
        FileKind::Video
    } else if matches!(extension, "mp3" | "wav" | "flac" | "aac" | "m4a" | "m4b" | "ogg" | "oga" | "opus" | "wma" | "aiff" | "aif" | "ape" | "alac" | "mid" | "midi" | "amr" | "au" | "ra" | "tta" | "wv" | "dsf" | "dff" | "ac3" | "eac3" | "caf" | "voc" | "spx") {
        FileKind::Audio
    } else if matches!(extension, "zip" | "tar" | "gz" | "tgz" | "xz" | "bz2" | "7z" | "rar" | "zst" | "lz" | "lzma" | "lz4" | "br" | "cab" | "ar" | "cpio" | "iso" | "dmg" | "deb" | "rpm" | "jar" | "war" | "ear") {
        FileKind::Archive
    } else if matches!(extension, "sh" | "bash" | "zsh" | "fish" | "ps1" | "psm1" | "bat" | "cmd") {
        FileKind::Shell
    } else if matches!(extension, "doc" | "docx" | "odt" | "rtf" | "ppt" | "pptx" | "odp" | "xls" | "xlsx" | "ods" | "epub" | "mobi") {
        FileKind::Text
    } else if matches!(extension, "ttf" | "otf" | "woff" | "woff2" | "eot") {
        FileKind::Font
    } else if std::fs::metadata(path)
        .map(|m| !m.permissions().readonly())
        .unwrap_or(false)
    {
        FileKind::Executable
    } else {
        FileKind::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_badge_is_visible_for_supported_files() {
        let rust_file = FileEntry::from_path(PathBuf::from("/tmp/main.rs"));
        let text_file = FileEntry::from_path(PathBuf::from("/tmp/readme.md"));

        assert_eq!(rust_file.extension_badge(), "\u{e7a8}");
        assert_eq!(text_file.extension_badge(), "\u{e73e}");
    }

    #[test]
    fn music_and_video_formats_share_compact_category_badges() {
        for name in ["track.mp3", "track.flac", "track.m4a", "track.opus", "track.mid"] {
            let entry = FileEntry::from_path(PathBuf::from("/tmp").join(name));
            assert_eq!(entry.kind, FileKind::Audio, "{name}");
            assert_eq!(entry.extension_badge(), "\u{ec1b}", "{name}");
        }

        for name in ["clip.mp4", "clip.mkv", "clip.mov", "clip.webm", "clip.m2ts"] {
            let entry = FileEntry::from_path(PathBuf::from("/tmp").join(name));
            assert_eq!(entry.kind, FileKind::Video, "{name}");
            assert_eq!(entry.extension_badge(), "\u{ead9}", "{name}");
        }
    }

    #[test]
    fn common_source_formats_use_the_code_category() {
        for name in ["main.go", "app.rb", "view.php", "Main.kt", "contract.sol", "Dockerfile"] {
            let entry = FileEntry::from_path(PathBuf::from("/tmp").join(name));
            assert_eq!(entry.kind, FileKind::Code, "{name}");
            assert_eq!(entry.extension_badge(), "\u{eae9}", "{name}");
        }
    }
}
