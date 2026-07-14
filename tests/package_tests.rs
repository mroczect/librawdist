mod common;
use common::MockFs;
use flate2::Compression;
use flate2::write::GzEncoder;
use librawdist::RawdistConfig;
use librawdist::RawdistError;
use librawdist::fs::RealFs;
use librawdist::package::{create_package, extract_to_temp, move_extracted};
use std::io::Write;
use std::path::Path;
use tar::Builder;
use tempfile::TempDir;

fn make_src(config_content: &[u8], files: Vec<(&str, &[u8])>) -> (TempDir, RawdistConfig) {
    let fs = RealFs;
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("rawdist.conf"), config_content).unwrap();
    for (name, data) in files {
        std::fs::write(dir.path().join(name), data).unwrap();
    }
    let config = RawdistConfig::load_from_dir(&fs, dir.path()).unwrap();
    (dir, config)
}

fn create_tar_with_path_traversal(path: &Path) {
    let file = std::fs::File::create(path).unwrap();
    let mut enc = GzEncoder::new(file, Compression::default());

    let mut header = [0u8; 512];
    let name = b"../escape.txt";
    header[0..name.len()].copy_from_slice(name);
    header[156] = b'0';
    let size_str = b"00000000000";
    header[124..124 + size_str.len()].copy_from_slice(size_str);
    header[257..262].copy_from_slice(b"ustar");
    for i in 148..156 {
        header[i] = b' ';
    }
    let checksum: u32 = header.iter().map(|&b| b as u32).sum();
    let chksum_str = format!("{:06o}\0 ", checksum);
    header[148..148 + chksum_str.len()].copy_from_slice(chksum_str.as_bytes());

    enc.write_all(&header).unwrap();
    let zeros = [0u8; 1024];
    enc.write_all(&zeros).unwrap();
    enc.finish().unwrap();
}

#[test]
fn create_and_extract_success() {
    let fs = RealFs;
    let config_toml = br#"
[package]
name = "test"
version = "0.1.0"
[rawssg]
type = "theme"
[files]
include = ["*.txt"]
[install]
target_dir = "themes/test"
"#;
    let (src, config) = make_src(config_toml, vec![("a.txt", b"hello")]);
    let archive = src.path().join("pkg.rawdist");
    create_package(&fs, src.path(), &archive, &config).unwrap();
    assert!(archive.exists());

    let extracted = extract_to_temp(&fs, &archive).unwrap();
    assert!(extracted.join("a.txt").exists());
    assert!(!extracted.join("checksums.sha256").exists());
    std::fs::remove_dir_all(&extracted).unwrap();
}

#[test]
fn extract_missing_checksums_file() {
    let fs = RealFs;
    let tmp = TempDir::new().unwrap();
    let bad_archive = tmp.path().join("no_checksum.rawdist");
    let file = std::fs::File::create(&bad_archive).unwrap();
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);
    let mut header = tar::Header::new_gnu();
    header.set_path("dummy.txt").unwrap();
    header.set_size(4);
    header.set_cksum();
    tar.append_data(&mut header, "dummy.txt", b"test".as_ref())
        .unwrap();
    let enc = tar.into_inner().unwrap();
    enc.finish().unwrap();
    let err = extract_to_temp(&fs, &bad_archive).unwrap_err();
    assert!(matches!(err, RawdistError::MissingFile { .. }));
}

#[test]
fn extract_path_traversal() {
    let fs = RealFs;
    let tmp = TempDir::new().unwrap();
    let archive_path = tmp.path().join("bad.rawdist");
    create_tar_with_path_traversal(&archive_path);

    let err = extract_to_temp(&fs, &archive_path).unwrap_err();
    assert!(matches!(err, RawdistError::PathTraversal(_)));
}

#[test]
fn extract_archive_too_large() {
    let mut mock = MockFs::new();
    mock.add_file(Path::new("/large.rawdist"), b"fake");
    mock.metadata_size = 600_000_000; // 600MB
    let err = extract_to_temp(&mock, Path::new("/large.rawdist")).unwrap_err();
    assert!(matches!(err, RawdistError::ArchiveTooLarge { .. }));
}

#[test]
fn move_extracted_target_exists() {
    let fs = RealFs;
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir(&src).unwrap();
    let target = tmp.path().join("target");
    std::fs::create_dir(&target).unwrap();
    let err = move_extracted(&fs, &src, &target).unwrap_err();
    assert!(matches!(err, RawdistError::Config(_)));
}

#[test]
fn move_extracted_success() {
    let fs = RealFs;
    let tmp = TempDir::new().unwrap();
    let src = tmp.path().join("src");
    std::fs::create_dir(&src).unwrap();
    std::fs::write(src.join("dummy.txt"), b"test").unwrap();
    let target = tmp.path().join("dest");
    move_extracted(&fs, &src, &target).unwrap();
    assert!(!src.exists());
    assert!(target.exists());
    assert!(target.join("dummy.txt").exists());
}

#[test]
fn move_extracted_create_parent_error() {
    let mut mock = MockFs::new();
    mock.create_dir_error = Some(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    let target = Path::new("/new_parent/dest");
    let err = move_extracted(&mock, Path::new("/src"), target).unwrap_err();
    assert!(matches!(err, RawdistError::Io(_)));
}
