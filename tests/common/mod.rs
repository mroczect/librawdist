#![allow(dead_code)]

use librawdist::fs::FileSystem;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

pub struct MockFs {
    pub files: HashMap<PathBuf, Vec<u8>>,
    pub dirs: Vec<PathBuf>,
    pub read_error: Option<io::Error>,
    pub write_error: Option<io::Error>,
    pub walk_error: Option<io::Error>,
    pub create_dir_error: Option<io::Error>,
    pub remove_dir_error: Option<io::Error>,
    pub copy_error: Option<io::Error>,
    pub rename_error: Option<io::Error>,
    pub canonicalize_error: Option<io::Error>,
    pub read_dir_entries: Vec<PathBuf>,
    pub read_dir_error: Option<io::Error>,
}

impl MockFs {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            dirs: Vec::new(),
            read_error: None,
            write_error: None,
            walk_error: None,
            create_dir_error: None,
            remove_dir_error: None,
            copy_error: None,
            rename_error: None,
            canonicalize_error: None,
            read_dir_entries: vec![],
            read_dir_error: None,
        }
    }

    pub fn add_file(&mut self, path: &Path, content: &[u8]) {
        self.files.insert(path.to_path_buf(), content.to_vec());
    }
}

impl FileSystem for MockFs {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        if let Some(err) = &self.read_error {
            return Err(io::Error::new(err.kind(), err.to_string()));
        }
        self.read(path)
            .map(|v| String::from_utf8_lossy(&v).into_owned())
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        if let Some(err) = &self.read_error {
            return Err(io::Error::new(err.kind(), err.to_string()));
        }
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "mock file not found"))
    }

    fn write(&self, _path: &Path, _content: &[u8]) -> io::Result<()> {
        if let Some(err) = &self.write_error {
            Err(io::Error::new(err.kind(), err.to_string()))
        } else {
            Ok(())
        }
    }

    fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        if let Some(err) = &self.create_dir_error {
            Err(io::Error::new(err.kind(), err.to_string()))
        } else {
            Ok(())
        }
    }

    fn remove_dir_all(&self, _path: &Path) -> io::Result<()> {
        if let Some(err) = &self.remove_dir_error {
            Err(io::Error::new(err.kind(), err.to_string()))
        } else {
            Ok(())
        }
    }

    fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path) || self.dirs.contains(&path.to_path_buf())
    }

    fn is_dir(&self, path: &Path) -> bool {
        self.dirs.contains(&path.to_path_buf())
    }

    fn is_file(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }

    fn read_dir(&self, _path: &Path) -> io::Result<Vec<PathBuf>> {
        if let Some(err) = &self.read_dir_error {
            return Err(io::Error::new(err.kind(), err.to_string()));
        }
        Ok(self.read_dir_entries.clone())
    }

    fn copy_file(&self, _from: &Path, _to: &Path) -> io::Result<u64> {
        if let Some(err) = &self.copy_error {
            Err(io::Error::new(err.kind(), err.to_string()))
        } else {
            Ok(0)
        }
    }

    fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        if let Some(err) = &self.rename_error {
            Err(io::Error::new(err.kind(), err.to_string()))
        } else {
            Ok(())
        }
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        if let Some(err) = &self.canonicalize_error {
            Err(io::Error::new(err.kind(), err.to_string()))
        } else {
            if path.starts_with("/") {
                Ok(path.to_path_buf())
            } else {
                Ok(PathBuf::from("/").join(path))
            }
        }
    }

    fn walk_dir(&self, _root: &Path) -> io::Result<Vec<PathBuf>> {
        if let Some(err) = &self.walk_error {
            return Err(io::Error::new(err.kind(), err.to_string()));
        }
        Ok(self.files.keys().cloned().collect())
    }
}
