use crate::checksum;
use crate::error::RawdistError;
use crate::fs::FileSystem;
use crate::types::RawdistConfig;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};

pub fn create_package(
    fs: &dyn FileSystem,
    src_dir: &Path,
    output_path: &Path,
    config: &RawdistConfig,
) -> Result<(), RawdistError> {
    log::info!("Packing directory: {}", src_dir.display());

    let checksums = checksum::generate_checksums(fs, src_dir, &config.files)?;
    let checksum_content = checksum::format_checksums(&checksums);
    log::debug!("Including {} files", checksums.len());

    let file = std::fs::File::create(output_path)?;
    let enc = GzEncoder::new(file, Compression::default());
    let mut tar = Builder::new(enc);

    for (rel, _) in &checksums {
        let full = src_dir.join(rel);
        tar.append_path_with_name(&full, rel)?;
    }

    let mut header = tar::Header::new_gnu();
    header.set_path("checksums.sha256")?;
    header.set_size(checksum_content.len() as u64);
    header.set_cksum();
    tar.append_data(&mut header, "checksums.sha256", checksum_content.as_bytes())?;

    let enc = tar.into_inner()?;
    enc.finish()?;
    log::info!("Package created: {}", output_path.display());
    Ok(())
}

pub fn extract_to_temp(fs: &dyn FileSystem, archive_path: &Path) -> Result<PathBuf, RawdistError> {
    let temp_dir = tempfile::tempdir()?;
    let dest = temp_dir.path().to_path_buf();

    let file = std::fs::File::open(archive_path)?;
    let dec = GzDecoder::new(file);
    let mut archive = Archive::new(dec);

    let dest_canonical = dest.canonicalize()?;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_path = entry.path()?.to_path_buf();

        if entry_path
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(RawdistError::PathTraversal(entry_path));
        }

        let joined = dest.join(&entry_path);
        if let Some(parent) = joined.parent() {
            std::fs::create_dir_all(parent)?;
            let parent_canon = parent.canonicalize()?;
            if !parent_canon.starts_with(&dest_canonical) {
                return Err(RawdistError::PathTraversal(entry_path));
            }
        }

        entry.unpack_in(&dest)?;
    }

    let checksum_file = dest.join("checksums.sha256");
    if !checksum_file.exists() {
        return Err(RawdistError::MissingFile {
            path: PathBuf::from("checksums.sha256"),
        });
    }
    let content = std::fs::read_to_string(&checksum_file)?;
    let expected = checksum::parse_checksums(&content)?;
    for (rel_path, expected_hash) in &expected {
        let actual_path = dest.join(rel_path);
        if !actual_path.exists() {
            return Err(RawdistError::MissingFile {
                path: rel_path.clone(),
            });
        }
        let actual_hash = checksum::hash_file(fs, &actual_path)?;
        if &actual_hash != expected_hash {
            return Err(RawdistError::ChecksumMismatch { path: actual_path });
        }
    }

    std::fs::remove_file(&checksum_file)?;
    let persistent = temp_dir.keep();
    Ok(persistent)
}

pub fn move_extracted(src: &Path, target: &Path) -> Result<(), RawdistError> {
    if target.exists() {
        return Err(RawdistError::Config(format!(
            "Target already exists: {}",
            target.display()
        )));
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(src, target)?;
    Ok(())
}
