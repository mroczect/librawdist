use crate::RawdistError;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawdistConfig {
    pub package: PackageMeta,
    pub rawssg: RawssgReqs,
    pub files: FilePatterns,
    pub install: InstallConfig,
    #[serde(default)]
    pub metadata: toml::value::Table,
}

impl RawdistConfig {
    pub fn new(
        package: PackageMeta,
        rawssg: RawssgReqs,
        files: FilePatterns,
        install: InstallConfig,
    ) -> Self {
        Self {
            package,
            rawssg,
            files,
            install,
            metadata: toml::value::Table::new(),
        }
    }

    pub fn validate(&self) -> Result<(), crate::error::RawdistError> {
        use crate::error::RawdistError;

        if self.package.name.is_empty() {
            return Err(RawdistError::Validation("package name is empty".into()));
        }
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

        let _ = Version::parse(&self.package.version).map_err(|e| {
            RawdistError::Validation(format!("invalid version '{}': {}", self.package.version, e))
        })?;

        if self.install.target_dir.starts_with('/') || self.install.target_dir.contains("..") {
            return Err(RawdistError::Validation(format!(
                "target_dir '{}' must be relative and cannot contain '..'",
                self.install.target_dir
            )));
        }

        if self.files.include.is_empty() {
            return Err(RawdistError::Validation(
                "no include patterns specified".into(),
            ));
        }

        if let Some(ref mc) = self.install.merge_config {
            if mc.starts_with('/') || mc.contains("..") {
                return Err(RawdistError::Validation(
                    "merge_config path must be relative and safe".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn load_from_dir(fs: &dyn crate::fs::FileSystem, dir: &std::path::Path) -> Result<Self, RawdistError> {
        let config_path = dir.join("rawdist.conf");
        if !fs.exists(&config_path) {
            return Err(RawdistError::MissingFile { path: config_path });
        }
        let content = fs.read_to_string(&config_path)?;
        let config: Self = toml::from_str(&content).map_err(|e| RawdistError::TomlParse {
            path: config_path,
            source: e,
        })?;
        config.validate()?;
        Ok(config)
    }
}

// PackageMeta tidak berubah
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawssgReqs {
    pub min_version: Option<String>,
    pub max_version: Option<String>,
    pub r#type: String,   // wajib diisi, tidak ada default
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilePatterns {
    pub include: Vec<String>,   // wajib, tidak ada default
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallConfig {
    pub target_dir: String,     // wajib
    #[serde(default)]
    pub merge_config: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub install_path: PathBuf,
    pub config_merged: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    #[serde(default)]
    pub packages: Vec<InstalledPackage>,
}
