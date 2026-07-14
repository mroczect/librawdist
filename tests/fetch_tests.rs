mod common;
use common::{MockFs, MockHttp};
use librawdist::RawdistError;
use librawdist::fetch::fetch_package;
use librawdist::fs::RealFs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn fetch_success_with_dest() {
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("pkg.rawdist");
    let http = MockHttp {
        response: Ok(b"filedata".to_vec()),
    };
    let fs = RealFs;
    let path = fetch_package(&fs, &http, "http://example.com/pkg.rawdist", Some(&dest)).unwrap();
    assert_eq!(path, dest);
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "filedata");
}

#[test]
fn fetch_success_to_cache() {
    let tmp = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("XDG_CACHE_HOME", tmp.path());
    }
    let http = MockHttp {
        response: Ok(b"data".to_vec()),
    };
    let fs = RealFs;
    let path = fetch_package(&fs, &http, "http://x.com/pkg.rawdist", None).unwrap();
    assert!(path.starts_with(tmp.path()));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "data");
    unsafe {
        std::env::remove_var("XDG_CACHE_HOME");
    }
}

#[test]
fn fetch_network_error() {
    let http = MockHttp {
        response: Err("fail".into()),
    };
    let fs = RealFs;
    let err = fetch_package(&fs, &http, "http://x", Some(Path::new("/tmp/x"))).unwrap_err();
    assert!(matches!(err, RawdistError::Network(_)));
}

#[test]
fn fetch_write_error() {
    let http = MockHttp {
        response: Ok(b"x".to_vec()),
    };
    let fs = RealFs;
    let err = fetch_package(
        &fs,
        &http,
        "http://x",
        Some(Path::new("/nonexistent_dir_/file")),
    )
    .unwrap_err();
    assert!(matches!(err, RawdistError::Io(_)));
}

#[test]
fn fetch_create_dir_error() {
    let mut mock = MockFs::new();
    mock.create_dir_error = Some(std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "denied",
    ));
    let http = MockHttp {
        response: Ok(b"data".to_vec()),
    };
    let err = fetch_package(&mock, &http, "http://x", None).unwrap_err();
    assert!(matches!(err, RawdistError::Io(_)));
}
