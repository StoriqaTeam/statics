//! Error for service module
use hyper::StatusCode;
use serde_json;
use stq_http::errors::{Codeable, PayloadCarrier};

/// Error for service module
#[derive(Debug, Fail)]
pub enum Error {
    #[fail(display = "Not found")]
    NotFound,
    #[fail(display = "Invalid image")]
    Image,
    #[fail(display = "Parse error")]
    Parse,
    #[fail(display = "Unauthorized")]
    Unauthorized,
    #[fail(display = "Network error")]
    Network,
}

impl Codeable for Error {
    fn code(&self) -> StatusCode {
        use self::Error::*;

        match self {
            NotFound => StatusCode::NotFound,
            Image => StatusCode::UnprocessableEntity,
            Parse => StatusCode::UnprocessableEntity,
            Unauthorized | Network => StatusCode::BadRequest,
        }
    }
}

impl PayloadCarrier for Error {
    fn payload(&self) -> Option<serde_json::Value> {
        None
    }
}
