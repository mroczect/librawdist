use crate::error::RawdistError;
use crate::fs::FileSystem;
use crate::types::Manifest;
use std::path::Path;

pub fn load_manifest(fs: &dyn FileSystem, path: &Path) -> Result<Manifest, RawdistError> {
    if !fs.exists(path) {
        return Ok(Manifest::default());
    }
    let content = fs.read_to_string(path)?;
    let manifest: Manifest = toml::from_str(&content).map_err(|e| RawdistError::TomlParse {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok(manifest)
}

pub fn save_manifest(fs: &dyn FileSystem, path: &Path, manifest: &Manifest) -> Result<(), RawdistError> {
    let content = toml::to_string_pretty(manifest)
        .map_err(|e| RawdistError::Config(e.to_string()))?;
    fs.write(path, content.as_bytes())?;
    Ok(())
}
