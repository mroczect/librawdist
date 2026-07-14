use crate::error::RawdistError;
use crate::fs::FileSystem;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockEntry {
    pub name: String,
    pub version: String,
    pub source: String,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LockFile {
    pub packages: Vec<LockEntry>,
}

impl LockFile {
    pub fn load(fs: &dyn FileSystem, path: &Path) -> Result<Self, RawdistError> {
        if !fs.exists(path) {
            return Ok(Self::default());
        }
        let content = fs.read_to_string(path)?;
        let lockfile: LockFile = toml::from_str(&content).map_err(|e| RawdistError::TomlParse {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(lockfile)
    }

    pub fn save(&self, fs: &dyn FileSystem, path: &Path) -> Result<(), RawdistError> {
        let content =
            toml::to_string_pretty(self).map_err(|e| RawdistError::Config(e.to_string()))?;
        fs.write(path, content.as_bytes())?;
        Ok(())
    }

    pub fn add_package(&mut self, name: &str, version: &str, source: &str, checksum: &str) {
        self.packages.retain(|p| p.name != name);
        self.packages.push(LockEntry {
            name: name.to_string(),
            version: version.to_string(),
            source: source.to_string(),
            checksum: checksum.to_string(),
        });
    }
}
