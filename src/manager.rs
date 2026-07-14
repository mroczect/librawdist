use crate::error::RawdistError;
use crate::fetch::HttpClient;
use crate::fs::FileSystem;
use crate::lock::LockFile;
use crate::manifest;
use crate::types::Manifest;
use crate::{
    RawdistConfig, create_package, fetch_package, install_package, remove_package, verify_package,
};
use std::path::{Path, PathBuf};

pub struct PackageManager<'a, F: FileSystem, H: HttpClient> {
    pub fs: &'a F,
    pub http: &'a H,
    pub manifest_path: PathBuf,
    pub lockfile_path: PathBuf,
}

impl<'a, F: FileSystem, H: HttpClient> PackageManager<'a, F, H> {
    pub fn new(fs: &'a F, http: &'a H, manifest_path: PathBuf, lockfile_path: PathBuf) -> Self {
        Self {
            fs,
            http,
            manifest_path,
            lockfile_path,
        }
    }

    pub fn install(
        &self,
        archive_path: &Path,
        target_override: Option<&Path>,
    ) -> Result<(), RawdistError> {
        install_package(self.fs, archive_path, target_override, &self.manifest_path)?;
        self.update_lockfile_from_manifest()?;
        Ok(())
    }

    pub fn install_from_url(
        &self,
        url: &str,
        target_override: Option<&Path>,
    ) -> Result<(), RawdistError> {
        let downloaded = fetch_package(self.fs, self.http, url, None)?;
        self.install(&downloaded, target_override)
    }

    pub fn uninstall(&self, package_name: &str) -> Result<(), RawdistError> {
        remove_package(self.fs, package_name, &self.manifest_path)?;
        self.update_lockfile_from_manifest()?;
        Ok(())
    }

    pub fn verify(
        &self,
        archive_path: &Path,
        keep_temp: bool,
    ) -> Result<Option<PathBuf>, RawdistError> {
        verify_package(self.fs, archive_path, keep_temp)
    }

    pub fn list(&self) -> Result<Manifest, RawdistError> {
        manifest::load_manifest(self.fs, &self.manifest_path)
    }

    pub fn create(
        &self,
        src_dir: &Path,
        output_path: &Path,
        config: &RawdistConfig,
    ) -> Result<(), RawdistError> {
        create_package(self.fs, src_dir, output_path, config)
    }

    fn update_lockfile_from_manifest(&self) -> Result<(), RawdistError> {
        let manifest = manifest::load_manifest(self.fs, &self.manifest_path)?;
        let mut lock = LockFile::load(self.fs, &self.lockfile_path)?;
        lock.packages
            .retain(|entry| manifest.packages.iter().any(|p| p.name == entry.name));
        lock.save(self.fs, &self.lockfile_path)?;
        Ok(())
    }
}
