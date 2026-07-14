use crate::error::RawdistError;
use std::io::Read;
use std::path::{Path, PathBuf};

pub fn fetch_package(url: &str, dest_path: Option<&Path>) -> Result<PathBuf, RawdistError> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| RawdistError::Network(e.to_string()))?;
    let mut body = Vec::new();
    response.into_body().read_to_end(&mut body)?;

    let dest = if let Some(p) = dest_path {
        p.to_path_buf()
    } else {
        let mut cache = dirs_next::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        cache.push("librawdist");
        cache.push("cache");
        std::fs::create_dir_all(&cache)?;
        let filename = url.split('/').last().unwrap_or("package.rawdist");
        cache.join(filename)
    };

    std::fs::write(&dest, &body)?;
    log::info!("Fetched package to {}", dest.display());
    Ok(dest)
}
