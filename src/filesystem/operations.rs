use std::fs;
#[cfg(unix)]
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum FileSystemError {
    #[error("path is not safe: {0}")]
    UnsafePath(String),
    #[error("permission denied for path: {0}")]
    PermissionDenied(String),
    #[error("filesystem operation failed: {0}")]
    OperationFailed(String),
}

fn is_special_file(file_type: std::fs::FileType) -> bool {
    file_type.is_fifo() || file_type.is_socket() || file_type.is_block_device() || file_type.is_char_device()
}

pub fn ensure_safe_path(path: &Path) -> Result<(), FileSystemError> {
    if path.as_os_str().is_empty() || path == Path::new("") {
        return Err(FileSystemError::UnsafePath(path.display().to_string()));
    }

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            let parent = path.parent().filter(|p| !p.as_os_str().is_empty());
            if let Some(parent) = parent {
                ensure_safe_path(parent)?;
            }
            return Ok(());
        }
    };

    let file_type = metadata.file_type();
    if file_type.is_symlink() || is_special_file(file_type) {
        return Err(FileSystemError::UnsafePath(path.display().to_string()));
    }

    let is_root = path == Path::new("/");
    if is_root {
        return Err(FileSystemError::UnsafePath(path.display().to_string()));
    }

    Ok(())
}

pub fn ensure_readable(path: &Path) -> Result<(), FileSystemError> {
    if !path.exists() {
        return Err(FileSystemError::OperationFailed(path.display().to_string()));
    }

    let metadata = fs::symlink_metadata(path)
        .map_err(|_| FileSystemError::OperationFailed(path.display().to_string()))?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || is_special_file(file_type) {
        return Err(FileSystemError::PermissionDenied(path.display().to_string()));
    }

    let mode = metadata.permissions().mode() & 0o777;
    let readable = if file_type.is_dir() {
        mode & 0o555 != 0
    } else {
        mode & 0o444 != 0
    };

    if !readable {
        return Err(FileSystemError::PermissionDenied(path.display().to_string()));
    }

    Ok(())
}

pub fn create_directory(path: &Path) -> Result<(), FileSystemError> {
    ensure_safe_path(path)?;
    fs::create_dir_all(path)
        .map_err(|_| FileSystemError::OperationFailed(path.display().to_string()))?;
    Ok(())
}

pub fn rename_path(from: &Path, new_name: &str) -> Result<PathBuf, FileSystemError> {
    ensure_readable(from)?;
    ensure_safe_path(from)?;
    let parent = from.parent().ok_or_else(|| {
        FileSystemError::OperationFailed(format!("missing parent for {}", from.display()))
    })?;
    let target = parent.join(new_name);
    ensure_safe_path(&target)?;
    fs::rename(from, &target)
        .map(|_| target)
        .map_err(|_| FileSystemError::OperationFailed(from.display().to_string()))
}

pub fn delete_path(path: &Path) -> Result<(), FileSystemError> {
    ensure_safe_path(path)?;
    ensure_readable(path)?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| FileSystemError::OperationFailed(path.display().to_string()))?;
    if metadata.file_type().is_symlink() {
        fs::remove_file(path)
            .map_err(|_| FileSystemError::OperationFailed(path.display().to_string()))?;
    } else if metadata.file_type().is_dir() {
        fs::remove_dir_all(path)
            .map_err(|_| FileSystemError::OperationFailed(path.display().to_string()))?;
    } else {
        fs::remove_file(path)
            .map_err(|_| FileSystemError::OperationFailed(path.display().to_string()))?;
    }
    Ok(())
}

pub fn copy_path(from: &Path, to: &Path) -> Result<(), FileSystemError> {
    ensure_safe_path(from)?;
    ensure_safe_path(to)?;
    ensure_readable(from)?;
    let metadata = fs::symlink_metadata(from)
        .map_err(|_| FileSystemError::OperationFailed(from.display().to_string()))?;
    if metadata.file_type().is_symlink() || is_special_file(metadata.file_type()) {
        return Err(FileSystemError::UnsafePath(from.display().to_string()));
    }
    if metadata.file_type().is_dir() {
        fs::create_dir_all(to)
            .map_err(|_| FileSystemError::OperationFailed(to.display().to_string()))?;
        for entry in fs::read_dir(from)
            .map_err(|_| FileSystemError::OperationFailed(from.display().to_string()))?
        {
            let entry = entry.map_err(|_| FileSystemError::OperationFailed(from.display().to_string()))?;
            let source = entry.path();
            let target = to.join(entry.file_name());
            copy_path(&source, &target)?;
        }
    } else {
        fs::copy(from, to)
            .map_err(|_| FileSystemError::OperationFailed(format!("{} -> {}", from.display(), to.display())))?;
    }
    Ok(())
}

pub fn move_path(from: &Path, to: &Path) -> Result<(), FileSystemError> {
    ensure_safe_path(from)?;
    ensure_safe_path(to)?;
    ensure_readable(from)?;
    if to.exists() {
        return Err(FileSystemError::OperationFailed(format!(
            "destination already exists: {}",
            to.display()
        )));
    }

    match fs::rename(from, to) {
        Ok(()) => Ok(()),
        Err(_) => {
            copy_path(from, to)?;
            delete_path(from)
        }
    }
}
