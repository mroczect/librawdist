use glob::Pattern;
use std::path::Path;

/// Checks whether a relative path matches any of the include patterns and none of the excludes.
pub fn is_included(rel_path: &Path, includes: &[String], excludes: &[String]) -> bool {
    let path_str = rel_path.to_string_lossy();
    // First, if any exclude pattern matches, reject.
    for pat in excludes {
        if let Ok(pattern) = Pattern::new(pat) {
            if pattern.matches(&path_str) {
                return false;
            }
        }
    }
    // Then, if any include pattern matches, accept.
    for pat in includes {
        if let Ok(pattern) = Pattern::new(pat) {
            if pattern.matches(&path_str) {
                return true;
            }
        }
    }
    false
}
