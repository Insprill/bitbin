use std::{fs, path::PathBuf};

use actix_web::{
    error::{ErrorInternalServerError, ErrorNotFound},
    http::header::ContentEncoding,
    Result,
};
use log::error;

use crate::{
    data::{DataReader, DataWriter},
    db::Content,
};

pub trait StorageBackend {
    fn backend_id(&self) -> &'static str;
    fn initialize(&self) -> Result<()>;
    fn save_content(&self, content: Content) -> Result<()>;
    fn get_content(&self, key: &str, skip_content: bool) -> Result<Content>;
    fn list_all_content(&self) -> Result<Vec<Content>>;
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

    fn initialize(&self) -> Result<()> {
        if !self.path.exists() {
            fs::create_dir(&self.path)?;
        }
        Ok(())
    }

    fn save_content(&self, content: Content) -> Result<()> {
        let content_data = content.content.ok_or_else(|| {
            ErrorInternalServerError("Tried saving content, but there's no content to save")
        })?;

        // Ensure we still have the data directory in case it got deleted for some reason
        self.initialize()?;

        // Pre-compute length so we don't need to re-allocate
        let len = 4 // Version (int)
        + 2 + content.key.len() // Key (ushort string)
        + 4 + content.content_type.len() // Content Type (int string)
        + 8 // Expiry (long)
        + 8 // Last Modified (long)
        + 1 // Is Modifiable (bool)
        + if content.modifiable { 2 } else { 0 }  // Auth Key (ushort string)
        + 4 // Content Encoding (int string)
        + 4 // Content Length (int)
        + content_data.len(); // Content
        let mut w = DataWriter::new(len);

        // Version
        w.write_int(2);

        // Key
        w.write_utf(&content.key)?;

        // Content Type
        w.write_utf_long(&content.content_type)?;

        // Expiry
        w.write_long(content.expiry.unwrap_or(-1));

        // Last Modified
        w.write_long(content.last_modified);

        // Is Modifiable
        w.write_bool(content.modifiable);

        // Auth Key
        if content.modifiable {
            w.write_utf(&content.auth_key.unwrap_or_default())?;
        }

        // Content Encoding
        w.write_utf_long(&content.content_encoding)?;

        w.write_int_from_usize(content_data.len())?;

        w.write_slice(&content_data);

        let data_path = self.path.join(content.key);
        if data_path.exists() {
            return Err(ErrorInternalServerError("Key already used"));
        }

        fs::write(data_path, w.get_data())?;

        Ok(())
    }

    fn get_content(&self, key: &str, skip_content: bool) -> Result<Content> {
        let data_path = self.path.join(key);
        if !data_path.exists() {
            return Err(ErrorNotFound("Invalid path"));
        }

        // todo: don't read all file data if we're skipping the content
        let file_data = fs::read(data_path)?;
        let mut r = DataReader::new(&file_data);

        let version = r.read_int();

        let key = r.read_utf()?;

        let content_type = r.read_utf_long()?;

        let expiry = r.read_long();
        let expiry = if expiry == -1 { None } else { Some(expiry) };

        let last_modified = r.read_long();
        let modifiable = r.read_bool();
        let auth_key = if modifiable {
            Some(r.read_utf()?)
        } else {
            None
        };

        let content_encoding = if version == 1 {
            ContentEncoding::Gzip.as_str().to_string()
        } else {
            r.read_utf_long()?
        };

        let content_length: usize = r.read_int_as_usize()?;
        let mut content = vec![0u8; content_length];
        r.read_fully(&mut content)?;

        Ok(Content {
            key,
            content_type,
            expiry,
            last_modified,
            modifiable,
            auth_key,
            content_encoding,
            backend_id: self.backend_id().to_string(),
            content_length,
            content: if skip_content { None } else { Some(content) },
        })
    }

    fn list_all_content(&self) -> Result<Vec<Content>> {
        Ok(fs::read_dir(&self.path)?
            .filter_map(|x| {
                if let Ok(path) = x {
                    if path.path().is_file() {
                        return path
                            .path()
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string());
                    } else {
                        return None;
                    }
                }
                None
            })
            .filter_map(|key| match self.get_content(&key, true) {
                Ok(content) => Some(content),
                Err(err) => {
                    error!("Failed to get content for paste {}: {}", key, err);
                    None
                }
            })
            .collect())
    }
}
