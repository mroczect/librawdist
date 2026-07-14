use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use semver;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibrawdistConfig {
    pub package: PackageMeta,
    #[serde(default)]
    pub rawssg: RawssgReqs,
    #[serde(default)]
    pub files: FilePatterns,
    #[serde(default)]
    pub install: InstallConfig,
    #[serde(default)]
    pub metadata: toml::value::Table,
}

impl LibrawdistConfig {
    /// Validates the configuration, checking all fields for correctness and safety.
    pub fn validate(&self) -> Result<(), crate::error::LibrawdistError> {
        use crate::error::LibrawdistError;

        // Package name
        if self.package.name.is_empty() {
            return Err(LibrawdistError::Validation("package name is empty".into()));
        }
        if !self
            .package
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(LibrawdistError::Validation(format!(
                "invalid package name '{}', only alphanumeric, '-' and '_' allowed",
                self.package.name
            )));
        }
        // Version must be valid semver
        let _ = semver::Version::parse(&self.package.version).map_err(|e| {
            LibrawdistError::Validation(format!("invalid version '{}': {}", self.package.version, e))
        })?;
        // target_dir must be relative and not contain '..'
        if self.install.target_dir.starts_with('/') || self.install.target_dir.contains("..") {
            return Err(LibrawdistError::Validation(format!(
                "target_dir '{}' must be relative and cannot contain '..'",
                self.install.target_dir
            )));
        }
        // At least one include pattern
        if self.files.include.is_empty() {
            return Err(LibrawdistError::Validation(
                "no include patterns specified".into(),
            ));
        }
        // merge_config path also safe
        if let Some(ref mc) = self.install.merge_config {
            if mc.starts_with('/') || mc.contains("..") {
                return Err(LibrawdistError::Validation(
                    "merge_config path must be relative and safe".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Metadata about the package.
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

/// Requirements for rawssg compatibility.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawssgReqs {
    #[serde(default)]
    pub min_version: Option<String>,
    #[serde(default)]
    pub max_version: Option<String>,
    #[serde(default = "default_package_type")]
    pub r#type: String,
}

impl Default for RawssgReqs {
    fn default() -> Self {
        Self {
            min_version: None,
            max_version: None,
            r#type: default_package_type(),
        }
    }
}

fn default_package_type() -> String {
    "theme".to_string()
}

/// File inclusion/exclusion patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePatterns {
    #[serde(default = "default_include")]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

impl Default for FilePatterns {
    fn default() -> Self {
        Self {
            include: default_include(),
            exclude: Vec::new(),
        }
    }
}

fn default_include() -> Vec<String> {
    vec!["**/*".to_string()]
}

/// Installation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallConfig {
    #[serde(default = "default_target_dir")]
    pub target_dir: String,
    #[serde(default)]
    pub merge_config: Option<String>,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            target_dir: default_target_dir(),
            merge_config: None,
        }
    }
}

fn default_target_dir() -> String {
    "themes/{{ package.name }}".to_string()
}

/// An entry in the local manifest tracking installed packages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub install_path: PathBuf,
    pub config_merged: Option<String>,
}

/// The manifest file (`rawssg-packages.toml`) structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    #[serde(default)]
    pub packages: Vec<InstalledPackage>,
}
