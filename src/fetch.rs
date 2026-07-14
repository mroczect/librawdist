use crate::error::RawdistError;
use crate::fs::FileSystem;
use std::path::{Path, PathBuf};

/// Abstract HTTP client for fetching package archives.
///
/// This trait decouples the package download mechanism from any concrete
/// implementation, enabling dependency injection for testing or alternative
/// backends. Implementations must return the raw bytes of the resource located
/// at `url` or a [`RawdistError`] describing a network or protocol failure.
///
/// # Examples
///
/// ```rust
/// use librawdist::fetch::HttpClient;
/// use librawdist::RawdistError;
///
/// struct MockClient;
/// impl HttpClient for MockClient {
///     fn get(&self, _url: &str) -> Result<Vec<u8>, RawdistError> {
///         Ok(b"fake archive data".to_vec())
///     }
/// }
/// ```
pub trait HttpClient {
    /// Perform an HTTP GET request and return the response body.
    ///
    /// # Arguments
    ///
    /// * `url` – The fully qualified URL of the resource to download.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` – The byte contents of the response.
    /// * `Err(RawdistError::Network(...))` – If the request fails, a
    ///   connection error occurs, or the response status indicates a
    ///   non‑success code (depending on the implementation).
    ///
    /// # Panics
    ///
    /// Implementations should not panic. The default [`UreqClient`] panics only
    /// on unrecoverable internal issues in `ureq` (e.g., TLS backend
    /// initialization), which are considered bugs in the dependency rather than
    /// user‑facing failures.
    fn get(&self, url: &str) -> Result<Vec<u8>, RawdistError>;
}

/// Concrete [`HttpClient`] backed by the `ureq` crate.
///
/// This is the default HTTP client used by the library when operating in a
/// production environment. It synchronously blocks the calling thread until the
/// request completes.
///
/// Ref: [`ureq` documentation](https://docs.rs/ureq) – HTTP client library.
pub struct UreqClient;

impl HttpClient for UreqClient {
    fn get(&self, url: &str) -> Result<Vec<u8>, RawdistError> {
        // Perform the HTTP GET. ureq::get returns a Result that is a union of
        // transport and HTTP-level errors; we convert any error into a
        // RawdistError::Network with the original message to preserve
        // diagnostics.
        let response = ureq::get(url)
            .call()
            .map_err(|e| RawdistError::Network(e.to_string()))?;

        // Read the entire response body into a Vec<u8>. Using into_body()
        // consumes the response, and read_to_vec() efficiently reads the
        // streaming body. Again, map any I/O error to Network.
        let body = response
            .into_body()
            .read_to_vec()
            .map_err(|e| RawdistError::Network(e.to_string()))?;
        Ok(body)
    }
}

/// Downloads a package archive and persists it to a local file.
///
/// If a destination path is explicitly provided via `dest_path`, the downloaded
/// bytes are written there directly. Otherwise, the archive is saved in a
/// platform‑specific cache directory (e.g., `$XDG_CACHE_HOME/librawdist/cache`
/// on Linux, `~/Library/Caches/librawdist/cache` on macOS, or
/// `%LOCALAPPDATA%\librawdist\cache` on Windows). A fallback to
/// `/tmp/librawdist-cache` is used when the system cache directory cannot be
/// determined.
///
/// The filename is derived from the last path segment of the URL; if the URL
/// does not contain a slash, the fallback name `"package.rawdist"` is used.
///
/// # Arguments
///
/// * `fs` – A [`FileSystem`] implementation used to create directories and
///   write the file.
/// * `client` – An [`HttpClient`] to retrieve the archive bytes.
/// * `url` – The URL of the archive to download.
/// * `dest_path` – Optional path where the archive will be written. When
///   `None`, the cache path is used.
///
/// # Returns
///
/// * `Ok(PathBuf)` – The final path of the saved archive file.
/// * `Err(RawdistError)` – If the HTTP request fails, the cache directory
///   cannot be created, or the file write fails.
///
/// # Panics
///
/// This function does not panic. The call to [`dirs_next::cache_dir()`] is
/// wrapped with `unwrap_or_else`, providing a safe fallback. The URL filename
/// extraction uses `unwrap_or`, which also supplies a default.
///
/// # Examples
///
/// ```rust
/// use librawdist::fetch::{fetch_package, HttpClient};
/// use librawdist::fs::RealFs;
/// use librawdist::RawdistError;
/// use std::path::Path;
///
/// // A mock client that returns predefined bytes instead of calling the network.
/// struct MockClient;
/// impl HttpClient for MockClient {
///     fn get(&self, _url: &str) -> Result<Vec<u8>, RawdistError> {
///         Ok(b"fake archive data".to_vec())
///     }
/// }
///
/// let fs = RealFs;
/// let client = MockClient;
/// let dest = fetch_package(&fs, &client, "https://example.com/pkg.rawdist", None)
///     .expect("Download should succeed");
/// assert!(dest.exists());
/// ```
pub fn fetch_package(
    fs: &dyn FileSystem,
    client: &dyn HttpClient,
    url: &str,
    dest_path: Option<&Path>,
) -> Result<PathBuf, RawdistError> {
    // Retrieve raw archive bytes.
    let body = client.get(url)?;

    // Decide where to store the file.
    let dest = if let Some(p) = dest_path {
        // Caller specified a concrete path; use it directly.
        p.to_path_buf()
    } else {
        // Resolve the platform cache directory. If the resolution fails
        // (e.g., missing HOME on Linux), fall back to a writable temporary
        // location to avoid crashing the process. This ensures the function
        // remains robust even in minimal container environments.
        let mut cache =
            dirs_next::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp/librawdist-cache"));
        cache.push("librawdist");
        cache.push("cache");
        fs.create_dir_all(&cache)?;

        // Extract a plausible filename from the URL's last path segment.
        // If the URL has no slash (e.g., "https://example.com"), default to
        // "package.rawdist" to avoid an empty filename.
        let filename = url.split('/').last().unwrap_or("package.rawdist");
        cache.join(filename)
    };

    // Persist the data atomically as far as the file system abstraction allows.
    fs.write(&dest, &body)?;
    log::info!("Fetched package to {}", dest.display());
    Ok(dest)
}
