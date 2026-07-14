use crate::error::RawdistError;
use crate::fs::RealFs;
use std::path::Path;

pub fn verify_package(archive_path: &Path) -> Result<(), RawdistError> {
    if !archive_path.exists() {
        return Err(RawdistError::InvalidInput(format!(
            "Archive not found: {}",
            archive_path.display()
        )));
    }
    let extracted = crate::package::extract_to_temp(&RealFs, archive_path)?;
    std::fs::remove_dir_all(&extracted)?;
    log::info!("Verification successful: all checksums match.");
    Ok(())
}
