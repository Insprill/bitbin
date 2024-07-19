use std::time::SystemTime;

use actix_web::web;
use anyhow::Result;
use rusqlite::{types::Null, OptionalExtension};
use serde::{Deserialize, Serialize};

pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
pub type Connection = r2d2::PooledConnection<r2d2_sqlite::SqliteConnectionManager>;

#[derive(Serialize, Deserialize)]
pub struct Content {
    pub key: String,
    pub content_type: String,
    pub expiry: Option<u32>,
    pub last_modified: i64,
    pub encoding: String,
    pub backend_id: String,
    pub content_length: usize,
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

pub async fn save_content_info(
    pool: &Pool,
    key: String,
    content_type: String,
    content_encoding: Vec<String>,
    backend_id: &'static str,
    content_length: usize,
) -> Result<usize> {
    let pool = pool.clone();

    let conn = web::block(move || pool.get()).await??;

    web::block(move || {
        let curr_time: i64 = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_millis()
            .try_into()?;
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
                key,
                content_type,
                Null, // We don't support expiration
                curr_time,
                content_encoding.join(","),
                backend_id,
                content_length,
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
                    encoding: row.get(4)?,
                    backend_id: row.get(5)?,
                    content_length: row.get(6)?,
                })
            })
            .optional()?)
    })
    .await?
}
