use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use crate::error::LibrawdistError;
use crate::fs::FileSystem;
use crate::filter;
use crate::types::FilePatterns;

/// Compute SHA-256 hash of a file.
pub fn hash_file(fs: &dyn FileSystem, path: &Path) -> Result<String, LibrawdistError> {
    let data = fs.read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}

/// Generate checksums for all files under `root` that match the given patterns.
/// Always includes `Librawdist.conf` even if not in the patterns.
pub fn generate_checksums(
    fs: &dyn FileSystem,
    root: &Path,
    patterns: &FilePatterns,
) -> Result<BTreeMap<PathBuf, String>, LibrawdistError> {
    let mut map = BTreeMap::new();
    let files = fs.walk_dir(root)?;
    for full_path in files {
        let rel = full_path.strip_prefix(root).unwrap().to_path_buf();
        // Always include the config file itself
        if rel.to_str() == Some("Librawdist.conf")
            || filter::is_included(&rel, &patterns.include, &patterns.exclude)
        {
            let hash = hash_file(fs, &full_path)?;
            map.insert(rel, hash);
        }
    }
    // Ensure Librawdist.conf is present
    if !map.contains_key(Path::new("Librawdist.conf")) {
        return Err(LibrawdistError::MissingFile {
            path: PathBuf::from("Librawdist.conf"),
        });
    }
    Ok(map)
}

/// Format a BTreeMap of checksums into the standard checksums.sha256 file content.
pub fn format_checksums(checksums: &BTreeMap<PathBuf, String>) -> String {
    let mut out = String::new();
    for (path, hash) in checksums {
        out.push_str(&format!("{}  {}\n", hash, path.display()));
    }
    out
}

/// Parse checksums.sha256 content back into a map.
pub fn parse_checksums(content: &str) -> Result<BTreeMap<PathBuf, String>, LibrawdistError> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Split by two spaces (standard format) or fallback to whitespace if not found.
        if let Some(pos) = line.find("  ") {
            let hash = &line[..pos];
            let path = line[pos + 2..].trim();
            map.insert(PathBuf::from(path), hash.to_string());
        } else {
            // Fallback: split by any whitespace (less strict)
            let mut parts = line.split_whitespace();
            let hash = parts.next().ok_or_else(|| {
                LibrawdistError::Config(format!("Invalid checksum line: {}", line))
            })?;
            let path = parts.next().unwrap_or("");
            map.insert(PathBuf::from(path), hash.to_string());
        }
    }
    Ok(map)
}
