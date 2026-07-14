use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Returns the absolute path to the compiled `rawdist` binary.
fn rawdist_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rawdist"))
}

fn run_rawdist(args: &[&str]) -> std::process::Output {
    Command::new(rawdist_binary())
        .args(args)
        .output()
        .expect("failed to execute rawdist binary")
}

fn run_rawdist_in_dir(dir: &std::path::Path, args: &[&str]) -> std::process::Output {
    Command::new(rawdist_binary())
        .args(args)
        .current_dir(dir)
        .output()
        .expect("failed to execute rawdist binary")
}

fn create_source_dir() -> TempDir {
    let dir = TempDir::new().unwrap();
    let config = r#"
[package]
name = "test-cli"
version = "0.1.0"

[rawssg]
type = "theme"

[files]
include = ["*.txt"]

[install]
target_dir = "themes/test-cli"
"#;
    fs::write(dir.path().join("rawdist.conf"), config).unwrap();
    fs::write(dir.path().join("dummy.txt"), b"hello world").unwrap();
    dir
}

#[test]
fn help_prints_usage_and_exits_zero() {
    let output = run_rawdist(&["--help"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("A robust package manager for rawssg"));
}

#[test]
fn version_prints_version_and_exits_zero() {
    let output = run_rawdist(&["--version"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn list_empty_manifest_prints_no_packages() {
    let tmp = TempDir::new().unwrap();
    let output = run_rawdist_in_dir(tmp.path(), &["list"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No installed packages."));
}

#[test]
fn pack_creates_archive() {
    let src = create_source_dir();
    let archive = src.path().join("out.rawdist");
    let output = run_rawdist_in_dir(src.path(), &["pack", "-o", archive.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(archive.exists());
}

#[test]
fn pack_missing_source_dir_fails() {
    let output = run_rawdist(&["pack", "/nonexistent/path"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Source directory not found") || stderr.contains("Failed to load configuration"));
}

#[test]
fn verify_valid_archive_succeeds() {
    let src = create_source_dir();
    let archive = src.path().join("test.rawdist");
    run_rawdist_in_dir(src.path(), &["pack", "-o", archive.to_str().unwrap()]);
    let output = run_rawdist_in_dir(src.path(), &["verify", archive.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Verification successful"));
}

#[test]
fn verify_missing_archive_fails() {
    let output = run_rawdist(&["verify", "/nonexistent.rawdist"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Archive not found"));
}

#[test]
fn install_and_remove_local_package() {
    let src = create_source_dir();
    let archive = src.path().join("test.rawdist");
    run_rawdist_in_dir(src.path(), &["pack", "-o", archive.to_str().unwrap()]);

    let target = TempDir::new().unwrap();
    let manifest = target.path().join("rawssg-packages.toml");
    let install_target = target.path().join("custom_dest");

    let output = run_rawdist_in_dir(
        target.path(),
        &[
            "install",
            archive.to_str().unwrap(),
            "-t",
            install_target.to_str().unwrap(),
            "-m",
            manifest.to_str().unwrap(),
        ],
    );
    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(install_target.exists());

    let list_output = run_rawdist_in_dir(target.path(), &["list", "-m", manifest.to_str().unwrap()]);
    assert!(list_output.status.success());
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(stdout.contains("test-cli"));

    let uninstall_output = run_rawdist_in_dir(
        target.path(),
        &["uninstall", "test-cli", "-m", manifest.to_str().unwrap()],
    );
    assert!(
        uninstall_output.status.success(),
        "uninstall failed: {}",
        String::from_utf8_lossy(&uninstall_output.stderr)
    );
    assert!(!install_target.exists());

    let list_output = run_rawdist_in_dir(target.path(), &["list", "-m", manifest.to_str().unwrap()]);
    assert!(list_output.status.success());
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(stdout.contains("No installed packages."));
}

#[test]
fn install_missing_file_fails() {
    let output = run_rawdist(&["install", "/nonexistent.rawdist"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("File not found"));
}

#[test]
fn uninstall_nonexistent_fails() {
    let tmp = TempDir::new().unwrap();
    let output = run_rawdist_in_dir(tmp.path(), &["uninstall", "ghost"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("NotInstalled") || stderr.contains("not found"));
}

#[test]
fn fetch_invalid_url_fails() {
    let tmp = TempDir::new().unwrap();
    let output = run_rawdist_in_dir(tmp.path(), &["fetch", "ftp://invalid.url"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid URL") || stderr.contains("must start with http"));
}

#[test]
fn info_on_source_dir() {
    let src = create_source_dir();
    let output = run_rawdist_in_dir(src.path(), &["info", "."]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test-cli") && stdout.contains("0.1.0"));
}

#[test]
fn info_on_archive() {
    let src = create_source_dir();
    let archive = src.path().join("test.rawdist");
    run_rawdist_in_dir(src.path(), &["pack", "-o", archive.to_str().unwrap()]);
    let output = run_rawdist_in_dir(src.path(), &["info", archive.to_str().unwrap()]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test-cli") && stdout.contains("0.1.0"));
}
