use crate::error::RawdistError;
use crate::types::Manifest;
use fs2::FileExt;
use std::io::Write;
use std::path::Path;

pub fn load_manifest(path: &Path) -> Result<Manifest, RawdistError> {
    if !path.exists() {
        return Ok(Manifest::default());
    }
    let file = std::fs::OpenOptions::new().read(true).open(path)?;
    file.lock_shared()?;
    let content = std::fs::read_to_string(path)?;
    let manifest: Manifest = toml::from_str(&content).map_err(|e| RawdistError::TomlParse {
        path: path.to_path_buf(),
        source: e,
    })?;
    Ok(manifest)
}

pub fn save_manifest(path: &Path, manifest: &Manifest) -> Result<(), RawdistError> {
    let tmp_path = path.with_extension("tmp");
    {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)?;
        file.lock_exclusive()?;
        let content =
            toml::to_string_pretty(manifest).map_err(|e| RawdistError::Config(e.to_string()))?;
        let mut f = file;
        f.write_all(content.as_bytes())?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}
