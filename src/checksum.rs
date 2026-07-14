use crate::error::RawdistError;
use crate::filter;
use crate::fs::FileSystem;
use crate::types::FilePatterns;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Computes the SHA-256 checksum of a file.
///
/// Reads the entire file contents using the provided [`FileSystem`] abstraction
/// and feeds the bytes into a SHA-256 hasher. The resulting digest is
/// hex-encoded before being returned. This function is used both during package
/// creation and during verification of extracted archives.
///
/// # Arguments
///
/// * `fs` - A reference to a [`FileSystem`] implementation. Enables injection of
///   real or mock filesystems for testing.
/// * `path` - The path to the file whose checksum is to be computed. The path
///   is expected to be valid and reachable through `fs`.
///
/// # Returns
///
/// * `Ok(String)` – A lowercase hexadecimal string representing the SHA-256
///   digest (64 characters).
/// * `Err(RawdistError)` – Propagates any I/O error from the underlying
///   [`FileSystem::read`] call or a hex-encoding failure (extremely unlikely).
///
/// # Panics
///
/// This function will not panic under normal operation. The hasher and hex
/// encoder are infallible in practice.
///
/// # Examples
///
/// ```rust
/// # use std::path::Path;
/// # use librawdist::fs::FileSystem;
/// # use librawdist::checksum::hash_file;
/// # struct DummyFs;
/// # impl FileSystem for DummyFs {
/// #     fn read(&self, _p: &Path) -> std::io::Result<Vec<u8>> { Ok(b"data".to_vec()) }
/// #     // ... other methods omitted for brevity
/// #     fn read_to_string(&self, _: &Path) -> std::io::Result<String> { todo!() }
/// #     fn write(&self, _: &Path, _: &[u8]) -> std::io::Result<()> { todo!() }
/// #     fn create_dir_all(&self, _: &Path) -> std::io::Result<()> { todo!() }
/// #     fn remove_dir_all(&self, _: &Path) -> std::io::Result<()> { todo!() }
/// #     fn remove_file(&self, _: &Path) -> std::io::Result<()> { todo!() }
/// #     fn exists(&self, _: &Path) -> bool { todo!() }
/// #     fn is_dir(&self, _: &Path) -> bool { todo!() }
/// #     fn is_file(&self, _: &Path) -> bool { todo!() }
/// #     fn read_dir(&self, _: &Path) -> std::io::Result<Vec<std::path::PathBuf>> { todo!() }
/// #     fn copy_file(&self, _: &Path, _: &Path) -> std::io::Result<u64> { todo!() }
/// #     fn rename(&self, _: &Path, _: &Path) -> std::io::Result<()> { todo!() }
/// #     fn canonicalize(&self, _: &Path) -> std::io::Result<std::path::PathBuf> { todo!() }
/// #     fn walk_dir(&self, _: &Path) -> std::io::Result<Vec<std::path::PathBuf>> { todo!() }
/// #     fn metadata(&self, _: &Path) -> std::io::Result<std::fs::Metadata> { todo!() }
/// # }
/// let fs = DummyFs;
/// let hash = hash_file(&fs, Path::new("dummy")).unwrap();
/// assert_eq!(hash.len(), 64);
/// ```
pub fn hash_file(fs: &dyn FileSystem, path: &Path) -> Result<String, RawdistError> {
    let data = fs.read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    // Hex encoding produces a deterministic, human-readable representation of the digest.
    Ok(hex::encode(hasher.finalize()))
}

/// Generates a checksum manifest for a directory tree according to include/exclude patterns.
///
/// Walks the entire `root` directory via the provided [`FileSystem`], computes
/// the SHA-256 hash for every file that is either `rawdist.conf` or matches the
/// supplied [`FilePatterns`] (after exclusion filtering). The resulting map
/// uses relative paths as keys, guaranteeing a deterministic iteration order
/// thanks to [`BTreeMap`].
///
/// The mandatory presence of `rawdist.conf` is enforced after processing all
/// discovered files; if the configuration file is not among the hashed entries,
/// a [`RawdistError::MissingFile`] is returned.
///
/// # Arguments
///
/// * `fs` - A reference to a [`FileSystem`] implementation, used for walking
///   the directory and reading files.
/// * `root` - The root directory from which relative paths will be computed.
///   Must be a valid directory that exists in `fs`.
/// * `patterns` - Inclusion and exclusion glob patterns from the package
///   configuration.
///
/// # Returns
///
/// * `Ok(BTreeMap<PathBuf, String>)` – A map from relative file path to its hex
///   SHA-256 hash. The map is guaranteed to contain the entry `rawdist.conf`.
/// * `Err(RawdistError)` – If directory walking fails, a file cannot be read,
///   path stripping fails (indicating a traversal attempt), or `rawdist.conf`
///   is missing.
///
/// # Panics
///
/// This function does not panic.
///
/// # Examples
///
/// ```rust
/// # use std::path::{Path, PathBuf};
/// # use librawdist::fs::FileSystem;
/// # use librawdist::types::FilePatterns;
/// # use librawdist::checksum::generate_checksums;
/// # struct MockFs {
/// #     files: Vec<PathBuf>,
/// #     content: Vec<u8>,
/// # }
/// # impl FileSystem for MockFs {
/// #     fn walk_dir(&self, _: &Path) -> std::io::Result<Vec<PathBuf>> { Ok(self.files.clone()) }
/// #     fn read(&self, _: &Path) -> std::io::Result<Vec<u8>> { Ok(self.content.clone()) }
/// #     // ... implement other required methods minimally
/// #     fn read_to_string(&self, _: &Path) -> std::io::Result<String> { todo!() }
/// #     fn write(&self, _: &Path, _: &[u8]) -> std::io::Result<()> { todo!() }
/// #     fn create_dir_all(&self, _: &Path) -> std::io::Result<()> { todo!() }
/// #     fn remove_dir_all(&self, _: &Path) -> std::io::Result<()> { todo!() }
/// #     fn remove_file(&self, _: &Path) -> std::io::Result<()> { todo!() }
/// #     fn exists(&self, _: &Path) -> bool { todo!() }
/// #     fn is_dir(&self, _: &Path) -> bool { todo!() }
/// #     fn is_file(&self, _: &Path) -> bool { todo!() }
/// #     fn read_dir(&self, _: &Path) -> std::io::Result<Vec<PathBuf>> { todo!() }
/// #     fn copy_file(&self, _: &Path, _: &Path) -> std::io::Result<u64> { todo!() }
/// #     fn rename(&self, _: &Path, _: &Path) -> std::io::Result<()> { todo!() }
/// #     fn canonicalize(&self, _: &Path) -> std::io::Result<PathBuf> { todo!() }
/// #     fn metadata(&self, _: &Path) -> std::io::Result<std::fs::Metadata> { todo!() }
/// # }
/// let fs = MockFs {
///     files: vec![PathBuf::from("root/rawdist.conf"), PathBuf::from("root/index.html")],
///     content: b"hello".to_vec(),
/// };
/// let patterns = FilePatterns { include: vec!["*.html".into()], exclude: vec![] };
/// let map = generate_checksums(&fs, Path::new("root"), &patterns).unwrap();
/// assert!(map.contains_key(Path::new("rawdist.conf")));
/// ```
pub fn generate_checksums(
    fs: &dyn FileSystem,
    root: &Path,
    patterns: &FilePatterns,
) -> Result<BTreeMap<PathBuf, String>, RawdistError> {
    // Use BTreeMap for deterministic output order; essential for reproducible checksum files.
    let mut map = BTreeMap::new();
    let files = fs.walk_dir(root)?;
    for full_path in files {
        // Compute a relative path and immediately detect path‑traversal attempts.
        let rel = full_path
            .strip_prefix(root)
            .map_err(|_| RawdistError::PathTraversal(full_path.clone()))?
            .to_path_buf();
        // Always include the mandatory configuration file, in addition to the glob‑matched files.
        if rel.to_str() == Some("rawdist.conf")
            || filter::is_included(&rel, &patterns.include, &patterns.exclude)
        {
            let hash = hash_file(fs, &full_path)?;
            map.insert(rel, hash);
        }
    }
    // The `rawdist.conf` file is the cornerstone of the package definition.
    // Its absence indicates a malformed source directory.
    if !map.contains_key(Path::new("rawdist.conf")) {
        return Err(RawdistError::MissingFile {
            path: PathBuf::from("rawdist.conf"),
        });
    }
    Ok(map)
}

/// Formats a checksum map into the standard `<hash>  <path>` line format.
///
/// This is the canonical textual representation used in `checksums.sha256`
/// files. Entries are emitted in the sorted order of the [`BTreeMap`] keys,
/// ensuring reproducibility.
///
/// # Arguments
///
/// * `checksums` - A reference to a [`BTreeMap`] mapping relative [`PathBuf`]
///   to their hex SHA-256 hash string.
///
/// # Returns
///
/// A single [`String`] containing one line per entry, each terminated by a
/// newline (`\n`). The lines follow the format `<hash><two spaces><path>`.
///
/// # Examples
///
/// ```rust
/// # use std::path::PathBuf;
/// # use std::collections::BTreeMap;
/// # use librawdist::checksum::format_checksums;
/// let mut map = BTreeMap::new();
/// map.insert(PathBuf::from("file.txt"), "abcdef".to_string());
/// let out = format_checksums(&map);
/// assert_eq!(out, "abcdef  file.txt\n");
/// ```
pub fn format_checksums(checksums: &BTreeMap<PathBuf, String>) -> String {
    let mut out = String::new();
    // Iteration order of BTreeMap is deterministic, guaranteeing stable checksum files.
    for (path, hash) in checksums {
        // The double space is required for compatibility with common `sha256sum` tooling.
        out.push_str(&format!("{}  {}\n", hash, path.display()));
    }
    out
}

/// Parses a checksum manifest string back into a map.
///
/// Attempts to parse each non‑empty line using the standard “double‑space”
/// separator (matching the output of [`format_checksums`]). If a line does not
/// contain the double‑space delimiter, the parser falls back to a
/// whitespace‑split heuristic to recover as much information as possible.
/// Lines that cannot provide a hash at all trigger a [`RawdistError::Config`]
/// error.
///
/// # Arguments
///
/// * `content` - The contents of a `checksums.sha256` file as a string slice.
///
/// # Returns
///
/// * `Ok(BTreeMap<PathBuf, String>)` – The parsed checksums. Paths may be empty
///   strings if the fallback parser encounters a line with only a hash.
/// * `Err(RawdistError)` – If a non‑empty line cannot be parsed (missing hash).
///
/// # Panics
///
/// This function does not panic.
///
/// # Examples
///
/// ```rust
/// # use std::path::PathBuf;
/// # use librawdist::checksum::parse_checksums;
/// let input = "abcdef  file.txt\n123456  other.bin\n";
/// let map = parse_checksums(input).unwrap();
/// assert_eq!(map.get(&PathBuf::from("file.txt")), Some(&"abcdef".to_string()));
/// ```
pub fn parse_checksums(content: &str) -> Result<BTreeMap<PathBuf, String>, RawdistError> {
    let mut map = BTreeMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Primary parsing strategy: locate the explicit double‑space separator.
        // This is the format produced by `sha256sum` and our own `format_checksums`.
        if let Some(pos) = line.find("  ") {
            let hash = &line[..pos];
            // Remove any leading/trailing whitespace from the path portion.
            let path = line[pos + 2..].trim();
            map.insert(PathBuf::from(path), hash.to_string());
        } else {
            // Fallback: split on any whitespace. This handles malformed lines gracefully,
            // treating the first token as the hash and the rest (if any) as the path.
            let mut parts = line.split_whitespace();
            let hash = parts
                .next()
                .ok_or_else(|| RawdistError::Config(format!("Invalid checksum line: {}", line)))?;
            // If no path token follows, default to an empty string.
            // This trade‑off ensures we do not lose the hash value for forensic purposes.
            let path = parts.next().unwrap_or("");
            map.insert(PathBuf::from(path), hash.to_string());
        }
    }
    Ok(map)
}
