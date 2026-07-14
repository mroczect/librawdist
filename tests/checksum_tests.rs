mod common;
use common::MockFs;
use librawdist::checksum;
use librawdist::types::FilePatterns;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn default_patterns() -> FilePatterns {
    FilePatterns {
        include: vec!["**/*".to_string()],
        exclude: vec![],
    }
}

#[test]
fn hash_file_success() {
    let mut mock = MockFs::new();
    mock.add_file(Path::new("/test.txt"), b"hello");
    let hash = checksum::hash_file(&mock, Path::new("/test.txt")).unwrap();
    assert_eq!(
        hash,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn hash_file_empty() {
    let mut mock = MockFs::new();
    mock.add_file(Path::new("/empty.txt"), b"");
    let hash = checksum::hash_file(&mock, Path::new("/empty.txt")).unwrap();
    assert_eq!(
        hash,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn hash_file_io_error() {
    let mut mock = MockFs::new();
    mock.read_error = Some(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    let err = checksum::hash_file(&mock, Path::new("/x")).unwrap_err();
    assert!(matches!(err, librawdist::RawdistError::Io(_)));
}

#[test]
fn generate_checksums_missing_rawdist_conf() {
    let mut mock = MockFs::new();
    mock.add_file(Path::new("/other.txt"), b"data");
    let patterns = default_patterns();
    let err = checksum::generate_checksums(&mock, Path::new("/"), &patterns).unwrap_err();
    assert!(matches!(err, librawdist::RawdistError::MissingFile { .. }));
}

#[test]
fn generate_checksums_includes_rawdist_conf_automatically() {
    let mut mock = MockFs::new();
    mock.add_file(Path::new("/rawdist.conf"), b"conf");
    mock.add_file(Path::new("/theme.css"), b"css");
    let patterns = FilePatterns {
        include: vec!["*.css".to_string()],
        exclude: vec![],
    };
    let map = checksum::generate_checksums(&mock, Path::new("/"), &patterns).unwrap();
    assert!(map.contains_key(Path::new("rawdist.conf")));
    assert!(map.contains_key(Path::new("theme.css")));
    assert_eq!(map.len(), 2);
}

#[test]
fn generate_checksums_exclude_pattern() {
    let mut mock = MockFs::new();
    mock.add_file(Path::new("/rawdist.conf"), b"conf");
    mock.add_file(Path::new("/include.css"), b"css");
    mock.add_file(Path::new("/exclude.js"), b"js");
    let patterns = FilePatterns {
        include: vec!["*".to_string()],
        exclude: vec!["*.js".to_string()],
    };
    let map = checksum::generate_checksums(&mock, Path::new("/"), &patterns).unwrap();
    assert!(map.contains_key(Path::new("rawdist.conf")));
    assert!(map.contains_key(Path::new("include.css")));
    assert!(!map.contains_key(Path::new("exclude.js")));
}

#[test]
fn generate_checksums_walk_error() {
    let mut mock = MockFs::new();
    mock.walk_error = Some(std::io::Error::new(std::io::ErrorKind::Other, "walk fail"));
    let patterns = default_patterns();
    let err = checksum::generate_checksums(&mock, Path::new("/"), &patterns).unwrap_err();
    assert!(matches!(err, librawdist::RawdistError::Io(_)));
}

#[test]
fn generate_checksums_strip_prefix_error() {
    let mut mock = MockFs::new();
    mock.add_file(Path::new("/other/rawdist.conf"), b"conf");
    mock.add_file(Path::new("/other/file.txt"), b"data");
    let patterns = default_patterns();
    let err =
        checksum::generate_checksums(&mock, Path::new("/something_else"), &patterns).unwrap_err();
    assert!(matches!(err, librawdist::RawdistError::PathTraversal(_)));
}

#[test]
fn format_checksums_basic() {
    let mut map = BTreeMap::new();
    map.insert(PathBuf::from("a.txt"), "hash_a".to_string());
    map.insert(PathBuf::from("b.txt"), "hash_b".to_string());
    let out = checksum::format_checksums(&map);
    assert_eq!(out, "hash_a  a.txt\nhash_b  b.txt\n");
}

#[test]
fn format_checksums_empty() {
    let map = BTreeMap::new();
    let out = checksum::format_checksums(&map);
    assert_eq!(out, "");
}

#[test]
fn parse_checksums_standard() {
    let content = "abc123  file.txt\ndef456  dir/file.log\n";
    let map = checksum::parse_checksums(content).unwrap();
    assert_eq!(map.len(), 2);
    assert_eq!(map.get(Path::new("file.txt")).unwrap(), "abc123");
    assert_eq!(map.get(Path::new("dir/file.log")).unwrap(), "def456");
}

#[test]
fn parse_checksums_fallback_split() {
    let content = "hash1 path1\nhash2  path2";
    let map = checksum::parse_checksums(content).unwrap();
    assert_eq!(map.get(Path::new("path1")).unwrap(), "hash1");
    assert_eq!(map.get(Path::new("path2")).unwrap(), "hash2");
}

#[test]
fn parse_checksums_empty_lines() {
    let content = "hash  path\n\nhash2  path2\n";
    let map = checksum::parse_checksums(content).unwrap();
    assert_eq!(map.len(), 2);
}

#[test]
fn parse_checksums_whitespace_only() {
    let content = "   \n  ";
    let map = checksum::parse_checksums(content).unwrap();
    assert!(map.is_empty());
}

#[test]
fn parse_checksums_single_word_no_path() {
    let content = "hashonly";
    let map = checksum::parse_checksums(content).unwrap();
    assert_eq!(map.get(Path::new("")).unwrap(), "hashonly");
}

#[test]
fn parse_checksums_line_with_only_spaces_and_no_hash() {
    let content = "  path";
    let map = checksum::parse_checksums(content).unwrap();
    assert_eq!(map.get(Path::new("")).unwrap(), "path");
}
