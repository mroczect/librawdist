mod common;
use common::MockFs;
use librawdist::RawdistError;
use librawdist::lock::LockFile;
use std::path::Path;

#[test]
fn lock_load_default() {
    let _fs = MockFs::new();
    let lock = LockFile::load(&_fs, Path::new("/nonexistent")).unwrap();
    assert!(lock.packages.is_empty());
}

#[test]
fn lock_save_and_load() {
    let tmp = tempfile::TempDir::new().unwrap();
    let path = tmp.path().join("Rawdist.lock");
    let mut lock = LockFile::default();
    lock.add_package("pkg", "1.0", "http://src", "abc123");
    let real_fs = librawdist::fs::RealFs;
    lock.save(&real_fs, &path).unwrap();
    let loaded = LockFile::load(&real_fs, &path).unwrap();
    assert_eq!(loaded.packages.len(), 1);
    assert_eq!(loaded.packages[0].checksum, "abc123");
}

#[test]
fn lock_add_replaces_existing() {
    let mut lock = LockFile::default();
    lock.add_package("x", "1", "url", "hash1");
    lock.add_package("x", "2", "url2", "hash2");
    assert_eq!(lock.packages.len(), 1);
    assert_eq!(lock.packages[0].version, "2");
}

#[test]
fn lock_load_invalid_toml() {
    let mut fs = MockFs::new();
    fs.add_file(Path::new("/invalid.lock"), b"not toml");
    let err = LockFile::load(&fs, Path::new("/invalid.lock")).unwrap_err();
    assert!(matches!(err, RawdistError::TomlParse { .. }));
}

#[test]
fn lock_save_error() {
    let mut fs = MockFs::new();
    fs.write_error = Some(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    let lock = LockFile::default();
    let err = lock.save(&fs, Path::new("/willfail")).unwrap_err();
    assert!(matches!(err, RawdistError::Io(_)));
}
