use librawdist::fs::RealFs;
use librawdist::fs::FileSystem;
use tempfile::TempDir;

#[test]
fn test_real_fs_read_write() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("test.txt");
    let fs = RealFs;
    fs.write(&path, b"hello").unwrap();
    assert!(fs.exists(&path));
    assert_eq!(fs.read_to_string(&path).unwrap(), "hello");
}

#[test]
fn test_real_fs_create_dir_all() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("a/b/c");
    RealFs.create_dir_all(&dir).unwrap();
    assert!(RealFs.is_dir(&dir));
}

#[test]
fn test_real_fs_remove_dir_all() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().join("todel");
    RealFs.create_dir_all(&dir).unwrap();
    assert!(dir.exists());
    RealFs.remove_dir_all(&dir).unwrap();
    assert!(!dir.exists());
}

#[test]
fn test_real_fs_read_dir() {
    let tmp = TempDir::new().unwrap();
    let f1 = tmp.path().join("a");
    let f2 = tmp.path().join("b");
    std::fs::write(&f1, b"x").unwrap();
    std::fs::write(&f2, b"x").unwrap();
    let entries = RealFs.read_dir(tmp.path()).unwrap();
    assert_eq!(entries.len(), 2);
}

#[test]
fn test_real_fs_copy_file() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    let dst = tmp.path().join("dst");
    std::fs::write(&src, b"data").unwrap();
    RealFs.copy_file(&src, &dst).unwrap();
    assert_eq!(std::fs::read(&dst).unwrap(), b"data");
}

#[test]
fn test_real_fs_rename() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("old");
    let dst = tmp.path().join("new");
    std::fs::write(&src, b"x").unwrap();
    RealFs.rename(&src, &dst).unwrap();
    assert!(!src.exists());
    assert!(dst.exists());
}

#[test]
fn test_real_fs_canonicalize() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("file");
    std::fs::write(&path, b"").unwrap();
    let canon = RealFs.canonicalize(&path).unwrap();
    assert!(canon.is_absolute());
}

#[test]
fn test_real_fs_walk_dir() {
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
