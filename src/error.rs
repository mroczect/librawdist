use miette::Diagnostic;
use thiserror::Error;
use std::path::PathBuf;

#[derive(Error, Debug, Diagnostic)]
pub enum LibrawdistError {
    /// I/O error from the underlying filesystem.
    #[error("I/O error")]
    #[diagnostic(code(Librawdist::io))]
    Io(#[from] std::io::Error),

    /// Configuration parsing or value error.
    #[error("Configuration error: {0}")]
    #[diagnostic(code(Librawdist::config))]
    Config(String),

    /// Failed to parse TOML at a specific path.
    #[error("Failed to parse TOML in {path}")]
    #[diagnostic(code(Librawdist::toml_parse))]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    /// Checksum verification failed for a file.
    #[error("Checksum verification failed for file {path}")]
    #[diagnostic(
        code(Librawdist::checksum_mismatch),
        help("The package may be corrupted or tampered. Download again from official source.")
    )]
    ChecksumMismatch { path: PathBuf },

    /// Required file is missing from the archive or directory.
    #[error("Missing required file {path}")]
    #[diagnostic(code(Librawdist::missing_file))]
    MissingFile { path: PathBuf },

    /// Manifest file error.
    #[error("Manifest error: {0}")]
    #[diagnostic(code(Librawdist::manifest))]
    Manifest(String),

    /// Package not found in manifest during removal.
    #[error("Package not found in manifest: {0}")]
    #[diagnostic(code(Librawdist::not_installed))]
    NotInstalled(String),

    /// Error from walking directories.
    #[error(transparent)]
    WalkDir(#[from] walkdir::Error),

    /// Invalid input from the caller.
    #[error("Invalid input: {0}")]
    #[diagnostic(code(Librawdist::invalid_input))]
    InvalidInput(String),

    /// Package validation failed.
    #[error("Package validation failed: {0}")]
    #[diagnostic(code(Librawdist::validation))]
    Validation(String),

    /// Version requirement not satisfied.
    #[error("Package {name} requires rawssg version {required} but current is {current}")]
    #[diagnostic(code(Librawdist::version_mismatch))]
    VersionMismatch {
        name: String,
        required: String,
        current: String,
    },
    #[error("Path traversal attempt detected: {0}")]
    #[diagnostic(code(Librawdist::path_traversal))]
    PathTraversal(PathBuf),
}
