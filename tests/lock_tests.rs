use librawdist::lock::LockFile;
use tempfile::TempDir;

#[test]
fn test_lock_load_default() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("nonexistent.lock");
    let lock = LockFile::load(&path).unwrap();
    assert!(lock.packages.is_empty());
}

#[test]
fn test_lock_save_and_load() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("Rawdist.lock");
    let mut lock = LockFile::default();
    lock.add_package("pkg", "1.0", "http://src", "abc123");
    lock.save(&path).unwrap();

    let loaded = LockFile::load(&path).unwrap();
    assert_eq!(loaded.packages.len(), 1);
    assert_eq!(loaded.packages[0].name, "pkg");
    assert_eq!(loaded.packages[0].checksum, "abc123");
}

#[test]
fn test_lock_add_replaces_existing() {
    let mut lock = LockFile::default();
    lock.add_package("x", "1", "url", "hash1");
    lock.add_package("x", "2", "url2", "hash2");
    assert_eq!(lock.packages.len(), 1);
    assert_eq!(lock.packages[0].version, "2");
}
