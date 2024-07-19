use actix_web::{
    dev::ServiceResponse, error::ErrorInternalServerError, middleware::ErrorHandlerResponse, Result,
};
use log::error;

pub fn handle_500<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let err = get_err_str(&res);
    if let Some(str) = &err {
        error!("{}", str);
    }

    Err(ErrorInternalServerError("Server error"))
}

fn get_err_str<B>(res: &ServiceResponse<B>) -> Option<String> {
    res.response().error().map(|err| err.to_string())
}
