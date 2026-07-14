use crate::error::RawdistError;
use crate::fetch::HttpClient;
use crate::fs::FileSystem;
use crate::lock::LockFile;
use crate::manifest;
use crate::types::Manifest;
use crate::{
    RawdistConfig, create_package, fetch_package, install_package, remove_package, verify_package,
};
use std::path::{Path, PathBuf};

/// High‑level orchestrator for package lifecycle operations.
///
/// `PackageManager` holds references to a [`FileSystem`] and an
/// [`HttpClient`], along with the paths to the manifest and lock file.
/// It provides a unified API for installing, uninstalling, verifying,
/// listing, and creating packages while keeping the manifest and lock
/// file in sync automatically.
///
/// # Generic Parameters
///
/// * `'a` – The lifetime of the borrowed references to the file system
///   and HTTP client. Both references are shared (`&'a`), so `'a` is
///   covariant in the struct. The struct cannot outlive the borrowed
///   backends.
/// * `F` – A concrete type implementing [`FileSystem`]. This allows
///   production use with [`RealFs`](crate::fs::RealFs) and testing with
///   mock file systems.
/// * `H` – A concrete type implementing [`HttpClient`]. Typically
///   [`UreqClient`](crate::fetch::UreqClient) for production.
///
/// # Examples
///
/// ```rust,no_run
/// use librawdist::PackageManager;
/// use librawdist::fs::RealFs;
/// use librawdist::fetch::UreqClient;
/// use std::path::PathBuf;
///
/// let fs = RealFs;
/// let http = UreqClient;
/// let manager = PackageManager::new(
///     &fs,
///     &http,
///     PathBuf::from("rawssg-packages.toml"),
///     PathBuf::from("Rawdist.lock"),
/// );
///
/// // List installed packages.
/// let manifest = manager.list().expect("failed to read manifest");
/// println!("Installed: {:?}", manifest.packages);
/// ```
pub struct PackageManager<'a, F: FileSystem, H: HttpClient> {
    /// The file system abstraction used for all I/O operations.
    pub fs: &'a F,
    /// The HTTP client abstraction for downloading packages.
    pub http: &'a H,
    /// Path to the TOML manifest file that tracks installed packages.
    pub manifest_path: PathBuf,
    /// Path to the lock file that pins exact package versions and checksums.
    pub lockfile_path: PathBuf,
}

impl<'a, F: FileSystem, H: HttpClient> PackageManager<'a, F, H> {
    /// Constructs a new `PackageManager` with the required backends and file
    /// paths.
    ///
    /// # Arguments
    ///
    /// * `fs` – A reference to a [`FileSystem`] implementation.
    /// * `http` – A reference to an [`HttpClient`] implementation.
    /// * `manifest_path` – The path to the manifest file (e.g.,
    ///   `rawssg-packages.toml`). It will be created if missing during
    ///   package operations.
    /// * `lockfile_path` – The path to the lock file (e.g., `Rawdist.lock`).
    ///   It will be created on first use.
    ///
    /// # Returns
    ///
    /// A new `PackageManager` instance with the given configuration.
    ///
    /// # Panics
    ///
    /// This constructor does not panic.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use librawdist::PackageManager;
    /// use librawdist::fs::RealFs;
    /// use librawdist::fetch::UreqClient;
    /// use std::path::PathBuf;
    ///
    /// let manager = PackageManager::new(
    ///     &RealFs,
    ///     &UreqClient,
    ///     PathBuf::from("manifest.toml"),
    ///     PathBuf::from("lock.toml"),
    /// );
    /// ```
    pub fn new(fs: &'a F, http: &'a H, manifest_path: PathBuf, lockfile_path: PathBuf) -> Self {
        Self {
            fs,
            http,
            manifest_path,
            lockfile_path,
        }
    }

    /// Installs a package from a local `.rawdist` archive.
    ///
    /// This is a convenience wrapper around [`install_package`] that also
    /// synchronises the lock file after a successful installation. The lock
    /// file is kept in sync with the manifest: any package in the lock file
    /// that is no longer present in the manifest is automatically pruned.
    ///
    /// # Arguments
    ///
    /// * `archive_path` – Path to the `.rawdist` archive.
    /// * `target_override` – If `Some`, install into this directory instead
    ///   of the one declared in the package’s configuration.
    ///
    /// # Returns
    ///
    /// * `Ok(())` – Installation and lock file update succeeded.
    /// * `Err(RawdistError)` – On any failure during installation or lock
    ///   file handling.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use librawdist::PackageManager;
    /// # use librawdist::fs::RealFs;
    /// # use librawdist::fetch::UreqClient;
    /// # use std::path::{Path, PathBuf};
    /// let manager = PackageManager::new(&RealFs, &UreqClient,
    ///     PathBuf::from("manifest.toml"), PathBuf::from("lock.toml"));
    /// manager.install(Path::new("my-theme.rawdist"), None)
    ///     .expect("Install failed");
    /// ```
    pub fn install(
        &self,
        archive_path: &Path,
        target_override: Option<&Path>,
    ) -> Result<(), RawdistError> {
        install_package(self.fs, archive_path, target_override, &self.manifest_path)?;
        // After altering the manifest, immediately refresh the lock file
        // to maintain the invariant that the lock file is a strict subset
        // of the manifest (only recorded packages with known checksums).
        self.update_lockfile_from_manifest()?;
        Ok(())
    }

    /// Downloads a package from a URL and then installs it.
    ///
    /// The archive is first fetched into the local cache using
    /// [`fetch_package`]; then the local installation logic is invoked via
    /// [`Self::install`]. This two‑step process ensures that network
    /// failures do not leave a partially installed package.
    ///
    /// # Arguments
    ///
    /// * `url` – The URL of the `.rawdist` archive to download.
    /// * `target_override` – Optional installation directory override,
    ///   forwarded to [`Self::install`].
    ///
    /// # Returns
    ///
    /// * `Ok(())` – The package was downloaded and installed successfully.
    /// * `Err(RawdistError)` – On network errors, I/O errors, or
    ///   installation failures.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use librawdist::PackageManager;
    /// # use librawdist::fs::RealFs;
    /// # use librawdist::fetch::UreqClient;
    /// # use std::path::PathBuf;
    /// let manager = PackageManager::new(&RealFs, &UreqClient,
    ///     PathBuf::from("manifest.toml"), PathBuf::from("lock.toml"));
    /// manager.install_from_url("https://example.com/my-theme.rawdist", None)
    ///     .expect("Install from URL failed");
    /// ```
    pub fn install_from_url(
        &self,
        url: &str,
        target_override: Option<&Path>,
    ) -> Result<(), RawdistError> {
        // Fetch the archive to the local cache first. If this fails,
        // no modification is made to the manifest or installed packages.
        let downloaded = fetch_package(self.fs, self.http, url, None)?;
        // Delegate to the local install path; this will also update the
        // lock file.
        self.install(&downloaded, target_override)
    }

    /// Removes an installed package by name.
    ///
    /// The package’s installation directory is deleted (if it exists),
    /// its entry is removed from the manifest, and the lock file is pruned
    /// to reflect the change. If the directory is already missing, only
    /// the manifest and lock file entries are cleaned up.
    ///
    /// # Arguments
    ///
    /// * `package_name` – The name of the package to uninstall, as it
    ///   appears in the manifest.
    ///
    /// # Returns
    ///
    /// * `Ok(())` – The package was successfully removed.
    /// * `Err(RawdistError::NotInstalled)` – If the package name is not
    ///   found in the manifest.
    /// * `Err(RawdistError)` – On any I/O or lock file error.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use librawdist::PackageManager;
    /// # use librawdist::fs::RealFs;
    /// # use librawdist::fetch::UreqClient;
    /// # use std::path::PathBuf;
    /// let manager = PackageManager::new(&RealFs, &UreqClient,
    ///     PathBuf::from("manifest.toml"), PathBuf::from("lock.toml"));
    /// manager.uninstall("my-theme")
    ///     .expect("Uninstall failed");
    /// ```
    pub fn uninstall(&self, package_name: &str) -> Result<(), RawdistError> {
        remove_package(self.fs, package_name, &self.manifest_path)?;
        // Keep the lock file in sync after manifest modification.
        self.update_lockfile_from_manifest()?;
        Ok(())
    }

    /// Verifies the integrity of a `.rawdist` archive.
    ///
    /// The archive is extracted to a temporary directory and all checksums
    /// are validated against the embedded `checksums.sha256` file. The
    /// temporary directory is normally deleted after verification, but can
    /// be retained for inspection by setting `keep_temp` to `true`.
    ///
    /// # Arguments
    ///
    /// * `archive_path` – Path to the `.rawdist` archive to verify.
    /// * `keep_temp` – If `true`, the extracted directory is preserved and
    ///   its path is returned.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(PathBuf))` – If `keep_temp` is `true`, the path to the
    ///   kept extracted directory.
    /// * `Ok(None)` – Verification succeeded and the temporary directory
    ///   was cleaned up.
    /// * `Err(RawdistError)` – If the archive is missing, corrupted, or
    ///   contains a checksum mismatch.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use librawdist::PackageManager;
    /// # use librawdist::fs::RealFs;
    /// # use librawdist::fetch::UreqClient;
    /// # use std::path::{Path, PathBuf};
    /// let manager = PackageManager::new(&RealFs, &UreqClient,
    ///     PathBuf::from("manifest.toml"), PathBuf::from("lock.toml"));
    /// let result = manager.verify(Path::new("pkg.rawdist"), false)
    ///     .expect("Verification failed");
    /// assert!(result.is_none());
    /// ```
    pub fn verify(
        &self,
        archive_path: &Path,
        keep_temp: bool,
    ) -> Result<Option<PathBuf>, RawdistError> {
        // verify_package handles the full extraction, checksum comparison,
        // and optional cleanup. No manifest or lock file interaction is
        // needed here.
        verify_package(self.fs, archive_path, keep_temp)
    }

    /// Lists all currently installed packages by loading the manifest.
    ///
    /// # Returns
    ///
    /// * `Ok(Manifest)` – The parsed manifest. If the manifest file does
    ///   not exist, an empty default manifest is returned.
    /// * `Err(RawdistError)` – On I/O errors or TOML parse failures.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use librawdist::PackageManager;
    /// # use librawdist::fs::RealFs;
    /// # use librawdist::fetch::UreqClient;
    /// # use std::path::PathBuf;
    /// let manager = PackageManager::new(&RealFs, &UreqClient,
    ///     PathBuf::from("manifest.toml"), PathBuf::from("lock.toml"));
    /// let manifest = manager.list().expect("failed to list");
    /// for pkg in manifest.packages {
    ///     println!("{} {}", pkg.name, pkg.version);
    /// }
    /// ```
    pub fn list(&self) -> Result<Manifest, RawdistError> {
        manifest::load_manifest(self.fs, &self.manifest_path)
    }

    /// Creates a new `.rawdist` package from a source directory.
    ///
    /// This is a thin wrapper around [`create_package`] that uses the
    /// manager’s file system backend. No manifest or lock file
    /// modifications are performed.
    ///
    /// # Arguments
    ///
    /// * `src_dir` – The source directory containing `rawdist.conf` and
    ///   the files to package.
    /// * `output_path` – Where to write the resulting `.rawdist` archive.
    /// * `config` – The validated [`RawdistConfig`] describing the package
    ///   metadata and file patterns.
    ///
    /// # Returns
    ///
    /// * `Ok(())` – The package was created and written to `output_path`.
    /// * `Err(RawdistError)` – If file gathering, checksum generation,
    ///   compression, or I/O fails.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use librawdist::PackageManager;
    /// # use librawdist::fs::RealFs;
    /// # use librawdist::fetch::UreqClient;
    /// # use librawdist::types::RawdistConfig;
    /// # use std::path::{Path, PathBuf};
    /// let manager = PackageManager::new(&RealFs, &UreqClient,
    ///     PathBuf::from("manifest.toml"), PathBuf::from("lock.toml"));
    /// let config = RawdistConfig::load_from_dir(&RealFs, Path::new("./source"))
    ///     .expect("config load failed");
    /// manager.create(Path::new("./source"), Path::new("out.rawdist"), &config)
    ///     .expect("creation failed");
    /// ```
    pub fn create(
        &self,
        src_dir: &Path,
        output_path: &Path,
        config: &RawdistConfig,
    ) -> Result<(), RawdistError> {
        create_package(self.fs, src_dir, output_path, config)
    }

    /// Synchronises the lock file so that it contains only the packages
    /// currently present in the manifest.
    ///
    /// This method loads the manifest and the existing lock file, removes
    /// any lock entries whose package name is no longer in the manifest,
    /// and then writes the updated lock file back. It is called
    /// automatically after every install and uninstall to maintain the
    /// invariant that the lock file is a subset of the manifest.
    ///
    /// # Returns
    ///
    /// * `Ok(())` – The lock file was successfully pruned and saved.
    /// * `Err(RawdistError)` – If loading the manifest, reading/writing
    ///   the lock file, or TOML serialization fails.
    fn update_lockfile_from_manifest(&self) -> Result<(), RawdistError> {
        let manifest = manifest::load_manifest(self.fs, &self.manifest_path)?;
        let mut lock = LockFile::load(self.fs, &self.lockfile_path)?;
        // Keep only those lock entries that still correspond to a
        // package in the manifest. This prunes stale entries left over
        // after uninstalls, or entries for packages that were removed
        // externally.
        lock.packages
            .retain(|entry| manifest.packages.iter().any(|p| p.name == entry.name));
        lock.save(self.fs, &self.lockfile_path)?;
        Ok(())
    }
}
