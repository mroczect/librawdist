use glob::Pattern;
use std::path::Path;

pub fn is_included(rel_path: &Path, includes: &[String], excludes: &[String]) -> bool {
    let path_str = rel_path.to_string_lossy();
    for pat in excludes {
        if let Ok(pattern) = Pattern::new(pat) {
            if pattern.matches(&path_str) {
                return false;
            }
        }
    }
    for pat in includes {
        if let Ok(pattern) = Pattern::new(pat) {
            if pattern.matches(&path_str) {
                return true;
            }
        }
    }
    false
}
