use std::{fs, path::PathBuf};

use actix_web::{
    error::{ErrorInternalServerError, ErrorNotFound},
    web::Bytes,
    Result,
};

pub trait StorageBackend {
    fn backend_id(&self) -> &'static str;
    fn save_content(&self, key: &str, bytes: Bytes) -> Result<()>;
    fn get_content(&self, key: &str) -> Result<Bytes>;
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

    fn save_content(&self, key: &str, bytes: Bytes) -> Result<()> {
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

    fn get_content(&self, key: &str) -> Result<Bytes> {
        let data_path = self.path.join(key);
        if !data_path.exists() {
            return Err(ErrorNotFound("Invalid path"));
        }

        let content_data = fs::read(data_path)?;
        Ok(content_data.into())
    }
}
