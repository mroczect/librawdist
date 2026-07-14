mod common;
use librawdist::fs::{FileSystem, RealFs};
use std::io::ErrorKind;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn real_fs_read_write() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("test.txt");
    let fs = RealFs;
    fs.write(&path, b"hello").unwrap();
    assert!(fs.exists(&path));
    assert_eq!(fs.read_to_string(&path).unwrap(), "hello");
}

#[test]
fn real_fs_create_dir_all() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("a/b/c");
    RealFs.create_dir_all(&dir).unwrap();
    assert!(RealFs.is_dir(&dir));
}

#[test]
fn real_fs_remove_dir_all() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("todel");
    RealFs.create_dir_all(&dir).unwrap();
    RealFs.remove_dir_all(&dir).unwrap();
    assert!(!dir.exists());
}

#[test]
fn real_fs_remove_file() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("file.txt");
    std::fs::write(&file, b"data").unwrap();
    RealFs.remove_file(&file).unwrap();
    assert!(!file.exists());
}

#[test]
fn real_fs_read_dir() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("a"), b"").unwrap();
    std::fs::write(tmp.path().join("b"), b"").unwrap();
    let entries = RealFs.read_dir(tmp.path()).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn real_fs_copy_file() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    std::fs::write(&src, b"data").unwrap();
    RealFs.copy_file(&src, &dst).unwrap();
    assert_eq!(std::fs::read(&dst).unwrap(), b"data");
}

#[test]
fn real_fs_rename() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("old");
    let dst = tmp.path().join("new");
    std::fs::write(&src, b"x").unwrap();
    RealFs.rename(&src, &dst).unwrap();
    assert!(!src.exists());
    assert!(dst.exists());
}

#[test]
fn real_fs_canonicalize() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("file");
    std::fs::write(&path, b"").unwrap();
    let canon = RealFs.canonicalize(&path).unwrap();
    assert!(canon.is_absolute());
}

#[test]
fn real_fs_walk_dir() {
    let tmp = TempDir::new().unwrap();
    std::fs::write(tmp.path().join("a.txt"), b"").unwrap();
    std::fs::create_dir(tmp.path().join("sub")).unwrap();
    std::fs::write(tmp.path().join("sub/b.txt"), b"").unwrap();
    let files = RealFs.walk_dir(tmp.path()).unwrap();
    let names: Vec<_> = files
        .iter()
        .map(|p| p.file_name().unwrap().to_str().unwrap())
        .collect();
    assert!(names.contains(&"a.txt"));
    assert!(names.contains(&"b.txt"));
}

#[test]
fn real_fs_read_nonexistent() {
    let err = RealFs
        .read(Path::new("/tmp/nonexistent_xyz123"))
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[test]
fn real_fs_read_dir_not_a_directory() {
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("file.txt");
    std::fs::write(&file, b"data").unwrap();
    let err = RealFs.read_dir(&file).unwrap_err();
    assert!(err.kind() == ErrorKind::NotADirectory || err.kind() == ErrorKind::Other);
}

#[test]
fn real_fs_remove_dir_nonexistent() {
    let err = RealFs
        .remove_dir_all(Path::new("/tmp/nonexistent_xyz123"))
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[test]
fn real_fs_copy_file_nonexistent_source() {
    let tmp = TempDir::new().unwrap();
    let dst = tmp.path().join("dst");
    let err = RealFs
        .copy_file(Path::new("/tmp/nonexistent_src"), &dst)
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[test]
fn mock_fs_write_error() {
    let mut mock = common::MockFs::new();
    mock.write_error = Some(std::io::Error::new(ErrorKind::PermissionDenied, "denied"));
    let err = mock.write(Path::new("/test"), b"x").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::PermissionDenied);
}

#[test]
fn mock_fs_remove_file_error() {
    let mut mock = common::MockFs::new();
    mock.remove_file_error = Some(std::io::Error::new(ErrorKind::PermissionDenied, "denied"));
    let err = mock.remove_file(Path::new("/test")).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::PermissionDenied);
}
