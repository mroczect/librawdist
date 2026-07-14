use flate2::write::GzEncoder;
use flate2::Compression;
use librawdist::fs::RealFs;
use librawdist::package::{create_package, extract_to_temp, move_extracted};
use librawdist::RawdistConfig;
use tar::Builder;
use tempfile::TempDir;

fn make_src() -> (TempDir, RawdistConfig) {
    let dir = TempDir::new().unwrap();
    std::fs::write(
        dir.path().join("rawdist.conf"),
        br#"
[package]
name = "test"
version = "0.1.0"

[files]
include = ["*.txt"]
"#,
    )
    .unwrap();
    std::fs::write(dir.path().join("a.txt"), b"hello").unwrap();
    let config = RawdistConfig::load_from_dir(dir.path()).unwrap();
    (dir, config)
}

#[test]
fn test_create_and_extract() {
    let (src, config) = make_src();
    let archive = src.path().join("pkg.rawdist");
    let fs = RealFs;
    create_package(&fs, src.path(), &archive, &config).unwrap();
    assert!(archive.exists());

    let extracted = extract_to_temp(&fs, &archive).unwrap();
    assert!(extracted.join("a.txt").exists());
    assert!(!extracted.join("checksums.sha256").exists());
    // cleanup
    std::fs::remove_dir_all(&extracted).unwrap();
}

#[test]
fn test_extract_missing_checksums_file() {
    // Buat archive tanpa checksums.sha256 secara manual
    let tmp = TempDir::new().unwrap();
    let bad_archive = tmp.path().join("no_checksum.rawdist");
    let file = std::fs::File::create(&bad_archive).unwrap();
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);
    // Tambahkan file biasa saja, tidak ada checksums.sha256
    let mut header = tar::Header::new_gnu();
    header.set_path("dummy.txt").unwrap();
    header.set_size(4);
    header.set_cksum();
    tar.append_data(&mut header, "dummy.txt", b"test".as_ref())
        .unwrap();
    let enc = tar.into_inner().unwrap();
    enc.finish().unwrap();

    // Ekstrak, harus error MissingFile (checksums.sha256)
    let err = extract_to_temp(&RealFs, &bad_archive).unwrap_err();
    assert!(matches!(err, librawdist::RawdistError::MissingFile { .. }));
}

#[test]
fn test_move_extracted_target_exists() {
    let src_dir = TempDir::new().unwrap();
    let target = src_dir.path().join("target");
    std::fs::create_dir(&target).unwrap();
    let err = move_extracted(src_dir.path(), &target).unwrap_err();
    assert!(matches!(err, librawdist::RawdistError::Config(_)));
}

#[test]
fn test_move_extracted_success() {
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(src.join("dummy.txt"), b"test").unwrap();

    let target = tmp.path().join("dest");
    move_extracted(&src, &target).unwrap();

    assert!(!src.exists());
    assert!(target.exists());
    assert!(target.join("dummy.txt").exists());
}