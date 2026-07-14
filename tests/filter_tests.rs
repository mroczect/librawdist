use librawdist::filter;
use std::path::Path;

#[test]
fn is_included_match() {
    assert!(filter::is_included(
        Path::new("test.css"),
        &["*.css".to_string()],
        &[]
    ));
}

#[test]
fn is_included_exclude_overrides_include() {
    assert!(!filter::is_included(
        Path::new("test.css"),
        &["*.css".to_string()],
        &["*.css".to_string()]
    ));
}

#[test]
fn is_included_no_match() {
    assert!(!filter::is_included(
        Path::new("test.js"),
        &["*.css".to_string()],
        &[]
    ));
}

#[test]
fn is_included_invalid_pattern_ignored() {
    assert!(!filter::is_included(
        Path::new("file.txt"),
        &["[invalid".to_string()],
        &[]
    ));
}

#[test]
fn is_included_multiple_includes() {
    assert!(filter::is_included(
        Path::new("file.txt"),
        &["*.txt".to_string(), "*.md".to_string()],
        &[]
    ));
}

#[test]
fn is_included_subdirectory_pattern() {
    assert!(filter::is_included(
        Path::new("subdir/file.css"),
        &["**/*.css".to_string()],
        &[]
    ));
}

#[test]
fn is_included_exclude_unmatched() {
    assert!(filter::is_included(
        Path::new("file.css"),
        &["*.css".to_string()],
        &["*.js".to_string()]
    ));
}
