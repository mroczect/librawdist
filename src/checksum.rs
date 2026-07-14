use crate::error::RawdistError;
use crate::filter;
use crate::fs::FileSystem;
use crate::types::FilePatterns;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn hash_file(fs: &dyn FileSystem, path: &Path) -> Result<String, RawdistError> {
    let data = fs.read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}

pub fn generate_checksums(
    fs: &dyn FileSystem,
    root: &Path,
    patterns: &FilePatterns,
) -> Result<BTreeMap<PathBuf, String>, RawdistError> {
    let mut map = BTreeMap::new();
    let files = fs.walk_dir(root)?;
    for full_path in files {
        let rel = full_path
            .strip_prefix(root)
            .map_err(|_| RawdistError::PathTraversal(full_path.clone()))?
            .to_path_buf();
        if rel.to_str() == Some("rawdist.conf")
            || filter::is_included(&rel, &patterns.include, &patterns.exclude)
        {
            let hash = hash_file(fs, &full_path)?;
            map.insert(rel, hash);
        }
    }
    if !map.contains_key(Path::new("rawdist.conf")) {
        return Err(RawdistError::MissingFile {
            path: PathBuf::from("rawdist.conf"),
        });
    }
    Ok(map)
}

pub fn format_checksums(checksums: &BTreeMap<PathBuf, String>) -> String {
    let mut out = String::new();
    for (path, hash) in checksums {
        out.push_str(&format!("{}  {}\n", hash, path.display()));
    }
    out
}

pub fn parse_checksums(content: &str) -> Result<BTreeMap<PathBuf, String>, RawdistError> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(pos) = line.find("  ") {
            let hash = &line[..pos];
            let path = line[pos + 2..].trim();
            map.insert(PathBuf::from(path), hash.to_string());
        } else {
            let mut parts = line.split_whitespace();
            let hash = parts
                .next()
                .ok_or_else(|| RawdistError::Config(format!("Invalid checksum line: {}", line)))?;
            let path = parts.next().unwrap_or("");
            map.insert(PathBuf::from(path), hash.to_string());
        }
    }
    Ok(map)
}
