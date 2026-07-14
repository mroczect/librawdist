mod common;
use librawdist::fetch::{fetch_package, HttpClient};
use librawdist::RawdistError;
use std::path::Path;
use tempfile::TempDir;

struct MockHttp {
    response: Result<Vec<u8>, String>,
}

impl HttpClient for MockHttp {
    fn get(&self, _url: &str) -> Result<Vec<u8>, RawdistError> {
        match &self.response {
            Ok(data) => Ok(data.clone()),
            Err(msg) => Err(RawdistError::Network(msg.clone())),
        }
    }
}

#[test]
fn test_fetch_success_to_dest() {
    let tmp = TempDir::new().unwrap();
    let dest = tmp.path().join("pkg.rawdist");
    let mock = MockHttp {
        response: Ok(b"filedata".to_vec()),
    };
    let path = fetch_package(&mock, "http://example.com/pkg.rawdist", Some(&dest)).unwrap();
    assert_eq!(path, dest);
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "filedata");
}

#[test]
fn test_fetch_success_to_cache() {
    let tmp = TempDir::new().unwrap();
    // override cache dir
    std::env::set_var("XDG_CACHE_HOME", tmp.path());
    let mock = MockHttp {
        response: Ok(b"data".to_vec()),
    };
    let path = fetch_package(&mock, "http://x.com/pkg.rawdist", None).unwrap();
    assert!(path.starts_with(tmp.path()));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "data");
    std::env::remove_var("XDG_CACHE_HOME");
}

#[test]
fn test_fetch_network_error() {
    let mock = MockHttp {
        response: Err("fail".to_string()),
    };
    let err = fetch_package(&mock, "http://x", Some(Path::new("/tmp/x"))).unwrap_err();
    assert!(matches!(err, RawdistError::Network(_)));
}

#[test]
fn test_fetch_write_error() {
    let mock = MockHttp {
        response: Ok(b"x".to_vec()),
    };
    // Tulis ke path yang tidak mungkin (direktori tidak ada)
    let err =
        fetch_package(&mock, "http://x", Some(Path::new("/nonexistent_dir_/file"))).unwrap_err();
    assert!(matches!(err, RawdistError::Io(_)));
}
