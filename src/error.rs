use miette::Diagnostic;
use std::path::PathBuf;
use thiserror::Error;

/// Central error type for the [`librawdist`](crate) package manager.
///
/// Every fallible operation in the library returns [`Result<_, RawdistError>`].
/// The enum implements [`Error`], [`Debug`], and [`Diagnostic`] to seamlessly
/// integrate with the `miette` reporting framework. Several variants support
/// automatic conversion from common third‑party error types (e.g.,
/// [`std::io::Error`], [`walkdir::Error`]) through the `#[from]` attribute,
/// enabling concise usage of the `?` operator.
///
/// # Examples
///
/// ```rust
/// use librawdist::RawdistError;
/// use std::path::PathBuf;
///
/// let err = RawdistError::MissingFile {
///     path: PathBuf::from("config.toml"),
/// };
/// assert_eq!(
///     err.to_string(),
///     "Missing required file config.toml"
/// );
/// ```
#[derive(Error, Debug, Diagnostic)]
pub enum RawdistError {
    /// An I/O error occurred while performing a file system operation.
    ///
    /// This variant transparently wraps a [`std::io::Error`] and is typically
    /// created by applying the `?` operator to `std::io::Result` values.
    #[error("I/O error")]
    #[diagnostic(code(librawdist::io))]
    Io(#[from] std::io::Error),

    /// A configuration error containing a human‑readable description.
    ///
    /// Used when a configuration value is missing, malformed, or semantically
    /// invalid.
    #[error("Configuration error: {0}")]
    #[diagnostic(code(librawdist::config))]
    Config(String),

    /// An error encountered while parsing a TOML file.
    ///
    /// The `path` field identifies the file that failed to parse, and the
    /// `source` field carries the underlying [`toml::de::Error`] for precise
    /// diagnostics. Ref: TOML crate documentation.
    #[error("Failed to parse TOML in {path}")]
    #[diagnostic(code(librawdist::toml_parse))]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    /// A checksum verification failure for a specific file.
    ///
    /// This error signals that the content of the given file does not match its
    /// expected SHA‑256 hash, indicating possible corruption or tampering.
    #[error("Checksum verification failed for file {path}")]
    #[diagnostic(
        code(librawdist::checksum_mismatch),
        help("The package may be corrupted or tampered. Download again from official source.")
    )]
    ChecksumMismatch { path: PathBuf },

    /// A required file is missing from the package or extraction.
    ///
    /// The `path` field holds the expected location of the missing file.
    #[error("Missing required file {path}")]
    #[diagnostic(code(librawdist::missing_file))]
    MissingFile { path: PathBuf },

    /// An error related to the manifest (the list of installed packages).
    ///
    /// The message describes the specific manifest problem.
    #[error("Manifest error: {0}")]
    #[diagnostic(code(librawdist::manifest))]
    Manifest(String),

    /// The requested package is not installed according to the manifest.
    ///
    /// The string contains the package name that was not found.
    #[error("Package not found in manifest: {0}")]
    #[diagnostic(code(librawdist::not_installed))]
    NotInstalled(String),

    /// An error emitted by the `walkdir` crate while traversing a directory.
    ///
    /// This variant transparently wraps a [`walkdir::Error`] and is
    /// automatically converted with `?`.
    #[error(transparent)]
    WalkDir(#[from] walkdir::Error),

    /// An invalid input value was supplied to an operation.
    ///
    /// The string describes why the input is considered invalid.
    #[error("Invalid input: {0}")]
    #[diagnostic(code(librawdist::invalid_input))]
    InvalidInput(String),

    /// Validation of a package or its configuration failed.
    ///
    /// The message details the validation rule that was violated (e.g., empty
    /// package name, illegal version, unsafe path).
    #[error("Package validation failed: {0}")]
    #[diagnostic(code(librawdist::validation))]
    Validation(String),

    /// A version requirement mismatch detected during installation or conflict
    /// resolution.
    ///
    /// The fields indicate the package name, the required version (from a
    /// dependency specification), and the currently installed version.
    #[error("Version requirement mismatch for {name}: requires {required}, current is {current}")]
    #[diagnostic(code(librawdist::version_mismatch))]
    VersionMismatch {
        name: String,
        required: String,
        current: String,
    },

    /// A potential path traversal attack was detected.
    ///
    /// The contained [`PathBuf`] is the offending path that attempts to escape
    /// the intended directory root. Operations are aborted immediately to
    /// protect the host file system.
    #[error("Path traversal attempt detected: {0}")]
    #[diagnostic(code(librawdist::path_traversal))]
    PathTraversal(PathBuf),

    /// A network‑related error occurred (e.g., HTTP request failure).
    ///
    /// The string contains the error description from the networking layer.
    #[error("Network error: {0}")]
    #[diagnostic(code(librawdist::network))]
    Network(String),

    /// An archive exceeded the maximum allowed size.
    ///
    /// The `size` field is the actual size in bytes, and `max` is the
    /// configured limit. This prevents decompression bombs.
    #[error("Archive is too large: {size} bytes exceeds maximum {max} bytes")]
    #[diagnostic(code(librawdist::archive_too_large))]
    ArchiveTooLarge { size: u64, max: u64 },
}
