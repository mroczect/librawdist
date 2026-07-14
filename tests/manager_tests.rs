mod common;
use common::{MockFs, MockHttp};
use librawdist::{PackageManager, RawdistError};

#[test]
fn manager_list_empty() {
    let fs = MockFs::new();
    let http = MockHttp {
        response: Ok(vec![]),
    };
    let mgr = PackageManager::new(&fs, &http, "/manifest.toml".into(), "/lock.toml".into());
    let list = mgr.list().unwrap();
    assert!(list.packages.is_empty());
}

#[test]
fn manager_uninstall_not_installed() {
    let fs = MockFs::new();
    let http = MockHttp {
        response: Ok(vec![]),
    };
    let mgr = PackageManager::new(&fs, &http, "/manifest.toml".into(), "/lock.toml".into());
    let err = mgr.uninstall("ghost").unwrap_err();
    assert!(matches!(err, RawdistError::NotInstalled(_)));
}

#[test]
fn manager_install_from_url_fetch_error() {
    let fs = MockFs::new();
    let http = MockHttp {
        response: Err("fail".into()),
    };
    let mgr = PackageManager::new(&fs, &http, "/manifest.toml".into(), "/lock.toml".into());
    let err = mgr.install_from_url("http://x", None).unwrap_err();
    assert!(matches!(err, RawdistError::Network(_)));
}
