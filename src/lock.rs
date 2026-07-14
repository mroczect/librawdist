use crate::error::RawdistError;
use crate::fs::FileSystem;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A single entry in the lock file, recording the exact version and integrity
/// of an installed package.
///
/// Lock entries are the foundation of reproducible builds: they pin a package
/// to a specific version, source location, and SHA‑256 checksum. This prevents
/// accidental upgrades and ensures that every installation of the same
/// lock file yields identical content.
///
/// # Fields
///
/// * `name` – The canonical package name.
/// * `version` – The exact semantic version string.
/// * `source` – The URL or local path from which the package was obtained.
/// * `checksum` – The SHA‑256 hash of the `.rawdist` archive.
///
/// # Examples
///
/// ```rust
/// use librawdist::lock::LockEntry;
///
/// let entry = LockEntry {
///     name: "my-theme".into(),
///     version: "1.2.0".into(),
///     source: "https://example.com/my-theme-1.2.0.rawdist".into(),
///     checksum: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890".into(),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEntry {
    pub name: String,
    pub version: String,
    pub source: String,
    pub checksum: String,
}

/// Represents the complete lock file (`Rawdist.lock`) for the project.
///
/// The lock file is a TOML‑formatted manifest that records every directly
/// installed package with its resolved metadata. It guarantees deterministic
/// re‑installation across different machines and CI environments.
///
/// An empty `LockFile` (the [`Default`] value) is returned when no lock file
/// exists yet, allowing new projects to bootstrap without manual file
/// creation.
///
/// # Examples
///
/// ```rust
/// use librawdist::lock::{LockFile, LockEntry};
///
/// let mut lock = LockFile::default();
/// lock.add_package(
///     "my-theme",
///     "1.0.0",
///     "https://example.com/my-theme.rawdist",
///     "abc123...",
/// );
/// assert_eq!(lock.packages.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockFile {
    pub packages: Vec<LockEntry>,
}

impl LockFile {
    /// Loads the lock file from disk, returning a default empty instance if the
    /// file does not exist.
    ///
    /// This behaviour simplifies first‑time use: callers do not need to check
    /// for existence or manually create the file. If the file exists but
    /// contains invalid TOML or cannot be read, an appropriate
    /// [`RawdistError`] is returned.
    ///
    /// # Arguments
    ///
    /// * `fs` – The [`FileSystem`] implementation used to read the file.
    /// * `path` – The path to the lock file (typically `Rawdist.lock`).
    ///
    /// # Returns
    ///
    /// * `Ok(LockFile)` – The parsed lock file, or an empty one if the file is
    ///   absent.
    /// * `Err(RawdistError)` – On I/O errors or TOML parsing failures. The
    ///   error variant will be [`RawdistError::TomlParse`] with the file path
    ///   and the underlying `toml::de::Error`.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use librawdist::lock::LockFile;
    /// use librawdist::fs::RealFs;
    /// use std::path::Path;
    ///
    /// let fs = RealFs;
    /// let lock = LockFile::load(&fs, Path::new("Rawdist.lock"))
    ///     .expect("Failed to load lock file");
    /// ```
    pub fn load(fs: &dyn FileSystem, path: &Path) -> Result<Self, RawdistError> {
        // If the lock file doesn't exist, return a default (empty) lock file.
        // This is an intentional design decision: the lock file is optional
        // until the first package is installed, and we want to avoid forcing
        // users to run a separate `init` command.
        if !fs.exists(path) {
            return Ok(Self::default());
        }
        let content = fs.read_to_string(path)?;
        let lockfile: LockFile = toml::from_str(&content).map_err(|e| RawdistError::TomlParse {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(lockfile)
    }

    /// Serializes the lock file and writes it to disk in pretty‑printed TOML.
    ///
    /// The file is written atomically (as far as the underlying
    /// [`FileSystem::write`] implementation guarantees) and will be created
    /// with all necessary parent directories.
    ///
    /// # Arguments
    ///
    /// * `fs` – The [`FileSystem`] implementation used to write the file.
    /// * `path` – The destination path for the lock file.
    ///
    /// # Returns
    ///
    /// * `Ok(())` – The lock file was successfully written.
    /// * `Err(RawdistError)` – If TOML serialization fails or the write
    ///   operation encounters an I/O error.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use librawdist::lock::LockFile;
    /// use librawdist::fs::RealFs;
    /// use std::path::Path;
    ///
    /// let fs = RealFs;
    /// let lock = LockFile::default();
    /// lock.save(&fs, Path::new("Rawdist.lock"))
    ///     .expect("Failed to save lock file");
    /// ```
    pub fn save(&self, fs: &dyn FileSystem, path: &Path) -> Result<(), RawdistError> {
        let content =
            toml::to_string_pretty(self).map_err(|e| RawdistError::Config(e.to_string()))?;
        fs.write(path, content.as_bytes())?;
        Ok(())
    }

    /// Adds or updates a package entry in the lock file (upsert).
    ///
    /// If an entry with the same `name` already exists, it is removed before
    /// the new entry is inserted. This ensures that the lock file never
    /// contains duplicates for a given package name, making upgrades and
    /// re‑installations straightforward.
    ///
    /// # Arguments
    ///
    /// * `name` – The package name.
    /// * `version` – The exact version string.
    /// * `source` – The URL or path to the archive.
    /// * `checksum` – The SHA‑256 checksum of the archive.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use librawdist::lock::LockFile;
    ///
    /// let mut lock = LockFile::default();
    /// lock.add_package("core", "2.0.0", "https://example.com/core.rawdist", "abc");
    /// lock.add_package("core", "2.0.1", "https://example.com/core-v2.0.1.rawdist", "def");
    /// assert_eq!(lock.packages.len(), 1);
    /// assert_eq!(lock.packages[0].version, "2.0.1");
    /// ```
    pub fn add_package(&mut self, name: &str, version: &str, source: &str, checksum: &str) {
        // Remove any existing entry with the same name to maintain the
        // invariant: at most one entry per package name. Then push the new
        // one so that the lock file always reflects the latest requested
        // state.
        self.packages.retain(|p| p.name != name);
        self.packages.push(LockEntry {
            name: name.to_string(),
            version: version.to_string(),
            source: source.to_string(),
            checksum: checksum.to_string(),
        });
    }
}
