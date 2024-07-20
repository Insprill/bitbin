use actix_web::{
    error::{ErrorBadRequest, ErrorInternalServerError, ErrorPayloadTooLarge},
    http::header::{self, ContentEncoding},
    post,
    web::{self, Bytes, Data},
    Error, HttpMessage, HttpRequest, HttpResponse, Responder,
};
use anyhow::Result;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::Serialize;
use std::{io::prelude::*, time::SystemTime};

use crate::{
    db::{self, Content},
    State,
};

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

    let key = random_string::generate(
        state.config.misc.keylength,
        random_string::charsets::ALPHANUMERIC,
    );

    // ah sweet, man-made horros beyond my comprehension
    let mut content_encoding = req
        .headers()
        .get(header::CONTENT_ENCODING)
        .and_then(|h| h.to_str().ok())
        .map(|t| {
            t.split(',')
                .filter_map(|t| {
                    t.split(';').next().map(|x| {
                        if x == "x-gzip" {
                            ContentEncoding::Gzip.as_str().to_string()
                        } else {
                            x.trim().to_string()
                        }
                    })
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let mut bytes: Vec<u8> = bytes.into();

    let compression_level = state.config.content.gzip_compression_level;
    if content_encoding.is_empty() {
        bytes = web::block(move || {
            let mut gz = GzEncoder::new(Vec::new(), Compression::new(compression_level));
            match gz.write_all(&bytes) {
                Ok(_) => Ok(gz.finish()),
                Err(err) => Err(err),
            }
        })
        .await???;
        content_encoding = vec![ContentEncoding::Gzip.as_str().to_string()];
    }

    let last_modified: i64 = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(ErrorInternalServerError)?
        .as_millis()
        .try_into()
        .map_err(ErrorInternalServerError)?;

    let content = Content {
        key: key.clone(),
        content_type,
        expiry: None, // Not supported (yet)
        last_modified,
        modifiable: false, // Not supported (yet)
        auth_key: None,
        content_encoding: content_encoding.join(","),
        backend_id: state.storage.backend_id().to_string(),
        content_length: bytes.len(),
        content: Some(bytes),
    };

    if let Err(err) = db::save_content_info(&state.pool, &content).await {
        return Err(ErrorInternalServerError(err));
    };

    if let Err(err) = state.storage.save_content(content) {
        //db::delete_content_info();
        return Err(ErrorInternalServerError(err));
    }

    let res = Response { key: &key };
    Ok(HttpResponse::Created()
        .insert_header(("Location", res.key))
        .json(res))
}

#[derive(Serialize)]
pub struct Response<'a> {
    key: &'a str,
}
