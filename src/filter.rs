use glob::Pattern;
use std::path::Path;

/// Determines whether a relative file path should be included in a package
/// based on include and exclude glob patterns.
///
/// Exclude patterns take priority: if a file matches any exclude pattern, it is
/// immediately rejected, regardless of later include matches. Include patterns
/// are only evaluated after all excludes have been checked. If the path matches
/// no exclude and at least one include, it is accepted. Otherwise, the file is
/// rejected.
///
/// Patterns are compiled using the [`glob::Pattern`] type. Any pattern that
/// fails to compile (e.g., due to an invalid glob syntax) is silently skipped.
/// This design prevents a misconfigured pattern from aborting the entire
/// filtering process, though it may mask errors.
///
/// # Arguments
///
/// * `rel_path` – The file path to test, expressed relative to the package
///   root. This is typically obtained by stripping the root prefix from a full
///   path. The path is converted to a string via [`Path::to_string_lossy`],
///   meaning non‑UTF‑8 sequences are replaced with the `U+FFFD` replacement
///   character.
/// * `includes` – A list of glob patterns (e.g., `**/*.html`, `assets/*.css`)
///   that define which files are candidates for inclusion.
/// * `excludes` – A list of glob patterns that explicitly exclude files. These
///   are checked first and act as a deny‑list.
///
/// # Returns
///
/// * `true` – The path should be included (matches an include pattern and no
///   exclude patterns).
/// * `false` – The path is excluded either because it matched an exclude
///   pattern or because it failed to match any include pattern.
///
/// # Examples
///
/// ```rust
/// use std::path::Path;
/// use librawdist::filter::is_included;
///
/// let includes = vec!["**/*.rs".to_string()];
/// let excludes = vec!["**/test/**".to_string()];
///
/// assert!(is_included(Path::new("src/main.rs"), &includes, &excludes));
/// assert!(!is_included(Path::new("src/test/mod.rs"), &includes, &excludes));
/// ```
pub fn is_included(rel_path: &Path, includes: &[String], excludes: &[String]) -> bool {
    // Convert the path to a string form for glob matching. Non‑UTF‑8
    // sequences become the replacement character, which is acceptable
    // for package filtering where source file paths are nearly always
    // valid Unicode.
    let path_str = rel_path.to_string_lossy();

    // Exclude patterns are evaluated first because they have higher
    // priority. This ensures that files explicitly ignored are never
    // accidentally included, even if a broad include glob would
    // otherwise match them (e.g., an exclude for "secret.key" overrides
    // a "*.key" include).
    for pat in excludes {
        // Pattern::new may fail if the glob syntax is invalid. We
        // silently skip such patterns to avoid halting the whole
        // operation, but in a strict environment this could mask
        // configuration errors. A future improvement could log a
        // warning.
        if let Ok(pattern) = Pattern::new(pat) {
            if pattern.matches(&path_str) {
                return false;
            }
        }
    }

    // Include patterns are checked only after the path has passed all
    // excludes. The first matching include pattern causes acceptance.
    for pat in includes {
        if let Ok(pattern) = Pattern::new(pat) {
            if pattern.matches(&path_str) {
                return true;
            }
        }
    }

    // If the path matched no exclude but also no include, it is
    // implicitly excluded (conservative default).
    false
}
