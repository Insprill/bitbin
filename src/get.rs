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

    // Once we have support for modifying existing content, we'll have to return the following
    // header for modifiable content, while returning the current one for static content.
    // public, no-cache, proxy-revalidate, no-transform
    // https://github.com/lucko/bytebin/blob/9ac4aef610c3aa6215f17c7af78568908659d7b6/src/main/java/me/lucko/bytebin/http/GetHandler.java#L100-L114
    let cache_control = "public, max-age=604800, no-transform, immutable";

    let content_data = state.storage.get_content(key)?;

    Ok(HttpResponse::Ok()
        .insert_header(("Last-Modified", content.last_modified))
        .insert_header(("Content-Type", content.content_type))
        .insert_header(("Cache-Control", cache_control))
        .body(content_data))
}
