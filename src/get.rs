use actix_web::{
    error::{ErrorInternalServerError, ErrorNotAcceptable, ErrorNotFound},
    get,
    http::header::{self, ContentEncoding},
    web::{self, Bytes, Data},
    Error, HttpRequest, HttpResponse, Responder,
};
use anyhow::Result;
use flate2::read::GzDecoder;
use log::warn;
use std::io::prelude::*;

use crate::{
    db::{self, Content},
    State,
};

const CACHE_CONTROL_STATIC: &str = "public, max-age=604800, no-transform, immutable";
#[allow(dead_code)]
const CACHE_CONTROL_DYNAMIC: &str = "public, no-cache, proxy-revalidate, no-transform";

#[get("/{key}")]
pub async fn get(state: Data<State>, req: HttpRequest) -> Result<impl Responder, Error> {
    let key = match req.match_info().get("key") {
        Some(k) => k,
        None => {
            return Err(ErrorNotFound("Invalid path"));
        }
    };

    // This is responsible for preventing path-traversal!
    if !validate_path(key) {
        return Err(ErrorNotFound("Invalid path"));
    }

    let content = match db::get_content_info(&state.pool, key.to_string()).await {
        Ok(Some(c)) => c,
        Ok(None) => return Err(ErrorNotFound("Invalid path")),
        Err(err) => return Err(ErrorInternalServerError(err)),
    };

    // Once we have support for modifying existing content, we'll have to return the dynamic
    // variant of this for modifiable content, while returning the current one for static content.
    // https://github.com/lucko/bytebin/blob/9ac4aef610c3aa6215f17c7af78568908659d7b6/src/main/java/me/lucko/bytebin/http/GetHandler.java#L100-L114
    let cache_control = CACHE_CONTROL_STATIC;

    let content_data = state.storage.get_content(key, false)?.content.unwrap();

    let mut res = HttpResponse::Ok();
    res.insert_header((header::LAST_MODIFIED, content.last_modified));
    res.insert_header((header::CONTENT_TYPE, content.content_type.clone()));
    res.insert_header((header::CACHE_CONTROL, cache_control));

    let accept_encoding = get_accepted_encoding(&req);
    if accepts_encoding(&content, &accept_encoding) {
        return Ok(res
            .insert_header((header::CONTENT_ENCODING, content.content_encoding))
            .body(content_data));
    }

    if content.content_encoding == ContentEncoding::Gzip.as_str() {
        warn!("[REQUEST] Request for 'key = {}' was made with incompatible Accept-Encoding headers! Content-Encoding = {}, Accept-Encoding = {}", key, content.content_encoding, accept_encoding);
        let content_data = web::block(move || {
            let mut gz = GzDecoder::new(content_data.as_slice());
            let mut s = Vec::new();
            match gz.read_to_end(&mut s) {
                Ok(_) => Ok(s),
                Err(err) => Err(err),
            }
        })
        .await??;
        return Ok(res
            .insert_header((header::CONTENT_ENCODING, ContentEncoding::Identity.as_str()))
            .body(Bytes::from(content_data)));
    }

    Err(ErrorNotAcceptable(format!(
        "Accept-Encoding \"{}\" does not contain Content-Encoding \"{}\"",
        accept_encoding, content.content_encoding
    )))
}

fn validate_path(path: &str) -> bool {
    return path.chars().all(|c| c.is_ascii_alphanumeric());
}

fn get_accepted_encoding(req: &HttpRequest) -> String {
    match req
        .headers()
        .get(header::ACCEPT_ENCODING)
        .and_then(|h| h.to_str().ok())
    {
        Some(accept_encoding) => {
            format!("{},{}", ContentEncoding::Identity.as_str(), accept_encoding)
        }
        None => ContentEncoding::Identity.as_str().to_string(),
    }
}

fn accepts_encoding(content: &Content, accept_encoding: &str) -> bool {
    if !accept_encoding.contains('*') {
        let accept_encodings: Vec<&str> = accept_encoding
            .split(',')
            .filter_map(|t| t.split(';').next())
            .map(|e| e.trim())
            .collect::<Vec<&str>>();
        if !content
            .content_encoding
            .split(',')
            .all(|ce| accept_encodings.contains(&ce) || ce == ContentEncoding::Identity.as_str())
        {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_path_test() {
        assert!(validate_path("abc123"));
        assert!(!validate_path("..abc"));
        assert!(!validate_path("../abc"));
        assert!(!validate_path("abc/def"));
    }
}
