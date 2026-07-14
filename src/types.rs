use crate::error::RawdistError;
use crate::fs::FileSystem;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The current configuration edition supported by this version of the library.
///
/// The edition string is embedded in every [`RawdistConfig`] and acts as a
/// version marker for the configuration format. The library refuses to process
/// a config whose edition does not match this constant, ensuring forward
/// compatibility checks. Ref: [`RawdistConfig::validate`].
///
/// # Examples
///
/// ```rust
/// use librawdist::types::CURRENT_EDITION;
/// assert_eq!(CURRENT_EDITION, "1");
/// ```
pub const CURRENT_EDITION: &str = "1";

/// The root configuration structure for a rawssg package.
///
/// This struct is parsed from the `rawdist.conf` file found in a package source
/// directory or inside a `.rawdist` archive. It contains all metadata needed
/// to describe, validate, install, and maintain the package.
///
/// # Serialization
///
/// The `edition` field is serialized with a default of `CURRENT_EDITION` via
/// the `default_edition` helper, so older config files that lack the field
/// remain compatible.
///
/// # Fields
///
/// - `edition`: Configuration format version. Must equal [`CURRENT_EDITION`].
/// - `package`: Package identity and metadata (name, version, authors, etc.).
/// - `rawssg`: Requirements specific to the `rawssg` ecosystem (type,
///   version constraints).
/// - `files`: Include/exclude glob patterns for packaging files.
/// - `install`: Installation directives (target directory, merge behaviour).
/// - `metadata`: Arbitrary key-value table for custom extensions.
///
/// # Examples
///
/// ```rust,no_run
/// use librawdist::types::RawdistConfig;
/// use librawdist::fs::RealFs;
/// use std::path::Path;
///
/// let fs = RealFs;
/// let config = RawdistConfig::load_from_dir(&fs, Path::new("./my_package"))
///     .expect("Invalid config");
/// println!("Package {} v{}", config.package.name, config.package.version);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawdistConfig {
    /// Configuration format edition. Defaults to `CURRENT_EDITION` when absent.
    #[serde(default = "default_edition")]
    pub edition: String,
    /// Package identity and metadata.
    pub package: PackageMeta,
    /// Requirements for the `rawssg` ecosystem.
    pub rawssg: RawssgReqs,
    /// File inclusion/exclusion patterns.
    pub files: FilePatterns,
    /// Installation behaviour.
    pub install: InstallConfig,
    /// Arbitrary additional metadata (e.g., custom keys for third-party tools).
    #[serde(default)]
    pub metadata: toml::value::Table,
}

/// Returns the current edition string as a [`String`].
///
/// This function exists solely to be used as the `#[serde(default = ...)]`
/// value for [`RawdistConfig::edition`], because serde requires a path to a
/// function.
fn default_edition() -> String {
    CURRENT_EDITION.to_string()
}

impl RawdistConfig {
    /// Constructs a new `RawdistConfig` with the given components and the
    /// current edition.
    ///
    /// The `metadata` field is initialised as an empty TOML table.
    ///
    /// # Arguments
    ///
    /// * `package` – The package metadata.
    /// * `rawssg` – The ecosystem requirements.
    /// * `files` – File patterns for packaging.
    /// * `install` – Installation configuration.
    ///
    /// # Returns
    ///
    /// A new `RawdistConfig` with `edition` set to [`CURRENT_EDITION`] and an
    /// empty metadata table.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use librawdist::types::{
    ///     RawdistConfig, PackageMeta, RawssgReqs, FilePatterns, InstallConfig,
    /// };
    ///
    /// let config = RawdistConfig::new(
    ///     PackageMeta {
    ///         name: "my-theme".into(),
    ///         version: "1.0.0".into(),
    ///         description: Some("A test theme".into()),
    ///         authors: None,
    ///         license: Some("MIT".into()),
    ///         repository: None,
    ///         homepage: None,
    ///         documentation: None,
    ///         keywords: None,
    ///         categories: None,
    ///         release_date: None,
    ///     },
    ///     RawssgReqs {
    ///         min_version: None,
    ///         max_version: None,
    ///         r#type: "theme".into(),
    ///     },
    ///     FilePatterns {
    ///         include: vec!["**/*.hbs".into()],
    ///         exclude: vec![],
    ///     },
    ///     InstallConfig {
    ///         target_dir: "themes/{{ package.name }}".into(),
    ///         merge_config: None,
    ///     },
    /// );
    /// assert_eq!(config.edition, "1");
    /// ```
    pub fn new(
        package: PackageMeta,
        rawssg: RawssgReqs,
        files: FilePatterns,
        install: InstallConfig,
    ) -> Self {
        Self {
            edition: default_edition(),
            package,
            rawssg,
            files,
            install,
            metadata: toml::value::Table::new(),
        }
    }

    /// Validates the configuration, returning an error for any rule violation.
    ///
    /// Checks performed:
    ///
    /// - Package name is non-empty and consists only of alphanumeric
    ///   characters, `-`, or `_`.
    /// - Package version is a valid semantic version (per [`semver::Version`]).
    /// - `install.target_dir` is a relative path that does not contain `..`.
    /// - At least one include pattern is specified.
    /// - `install.merge_config` (if present) is a safe relative path.
    /// - `edition` matches [`CURRENT_EDITION`].
    ///
    /// # Returns
    ///
    /// * `Ok(())` – Configuration is valid.
    /// * `Err(RawdistError::Validation(...))` – A human‑readable description
    ///   of the violation.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use librawdist::types::*;
    /// let mut config = RawdistConfig::new(
    ///     PackageMeta {
    ///         name: "bad/name".into(),
    ///         version: "0.1.0".into(),
    ///         description: None,
    ///         authors: None,
    ///         license: None,
    ///         repository: None,
    ///         homepage: None,
    ///         documentation: None,
    ///         keywords: None,
    ///         categories: None,
    ///         release_date: None,
    ///     },
    ///     RawssgReqs { min_version: None, max_version: None, r#type: "theme".into() },
    ///     FilePatterns { include: vec!["*.hbs".into()], exclude: vec![] },
    ///     InstallConfig { target_dir: "themes".into(), merge_config: None },
    /// );
    /// assert!(config.validate().is_err()); // bad name
    /// config.package.name = "valid-name".into();
    /// assert!(config.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), RawdistError> {
        // Ensure the package name is non-empty before checking its
        // character set, avoiding a trivially passing check on an empty
        // string.
        if self.package.name.is_empty() {
            return Err(RawdistError::Validation("package name is empty".into()));
        }
        // Restrict package names to a safe subset to prevent filesystem
        // injection, CLI argument issues, and overly complex paths.
        if !self
            .package
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(RawdistError::Validation(format!(
                "invalid package name '{}', only alphanumeric, '-' and '_' allowed",
                self.package.name
            )));
        }
        // Parse the version using the `semver` crate to catch invalid
        // formats early. The parsed value is discarded; only the error
        // matters.
        let _ = Version::parse(&self.package.version).map_err(|e| {
            RawdistError::Validation(format!("invalid version '{}': {}", self.package.version, e))
        })?;

        // target_dir must be relative to prevent installation outside the
        // intended root and must not contain `..` to block trivial path
        // traversal.
        if self.install.target_dir.starts_with('/') || self.install.target_dir.contains("..") {
            return Err(RawdistError::Validation(format!(
                "target_dir '{}' must be relative and cannot contain '..'",
                self.install.target_dir
            )));
        }

        // Require at least one include pattern; an empty list would result
        // in an empty package, which is almost certainly a configuration
        // mistake.
        if self.files.include.is_empty() {
            return Err(RawdistError::Validation(
                "no include patterns specified".into(),
            ));
        }

        // If a merge_config path is provided, it must obey the same safety
        // constraints as target_dir.
        if let Some(ref mc) = self.install.merge_config {
            if mc.starts_with('/') || mc.contains("..") {
                return Err(RawdistError::Validation(
                    "merge_config path must be relative and safe".into(),
                ));
            }
        }

        // Edition checking allows the library to reject configs from
        // future versions that it cannot handle.
        if self.edition != CURRENT_EDITION {
            return Err(RawdistError::Validation(format!(
                "unsupported edition '{}', expected '{}'",
                self.edition, CURRENT_EDITION
            )));
        }

        Ok(())
    }

    /// Loads and validates a `RawdistConfig` from a source directory.
    ///
    /// This is the primary entry point for reading a package's configuration.
    /// It expects a file named `rawdist.conf` in the given directory, parses
    /// it as TOML, and runs all validation rules defined in [`validate`]
    /// before returning.
    ///
    /// # Arguments
    ///
    /// * `fs` – The [`FileSystem`] implementation to use for file existence
    ///   check and reading.
    /// * `dir` – The directory containing `rawdist.conf`.
    ///
    /// # Returns
    ///
    /// * `Ok(RawdistConfig)` – The parsed and validated configuration.
    /// * `Err(RawdistError::MissingFile)` – If `rawdist.conf` does not exist.
    /// * `Err(RawdistError::TomlParse)` – If the file contains invalid TOML.
    /// * `Err(RawdistError::Validation(...))` – If any validation rule fails.
    ///
    /// # Panics
    ///
    /// This method does not panic.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use librawdist::types::RawdistConfig;
    /// use librawdist::fs::RealFs;
    /// use std::path::Path;
    ///
    /// let fs = RealFs;
    /// let config = RawdistConfig::load_from_dir(&fs, Path::new("./src"))
    ///     .expect("Failed to load rawdist.conf");
    /// ```
    pub fn load_from_dir(fs: &dyn FileSystem, dir: &Path) -> Result<Self, RawdistError> {
        let config_path = dir.join("rawdist.conf");
        // Explicitly check for existence to give a clear, early error
        // rather than a generic I/O error from read_to_string.
        if !fs.exists(&config_path) {
            return Err(RawdistError::MissingFile { path: config_path });
        }
        let content = fs.read_to_string(&config_path)?;
        let config: Self = toml::from_str(&content).map_err(|e| RawdistError::TomlParse {
            path: config_path,
            source: e,
        })?;
        // Validate immediately after parsing to ensure the loaded config
        // is always in a consistent state.
        config.validate()?;
        Ok(config)
    }

    /// Resolves the installation target directory by substituting template
    /// variables.
    ///
    /// The string `{{ package.name }}` is replaced with the package name,
    /// and `{{ package.version }}` with the version. No other substitutions
    /// are performed. If a template variable is missing (e.g.,
    /// `{{ unknown }}`), it is left unchanged.
    ///
    /// # Returns
    ///
    /// A new [`String`] with all known placeholders replaced.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use librawdist::types::*;
    /// let config = RawdistConfig::new(
    ///     PackageMeta {
    ///         name: "my-theme".into(),
    ///         version: "2.0.0".into(),
    ///         description: None,
    ///         authors: None,
    ///         license: None,
    ///         repository: None,
    ///         homepage: None,
    ///         documentation: None,
    ///         keywords: None,
    ///         categories: None,
    ///         release_date: None,
    ///     },
    ///     RawssgReqs { min_version: None, max_version: None, r#type: "theme".into() },
    ///     FilePatterns { include: vec!["*.hbs".into()], exclude: vec![] },
    ///     InstallConfig {
    ///         target_dir: "themes/{{ package.name }}-{{ package.version }}".into(),
    ///         merge_config: None,
    ///     },
    /// );
    /// assert_eq!(config.resolve_target_dir(), "themes/my-theme-2.0.0");
    /// ```
    pub fn resolve_target_dir(&self) -> String {
        // Simple string replacements are sufficient because the values
        // have already passed validation and contain no special characters
        // that would break path semantics.
        self.install
            .target_dir
            .replace("{{ package.name }}", &self.package.name)
            .replace("{{ package.version }}", &self.package.version)
    }
}

/// Metadata describing a package's identity and optional attributes.
///
/// All fields except `name` and `version` are optional and may be omitted in
/// the `rawdist.conf`. This struct is serialized as part of the package
/// configuration.
///
/// # Fields
///
/// - `name`: The canonical package name (alphanumeric, `-`, `_`).
/// - `version`: A semantic version string (e.g., `1.2.3`).
/// - `description`: A short summary of the package.
/// - `authors`: A list of author names.
/// - `license`: The SPDX license identifier or custom license string.
/// - `repository`: URL to the source repository.
/// - `homepage`: URL to the project homepage.
/// - `documentation`: URL to online documentation.
/// - `keywords`: List of keywords for categorization.
/// - `categories`: List of categories (e.g., `["theme", "plugin"]`).
/// - `release_date`: ISO‑8601 date string of the release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMeta {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub authors: Option<Vec<String>>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub release_date: Option<String>,
}

/// Requirements imposed on the `rawssg` ecosystem version.
///
/// This information is stored in the package and can be used by a resolver to
/// ensure compatibility with the installed `rawssg` toolchain.
///
/// # Fields
///
/// - `min_version`: Minimum compatible `rawssg` version (inclusive).
/// - `max_version`: Maximum compatible `rawssg` version (inclusive).
/// - `r#type`: The package type (e.g., `"theme"`, `"plugin"`). The `r#`
///   prefix is required because `type` is a Rust keyword.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawssgReqs {
    pub min_version: Option<String>,
    pub max_version: Option<String>,
    pub r#type: String,
}

/// File inclusion and exclusion patterns for packaging.
///
/// The `include` list contains glob patterns that files must match to be
/// packed. The `exclude` list overrides `include`: any file matching an
/// exclusion pattern is omitted even if it matches an inclusion pattern.
/// Patterns follow the `glob` crate syntax (e.g., `**/*.html`, `assets/*.css`).
///
/// # Fields
///
/// - `include`: Required list of inclusion patterns.
/// - `exclude`: Optional list of exclusion patterns, defaults to empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePatterns {
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// Installation instructions for a package.
///
/// Determines where the package files are placed on the target system and how
/// the package's configuration may be merged with an existing setup.
///
/// # Fields
///
/// - `target_dir`: A relative path (may contain `{{ package.name }}` and
///   `{{ package.version }}` placeholders) where the package content will be
///   installed.
/// - `merge_config`: Optional relative path to a configuration file within the
///   package that can be merged into a project-level config during
///   installation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallConfig {
    pub target_dir: String,
    #[serde(default)]
    pub merge_config: Option<String>,
}

/// A record of an installed package, stored in the manifest.
///
/// This struct represents a fully installed package and its location on disk.
/// It is serialized as part of the project's manifest file.
///
/// # Fields
///
/// - `name`: Package name.
/// - `version`: Installed version string.
/// - `install_path`: Absolute path to the installed package directory.
/// - `config_merged`: If the package had a `merge_config`, the path (relative
///   to the project root) of the merged configuration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub install_path: PathBuf,
    pub config_merged: Option<String>,
}

/// The project manifest listing all installed packages.
///
/// This structure is serialized as the `rawssg-packages.toml` file and serves
/// as the source of truth for which packages are currently installed and where
/// they reside.
///
/// # Fields
///
/// - `packages`: A list of [`InstalledPackage`] entries. Defaults to an empty
///   vector when the manifest is absent or empty.
///
/// # Examples
///
/// ```rust
/// use librawdist::types::Manifest;
///
/// let manifest = Manifest::default();
/// assert!(manifest.packages.is_empty());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    #[serde(default)]
    pub packages: Vec<InstalledPackage>,
}
