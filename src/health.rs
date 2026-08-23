use actix_web::{
    http::{header, Error},
    route, HttpResponse, Responder,
};

#[route("/health", method = "GET", method = "HEAD")]
pub async fn health() -> Result<impl Responder, Error> {
    Ok(HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body("{\"status\":\"ok\"}"))
}
