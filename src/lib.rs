//! The `librawdist` crate provides a robust package manager library for the
//! [rawssg](https://mroczect.github.io/rawssg/) ecosystem.
//!
//! It offers building, verifying, fetching, installing, and removing packages
//! in a safe and reproducible manner. All core abstractions — file systems,
//! HTTP clients, and configuration — are defined as traits, enabling
//! dependency injection for testing and custom backends.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use librawdist::{PackageManager, RealFs, UreqClient};
//! use std::path::PathBuf;
//!
//! let fs = RealFs;
//! let http = UreqClient;
//! let manager = PackageManager::new(
//!     &fs,
//!     &http,
//!     PathBuf::from("rawssg-packages.toml"),
//!     PathBuf::from("Rawdist.lock"),
//! );
//!
//! // List installed packages
//! let manifest = manager.list().unwrap();
//! assert!(manifest.packages.is_empty());
//! ```
//!
//! # Crate Features
//!
//! This crate does not expose any feature flags. All functionality is
//! available by default.
//!
//! # Crate Organisation
//!
//! | Module        | Purpose |
//! |---------------|---------|
//! | [`checksum`]  | SHA‑256 hashing and checksum file generation/parsing. |
//! | [`error`]     | The central [`RawdistError`] type. |
//! | [`fetch`]     | HTTP client trait and default `ureq` implementation; downloading packages. |
//! | [`filter`]    | Glob pattern filtering for file inclusion/exclusion. |
//! | [`fs`]        | File system abstraction trait and real file system implementation. |
//! | [`install`]   | High‑level installation and removal logic. |
//! | [`lock`]      | Lock file data structures and I/O. |
//! | [`manager`]   | The [`PackageManager`] orchestrator. |
//! | [`manifest`]  | Manifest (installed package list) loading and saving. |
//! | [`package`]   | Archive creation, extraction, and verification. |
//! | [`types`]     | Configuration and manifest types. |
//! | [`verify`]    | Standalone archive integrity verification. |

// ---------------------------------------------------------------------------
// Public module declarations
// ---------------------------------------------------------------------------

/// File checksum generation and verification utilities.
///
/// This module provides functions to hash individual files, generate a
/// manifest of checksums for a directory tree (respecting include/exclude
/// patterns), and format/parse the standard `checksums.sha256` file used
/// inside `.rawdist` archives.
pub mod checksum;

/// The central error type for the entire library.
///
/// Contains the [`RawdistError`] enum which implements [`Error`], [`Debug`],
/// and [`Diagnostic`] (from `miette`), enabling rich error reporting with
/// error codes and help texts.
pub mod error;

/// HTTP client abstraction and default implementation.
///
/// Defines the [`HttpClient`] trait for performing GET requests and provides
/// [`UreqClient`] as the production backend using the `ureq` crate. Also
/// includes the [`fetch_package`] function for downloading `.rawdist` archives
/// to the local cache.
pub mod fetch;

/// File inclusion/exclusion filtering using glob patterns.
///
/// Exposes the [`is_included`](filter::is_included) function that determines
/// whether a relative file path should be included in a package based on
/// user‑supplied include and exclude patterns.
pub mod filter;

/// File system abstraction trait and real implementation.
///
/// The [`FileSystem`] trait defines all I/O operations required by the
/// library, allowing in‑memory or test implementations to be substituted.
/// [`RealFs`] delegates directly to [`std::fs`] and [`walkdir`].
pub mod fs;

/// Package installation and removal logic.
///
/// Contains the [`install_package`] and [`remove_package`] functions that
/// validate, extract, move, and update the manifest when installing or
/// uninstalling a package.
pub mod install;

/// Lock file management for reproducible installs.
///
/// Provides the [`LockFile`] struct and its [`LockEntry`] components, along
/// with methods for loading, saving, and adding packages.
pub mod lock;

/// High‑level package manager orchestrator.
///
/// The [`PackageManager`] struct brings together a file system backend, an
/// HTTP client, and the paths to the manifest and lock file. It offers
/// convenient methods for install, uninstall, verify, list, and create.
pub mod manager;

/// Manifest I/O helpers.
///
/// Contains [`load_manifest`] and [`save_manifest`] functions that read and
/// write the project's installed‑package list (a TOML file).
pub mod manifest;

/// Archive creation and extraction.
///
/// Implements [`create_package`], [`extract_to_temp`], and [`move_extracted`].
/// Archive integrity (size limits, path traversal prevention, checksum
/// verification) is handled here.
pub mod package;

/// Configuration and manifest data types.
///
/// Defines [`RawdistConfig`], [`PackageMeta`], [`RawssgReqs`],
/// [`FilePatterns`], [`InstallConfig`], [`InstalledPackage`], and
/// [`Manifest`]. These types are serializable/deserializable with `serde` and
/// are the backbone of the package specification.
pub mod types;

/// Standalone package verification utility.
///
/// Provides the [`verify_package`] function that extracts a `.rawdist`
/// archive, checks all checksums, and optionally retains the extracted
/// content for inspection.
pub mod verify;

// ---------------------------------------------------------------------------
// Convenience re-exports at the crate root
// ---------------------------------------------------------------------------

/// Re-export of the central error type.
///
/// This brings [`error::RawdistError`] into the crate root so that users can
/// write `librawdist::RawdistError` instead of the longer path.
pub use error::RawdistError;

/// Re-exports of the HTTP client trait and default implementation, plus the
/// `fetch_package` function.
///
/// - [`HttpClient`](fetch::HttpClient): asynchronous GET request trait.
/// - [`UreqClient`](fetch::UreqClient): production implementation using `ureq`.
/// - [`fetch_package`](fetch::fetch_package): downloads an archive from a URL.
pub use fetch::{HttpClient, UreqClient, fetch_package};

/// Re-exports of the file system abstraction and its default real
/// implementation.
///
/// - [`FileSystem`](fs::FileSystem): trait for all I/O operations.
/// - [`RealFs`](fs::RealFs): concrete implementation delegating to `std::fs`.
pub use fs::{FileSystem, RealFs};

/// Re-exports of the installation and removal functions.
///
/// - [`install_package`](install::install_package): installs a `.rawdist`
///   archive.
/// - [`remove_package`](install::remove_package): removes an installed
///   package.
pub use install::{install_package, remove_package};

/// Re-export of the lock file data structure.
///
/// [`LockFile`](lock::LockFile) represents the contents of `Rawdist.lock`.
pub use lock::LockFile;

/// Re-export of the package manager.
///
/// [`PackageManager`](manager::PackageManager) is the main entry point for
/// most operations.
pub use manager::PackageManager;

/// Re-exports of the manifest I/O functions.
///
/// - [`load_manifest`](manifest::load_manifest): reads the manifest TOML.
/// - [`save_manifest`](manifest::save_manifest): writes it back.
pub use manifest::{load_manifest, save_manifest};

/// Re-exports of the archive creation and extraction functions.
///
/// - [`create_package`](package::create_package): builds a `.rawdist` archive.
/// - [`extract_to_temp`](package::extract_to_temp): extracts and verifies an
///   archive to a temporary directory.
/// - [`move_extracted`](package::move_extracted): moves an extracted directory
///   to its final location.
pub use package::{create_package, extract_to_temp, move_extracted};

/// Re-exports of the main configuration and manifest types.
///
/// - [`RawdistConfig`](types::RawdistConfig): the package configuration
///   (parsed from `rawdist.conf`).
/// - [`Manifest`](types::Manifest): the list of installed packages.
pub use types::{Manifest, RawdistConfig};

/// Re-export of the standalone verification function.
///
/// [`verify_package`](verify::verify_package) checks the integrity of a
/// `.rawdist` archive without installing it.
pub use verify::verify_package;
