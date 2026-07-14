use crate::error::RawdistError;
use crate::fs::FileSystem;
use std::path::Path;

pub fn verify_package(
    fs: &dyn FileSystem,
    archive_path: &Path,
    keep_temp: bool,
) -> Result<Option<std::path::PathBuf>, RawdistError> {
    if !fs.exists(archive_path) {
        return Err(RawdistError::InvalidInput(format!(
            "Archive not found: {}",
            archive_path.display()
        )));
    }
    let extracted = crate::package::extract_to_temp(fs, archive_path)?;
    if keep_temp {
        log::info!(
            "Verification successful. Extracted at {}",
            extracted.display()
        );
        Ok(Some(extracted))
    } else {
        fs.remove_dir_all(&extracted)?;
        log::info!("Verification successful: all checksums match.");
        Ok(None)
    }
}
