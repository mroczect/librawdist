use librawdist::verify_package;
use std::path::Path;

#[test]
fn test_verify_archive_not_found() {
    let err = verify_package(Path::new("/nonexistent.rawdist")).unwrap_err();
    assert!(matches!(err, librawdist::RawdistError::InvalidInput(_)));
}

#[test]
fn test_verify_valid_archive() {
    // Buat package sederhana, lalu verifikasi
    let tmp = tempfile::TempDir::new().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(src.join("rawdist.conf"), br#"
[package]
name = "p"
version = "0.1.0"

[files]
include = ["*.txt"]
"#).unwrap();
    std::fs::write(src.join("a.txt"), b"data").unwrap();
    let config = librawdist::RawdistConfig::load_from_dir(&src).unwrap();
    let archive = tmp.path().join("p.rawdist");
    librawdist::create_package(&librawdist::fs::RealFs, &src, &archive, &config).unwrap();
    assert!(verify_package(&archive).is_ok());
}
