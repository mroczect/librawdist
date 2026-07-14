use librawdist::filter;
use std::path::Path;

#[test]
fn test_is_included_match() {
    assert!(filter::is_included(
        Path::new("test.css"),
        &vec!["*.css".to_string()],
        &[]
    ));
}

#[test]
fn test_is_included_exclude_overrides() {
    assert!(!filter::is_included(
        Path::new("test.css"),
        &vec!["*.css".to_string()],
        &vec!["*.css".to_string()]
    ));
}

#[test]
fn test_is_included_no_match() {
    assert!(!filter::is_included(
        Path::new("test.js"),
        &vec!["*.css".to_string()],
        &[]
    ));
}

#[test]
fn test_is_included_invalid_pattern_ignored() {
    // Pattern invalid tidak mempengaruhi
    assert!(!filter::is_included(
        Path::new("file.txt"),
        &vec!["[invalid".to_string()],
        &[]
    ));
}

#[test]
fn test_is_included_multiple_includes() {
    assert!(filter::is_included(
        Path::new("file.txt"),
        &vec!["*.txt".to_string(), "*.md".to_string()],
        &[]
    ));
}
