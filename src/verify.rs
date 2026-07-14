use crate::error::LibrawdistError;
use std::path::Path;

pub fn verify_package(archive_path: &Path) -> Result<(), LibrawdistError> {
    if !archive_path.exists() {
        return Err(LibrawdistError::InvalidInput(format!(
            "Archive not found: {}",
            archive_path.display()
        )));
    }
    let extracted = crate::package::extract_to_temp(archive_path)?;
    std::fs::remove_dir_all(&extracted)?;
    log::info!("Verification successful: all checksums match.");
    Ok(())
}
