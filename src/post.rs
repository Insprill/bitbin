use std::{fs, path::PathBuf};

use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorPayloadTooLarge},
    post,
    web::{Bytes, Data},
    Error, HttpResponse, Responder,
};
use anyhow::Result;
use serde::Serialize;

use crate::{db, State};

const MB_LEN: usize = 1024 * 1024;

#[post("/post")]
pub async fn post(state: Data<State>, bytes: Bytes) -> Result<impl Responder, Error> {
    if bytes.is_empty() {
        return Err(ErrorBadRequest("Missing content"));
    }

    let len = bytes.len();

    if len > state.config.content.maxsize * MB_LEN {
        return Err(ErrorPayloadTooLarge("Content too large"));
    }

    let res = Response {
        key: random_string::generate(
            state.config.misc.keylength,
            random_string::charsets::ALPHANUMERIC,
        ),
    };

    let data_path = PathBuf::from("content");

    if !data_path.exists() {
        let _ = fs::create_dir(&data_path);
    }

    let data_path = data_path.join(&res.key);
    if data_path.exists() {
        return Err(ErrorInternalServerError("Key already used"));
    }

    fs::write(data_path, bytes)?;

    match db::save_content_info(&state.pool, res.key.clone(), "".to_string(), len).await {
        Ok(_) => {}
        Err(err) => {
            log::info!("shit broke");
            return Err(ErrorInternalServerError(err));
        }
    };

    Ok(HttpResponse::Created()
        .insert_header(("Location", res.key.clone()))
        .json(res))
}

#[derive(Serialize)]
pub struct Response {
    key: String,
}
