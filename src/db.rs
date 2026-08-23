use actix_web::web;
use anyhow::Result;
use rusqlite::{OptionalExtension, types::Null};
use serde::{Deserialize, Serialize};

pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

#[derive(Clone, Serialize, Deserialize)]
pub struct Content {
    pub key: String,
    pub content_type: String,
    pub expiry: Option<i64>,
    pub last_modified: i64,
    pub modifiable: bool,
    pub auth_key: Option<String>,
    pub content_encoding: String,
    pub backend_id: String,
    pub content_length: i32,
    pub content: Option<Vec<u8>>,
}

impl Content {
    pub fn clone_metadata(&self) -> Content {
        Content {
            key: self.key.clone(),
            content_type: self.content_type.clone(),
            expiry: self.expiry,
            last_modified: self.last_modified,
            modifiable: self.modifiable,
            auth_key: self.auth_key.clone(),
            content_encoding: self.content_encoding.clone(),
            backend_id: self.backend_id.clone(),
            content_length: self.content_length,
            content: None,
        }
    }
}

pub fn create_db(conn: Connection) -> Result<usize> {
    Ok(conn.execute(
        "CREATE TABLE `content` (
            `key` VARCHAR NOT NULL ,
            `content_type` VARCHAR ,
            `expiry` INTEGER ,
            `last_modified` BIGINT ,
            `encoding` VARCHAR ,
            `backend_id` VARCHAR ,
            `content_length` INTEGER ,
            PRIMARY KEY (`key`)
        );",
        (),
    )?)
}

pub async fn save_content_info(pool: &Pool, content: &Content) -> Result<usize> {
    let pool = pool.clone();

    let conn = web::block(move || pool.get()).await??;

    let content = content.clone_metadata();

    web::block(move || {
        // INSERT INTO content VALUES('2TIzc','text/plain',NULL,1721160516802,'gzip','local',157);
        Ok(conn.execute(
            "INSERT INTO content (
                key,
                content_type,
                expiry,
                last_modified, 
                encoding,
                backend_id,
                content_length
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            (
                content.key,
                content.content_type,
                Null, // We don't support expiration
                content.last_modified,
                content.content_encoding,
                content.backend_id,
                content.content_length,
            ),
        )?)
    })
    .await?
}

pub async fn get_content_info(pool: &Pool, key: String) -> Result<Option<Content>> {
    let pool = pool.clone();

    let conn = web::block(move || pool.get()).await??;

    web::block(move || {
        let mut stmt = conn.prepare("SELECT * FROM content WHERE key=:key;")?;
        Ok(stmt
            .query_row(&[(":key", &key)], |row| {
                Ok(Content {
                    key: row.get(0)?,
                    content_type: row.get(1)?,
                    expiry: row.get(2)?,
                    last_modified: row.get(3)?,
                    modifiable: false,
                    auth_key: None,
                    content_encoding: row.get(4)?,
                    backend_id: row.get(5)?,
                    content_length: row.get(6)?,
                    content: None,
                })
            })
            .optional()?)
    })
    .await?
}
