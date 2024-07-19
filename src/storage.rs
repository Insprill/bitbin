use std::{fs, path::PathBuf};

use actix_web::{
    error::{ErrorInternalServerError, ErrorNotFound},
    Result,
};

pub trait StorageBackend {
    fn backend_id(&self) -> &'static str;
    fn save_content(&self, key: &str, bytes: Vec<u8>) -> Result<()>;
    fn get_content(&self, key: &str) -> Result<Vec<u8>>;
}

#[derive(Debug)]
pub struct LocalStorage {
    pub path: PathBuf,
}

impl LocalStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl StorageBackend for LocalStorage {
    fn backend_id(&self) -> &'static str {
        "local"
    }

    fn save_content(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        if !self.path.exists() {
            fs::create_dir(&self.path)?;
        }

        let data_path = self.path.join(key);
        if data_path.exists() {
            return Err(ErrorInternalServerError("Key already used"));
        }

        fs::write(data_path, bytes)?;

        Ok(())
    }

    fn get_content(&self, key: &str) -> Result<Vec<u8>> {
        let data_path = self.path.join(key);
        if !data_path.exists() {
            return Err(ErrorNotFound("Invalid path"));
        }

        let content_data = fs::read(data_path)?;
        Ok(content_data)
    }
}
