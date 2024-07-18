use std::{fs, path::PathBuf};

use actix_web::{
    error::{ErrorInternalServerError, ErrorNotFound},
    get,
    web::Data,
    Error, HttpRequest, HttpResponse, Responder,
};
use anyhow::Result;
use lazy_regex::*;

use crate::{db, State};

pub static VALID_KEY_PATTERN: Lazy<Regex> = lazy_regex!("^[a-zA-Z0-9]*$");

#[get("/{key}")]
pub async fn get(state: Data<State>, req: HttpRequest) -> Result<impl Responder, Error> {
    let key = match req.match_info().get("key") {
        Some(k) => k,
        None => {
            return Err(ErrorNotFound("Invalid path"));
        }
    };

    if key.contains('.') || !VALID_KEY_PATTERN.is_match(key) {
        return Err(ErrorNotFound("Invalid path"));
    }

    let content = match db::get_content_info(&state.pool, key.to_string()).await {
        Ok(Some(c)) => c,
        Ok(None) => return Err(ErrorNotFound("Invalid path")),
        Err(err) => return Err(ErrorInternalServerError(err)),
    };

    let data_path = PathBuf::from("content");

    let data_path = data_path.join(key);
    if !data_path.exists() {
        return Err(ErrorNotFound("Invalid path"));
    }

    let content_data = fs::read_to_string(data_path)?;

    Ok(HttpResponse::Ok()
        .insert_header(("Last-Modified", content.last_modified))
        .body(content_data))
}
