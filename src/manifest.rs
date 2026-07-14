use crate::error::LibrawdistError;
use crate::types::Manifest;
use std::path::Path;
use std::io::Write;
use fs2::FileExt;

/// Load the manifest from a given path, creating a default if it doesn't exist.
/// Uses a shared file lock to ensure consistency.
pub fn load_manifest(path: &Path) -> Result<Manifest, LibrawdistError> {
    if !path.exists() {
        return Ok(Manifest::default());
    }
    let file = std::fs::OpenOptions::new().read(true).open(path)?;
    file.lock_shared()?;
    let content = std::fs::read_to_string(path)?;
    let manifest: Manifest = toml::from_str(&content).map_err(|e| LibrawdistError::TomlParse {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok(manifest)
}

/// Save the manifest to the given path atomically (write to temp, rename).
/// Uses an exclusive lock.
pub fn save_manifest(path: &Path, manifest: &Manifest) -> Result<(), LibrawdistError> {
    let tmp_path = path.with_extension("tmp");
    {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)?;
        file.lock_exclusive()?;
        let content = toml::to_string_pretty(manifest)
            .map_err(|e| LibrawdistError::Config(e.to_string()))?;
        let mut f = file;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}
