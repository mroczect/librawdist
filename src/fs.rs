use std::io;
use std::path::{Path, PathBuf};

/// Abstraction over file system operations used throughout the library.
///
/// This trait enables dependency injection for unit testing and allows
/// alternative backends (e.g., in‑memory virtual file systems) to be
/// substituted without modifying business logic. Every method corresponds
/// closely to a standard library function, returning [`io::Result`] so that
/// I/O errors propagate naturally. Implementations must uphold the same
/// semantics as their `std::fs` counterparts unless otherwise noted.
///
/// # Examples
///
/// ```rust
/// use librawdist::fs::{FileSystem, RealFs};
/// use std::path::Path;
///
/// let fs = RealFs;
/// fs.create_dir_all(Path::new("/tmp/test_fs"))
///     .expect("Failed to create test directory");
/// assert!(fs.exists(Path::new("/tmp/test_fs")));
/// ```
pub trait FileSystem {
    /// Reads the entire contents of a file into a [`String`].
    ///
    /// # Arguments
    ///
    /// * `path` – The path of the file to read.
    ///
    /// # Returns
    ///
    /// * `Ok(String)` – The file contents as a UTF‑8 string.
    /// * `Err(io::Error)` – If the file does not exist, cannot be read, or
    ///   contains invalid UTF‑8.
    ///
    /// # Panics
    ///
    /// This method should not panic; implementations are expected to return
    /// errors.
    fn read_to_string(&self, path: &Path) -> io::Result<String>;

    /// Reads the entire contents of a file as a raw byte vector.
    ///
    /// # Arguments
    ///
    /// * `path` – The path of the file to read.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` – The file contents.
    /// * `Err(io::Error)` – If the file cannot be opened or read.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Writes a byte slice to a file, creating parent directories if
    /// necessary.
    ///
    /// # Arguments
    ///
    /// * `path` – The path where the data will be written.
    /// * `content` – The data to write.
    ///
    /// # Returns
    ///
    /// * `Ok(())` – On successful write.
    /// * `Err(io::Error)` – If any parent directory cannot be created or the
    ///   write fails.
    fn write(&self, path: &Path, content: &[u8]) -> io::Result<()>;

    /// Recursively creates a directory and all missing parents.
    ///
    /// # Arguments
    ///
    /// * `path` – The directory path to create.
    ///
    /// # Returns
    ///
    /// * `Ok(())` – On success, even if the directory already exists.
    /// * `Err(io::Error)` – If a parent cannot be created due to permissions or
    ///   other I/O errors.
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Removes a directory and all of its contents.
    ///
    /// # Arguments
    ///
    /// * `path` – The directory to remove.
    ///
    /// # Returns
    ///
    /// * `Ok(())` – On successful removal.
    /// * `Err(io::Error)` – If the directory does not exist or cannot be
    ///   deleted.
    fn remove_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Removes a single file.
    ///
    /// # Arguments
    ///
    /// * `path` – The file to remove.
    ///
    /// # Returns
    ///
    /// * `Ok(())` – On success.
    /// * `Err(io::Error)` – If the file does not exist or permissions are
    ///   insufficient.
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Returns `true` if the path exists on the file system.
    ///
    /// # Arguments
    ///
    /// * `path` – The path to check.
    fn exists(&self, path: &Path) -> bool;

    /// Returns `true` if the path is a directory.
    ///
    /// # Arguments
    ///
    /// * `path` – The path to inspect.
    fn is_dir(&self, path: &Path) -> bool;

    /// Returns `true` if the path is a regular file.
    ///
    /// # Arguments
    ///
    /// * `path` – The path to inspect.
    fn is_file(&self, path: &Path) -> bool;

    /// Returns the immediate children (files and directories) of a directory.
    ///
    /// # Arguments
    ///
    /// * `path` – The directory to list.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PathBuf>)` – A vector of child paths.
    /// * `Err(io::Error)` – If the directory cannot be read.
    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>>;

    /// Copies a file from `from` to `to`, returning the number of bytes
    /// copied.
    ///
    /// # Arguments
    ///
    /// * `from` – Source file path.
    /// * `to` – Destination file path.
    ///
    /// # Returns
    ///
    /// * `Ok(u64)` – The number of bytes copied.
    /// * `Err(io::Error)` – On copy failure.
    fn copy_file(&self, from: &Path, to: &Path) -> io::Result<u64>;

    /// Renames (moves) a file or directory.
    ///
    /// This operation is atomic on most modern file systems when both paths
    /// reside on the same mount point.
    ///
    /// # Arguments
    ///
    /// * `from` – The current path.
    /// * `to` – The new path.
    ///
    /// # Returns
    ///
    /// * `Ok(())` – On success.
    /// * `Err(io::Error)` – If the rename fails.
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Returns the canonical, absolute form of a path with all intermediate
    /// components normalized and symbolic links resolved.
    ///
    /// This function acts as a security boundary when validating paths against
    /// directory roots to prevent path traversal attacks.
    ///
    /// # Arguments
    ///
    /// * `path` – The path to canonicalize.
    ///
    /// # Returns
    ///
    /// * `Ok(PathBuf)` – The canonical path.
    /// * `Err(io::Error)` – If the path does not exist or resolution fails.
    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf>;

    /// Recursively walks a directory tree, returning all regular file paths.
    ///
    /// Symbolic links are followed by default (the `walkdir` crate
    /// configuration). Only plain files are collected; directories themselves
    /// are omitted from the result.
    ///
    /// # Arguments
    ///
    /// * `root` – The directory to walk.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<PathBuf>)` – A list of all regular files found under `root`.
    /// * `Err(io::Error)` – If the traversal itself fails (e.g., permission
    ///   denied on the root).
    fn walk_dir(&self, root: &Path) -> io::Result<Vec<PathBuf>>;

    /// Retrieves file metadata, used primarily for checking the size of an
    /// archive before decompression.
    ///
    /// # Arguments
    ///
    /// * `path` – The file or directory to query.
    ///
    /// # Returns
    ///
    /// * `Ok(std::fs::Metadata)` – The file metadata.
    /// * `Err(io::Error)` – If the path does not exist or cannot be accessed.
    fn metadata(&self, path: &Path) -> io::Result<std::fs::Metadata>;
}

/// The real file system implementation that delegates directly to
/// [`std::fs`] and [`walkdir`].
///
/// All methods simply forward to the corresponding standard library function,
/// with one exception: [`write`](RealFs::write) automatically creates parent
/// directories before writing, mirroring common convenience behaviour.
///
/// This is the default backend used in production builds of the library.
///
/// # Example
///
/// ```rust
/// use librawdist::fs::RealFs;
///
/// let fs = RealFs;
/// ```
pub struct RealFs;

impl FileSystem for RealFs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn write(&self, path: &Path, content: &[u8]) -> io::Result<()> {
        // Automatically create parent directories to avoid errors when
        // writing to a nested location that doesn't yet exist. This is a
        // deliberate divergence from `std::fs::write` alone, which would
        // return an error if the parent is missing.
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn remove_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }

    fn exists(&self, path: &Path) -> bool {
        // Directly call the `Path::exists` convenience method.
        path.exists()
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<PathBuf>> {
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(path)? {
            // Collect only the path portion; the full DirEntry is
            // discarded because downstream code only needs the
            // location.
            entries.push(entry?.path());
        }
        Ok(entries)
    }

    fn copy_file(&self, from: &Path, to: &Path) -> io::Result<u64> {
        std::fs::copy(from, to)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        path.canonicalize()
    }

    fn walk_dir(&self, root: &Path) -> io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        // `walkdir::WalkDir` is used instead of manual recursion because
        // it efficiently handles deep trees, symlink cycles (configurable),
        // and errors on a per‑entry basis, making it the most robust
        // choice for a package manager that processes arbitrary user
        // content.
        for entry in walkdir::WalkDir::new(root) {
            let entry = entry?;
            // Only regular files are relevant for packaging; directories
            // are implicitly represented by the file paths.
            if entry.file_type().is_file() {
                files.push(entry.into_path());
            }
        }
        Ok(files)
    }

    fn metadata(&self, path: &Path) -> io::Result<std::fs::Metadata> {
        path.metadata()
    }
}
