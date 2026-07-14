use crate::error::RawdistError;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::io::Write;
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
    pub fn load(path: &Path) -> Result<Self, RawdistError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let file = std::fs::OpenOptions::new().read(true).open(path)?;
        file.lock_shared()?;
        let content = std::fs::read_to_string(path)?;
        let lockfile: LockFile = toml::from_str(&content).map_err(|e| RawdistError::TomlParse {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(lockfile)
    }

    pub fn save(&self, path: &Path) -> Result<(), RawdistError> {
        let tmp_path = path.with_extension("tmp");
        {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)?;
            file.lock_exclusive()?;
            let content =
                toml::to_string_pretty(self).map_err(|e| RawdistError::Config(e.to_string()))?;
            let mut f = file;
            f.write_all(content.as_bytes())?;
            f.sync_all()?;
        }
        std::fs::rename(&tmp_path, path)?;
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
