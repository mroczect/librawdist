use crate::error::RawdistError;
use crate::fs::FileSystem;
use std::path::{Path, PathBuf};

pub trait HttpClient {
    fn get(&self, url: &str) -> Result<Vec<u8>, RawdistError>;
}

pub struct UreqClient;

impl HttpClient for UreqClient {
    fn get(&self, url: &str) -> Result<Vec<u8>, RawdistError> {
        let response = ureq::get(url)
            .call()
            .map_err(|e| RawdistError::Network(e.to_string()))?;
        response
            .into_body()
            .read_to_vec()
            .map_err(|e| RawdistError::Network(e.to_string()))
    }
}

pub fn fetch_package(
    fs: &dyn FileSystem,
    client: &dyn HttpClient,
    url: &str,
    dest_path: Option<&Path>,
) -> Result<PathBuf, RawdistError> {
    let body = client.get(url)?;

    let dest = if let Some(p) = dest_path {
        p.to_path_buf()
    } else {
        let mut cache = dirs_next::cache_dir().unwrap_or_else(|| PathBuf::from("."));
        cache.push("librawdist");
        cache.push("cache");
        fs.create_dir_all(&cache)?;
        let filename = url.split('/').last().unwrap_or("package.rawdist");
        cache.join(filename)
    };

    fs.write(&dest, &body)?;
    log::info!("Fetched package to {}", dest.display());
    Ok(dest)
}
