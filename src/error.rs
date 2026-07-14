use miette::Diagnostic;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
pub enum RawdistError {
    #[error("I/O error")]
    #[diagnostic(code(librawdist::io))]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    #[diagnostic(code(librawdist::config))]
    Config(String),

    #[error("Failed to parse TOML in {path}")]
    #[diagnostic(code(librawdist::toml_parse))]
    TomlParse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("Checksum verification failed for file {path}")]
    #[diagnostic(
        code(librawdist::checksum_mismatch),
        help("The package may be corrupted or tampered. Download again from official source.")
    )]
    ChecksumMismatch { path: PathBuf },

    #[error("Missing required file {path}")]
    #[diagnostic(code(librawdist::missing_file))]
    MissingFile { path: PathBuf },

    #[error("Manifest error: {0}")]
    #[diagnostic(code(librawdist::manifest))]
    Manifest(String),

    #[error("Package not found in manifest: {0}")]
    #[diagnostic(code(librawdist::not_installed))]
    NotInstalled(String),

    #[error(transparent)]
    WalkDir(#[from] walkdir::Error),

    #[error("Invalid input: {0}")]
    #[diagnostic(code(librawdist::invalid_input))]
    InvalidInput(String),

    #[error("Package validation failed: {0}")]
    #[diagnostic(code(librawdist::validation))]
    Validation(String),

    #[error("Version requirement mismatch for {name}: requires {required}, current is {current}")]
    #[diagnostic(code(librawdist::version_mismatch))]
    VersionMismatch {
        name: String,
        required: String,
        current: String,
    },

    #[error("Path traversal attempt detected: {0}")]
    #[diagnostic(code(librawdist::path_traversal))]
    PathTraversal(PathBuf),

    #[error("Network error: {0}")]
    #[diagnostic(code(librawdist::network))]
    Network(String),
}
