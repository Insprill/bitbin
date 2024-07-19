use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorPayloadTooLarge},
    post,
    web::{Bytes, Data},
    Error, HttpMessage, HttpRequest, HttpResponse, Responder,
};
use anyhow::Result;
use serde::Serialize;

use crate::{db, State};

const MB_LEN: usize = 1024 * 1024;

#[post("/post")]
pub async fn post(
    state: Data<State>,
    req: HttpRequest,
    bytes: Bytes,
) -> Result<impl Responder, Error> {
    if bytes.is_empty() {
        return Err(ErrorBadRequest("Missing content"));
    }

    let len = bytes.len();

    if len > state.config.content.maxsize * MB_LEN {
        return Err(ErrorPayloadTooLarge("Content too large"));
    }

    let content_type = Some(req.content_type())
        .filter(|x| !x.is_empty())
        .unwrap_or("text/plain")
        .to_string();

    let res = Response {
        key: random_string::generate(
            state.config.misc.keylength,
            random_string::charsets::ALPHANUMERIC,
        ),
    };

    state.storage.save_content(&res.key, bytes)?;

    if let Err(err) = db::save_content_info(
        &state.pool,
        res.key.clone(),
        content_type,
        state.storage.backend_id(),
        len,
    )
    .await
    {
        return Err(ErrorInternalServerError(err));
    };

    Ok(HttpResponse::Created()
        .insert_header(("Location", res.key.clone()))
        .json(res))
}

#[derive(Serialize)]
pub struct Response {
    key: String,
}
