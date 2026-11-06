use std::fs;
use std::path::PathBuf;

use tfsv::filesystem::{copy_path, create_directory, delete_path, ensure_readable, ensure_safe_path, move_path, rename_path};

#[test]
fn create_directory_creates_dir() {
    let dir = std::env::temp_dir().join(format!("tfsv-create-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    assert!(create_directory(&dir).is_ok());
    assert!(dir.exists() && dir.is_dir());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn rename_and_delete_paths_work() {
    let root = std::env::temp_dir().join(format!("tfsv-op-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let original = root.join("old-name.txt");
    fs::write(&original, "hello").unwrap();

    let renamed = rename_path(&original, "new-name.txt").unwrap();
    assert!(renamed.exists());
    assert!(!original.exists());

    delete_path(&renamed).unwrap();
    assert!(!renamed.exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn copy_path_copies_directory_tree() {
    let root = std::env::temp_dir().join(format!("tfsv-copy-{}", std::process::id()));
    let source = root.join("source");
    let target = root.join("copy");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("file.txt"), "data").unwrap();

    copy_path(&source, &target).unwrap();
    assert!(target.exists());
    assert!(target.join("file.txt").exists());

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn move_path_moves_item_to_destination() {
    let root = std::env::temp_dir().join(format!("tfsv-move-{}", std::process::id()));
    let source_dir = root.join("source");
    let destination_dir = root.join("destination");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&source_dir).unwrap();
    fs::create_dir_all(&destination_dir).unwrap();
    let source = source_dir.join("move.txt");
    let destination = destination_dir.join("move.txt");
    fs::write(&source, "move data").unwrap();

    move_path(&source, &destination).unwrap();
    assert!(!source.exists());
    assert_eq!(fs::read(&destination).unwrap(), b"move data");

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn safe_path_validation_rejects_symlinks_and_missing_files() {
    let root = std::env::temp_dir().join(format!("tfsv-safety-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    let target = root.join("target.txt");
    fs::write(&target, "hello").unwrap();

    #[cfg(unix)]
    {
        let link = root.join("link.txt");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(ensure_safe_path(&link).is_err());
    }

    let missing = root.join("missing.txt");
    assert!(ensure_readable(&missing).is_err());

    let _ = fs::remove_dir_all(&root);
}
