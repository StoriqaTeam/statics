use stq_http::errors::ControllerError;
use stq_http::client::Error as HttpError;

#[derive(Debug, Fail)]
pub enum ServiceError {
    #[fail(display = "Not found")] NotFound,
    #[fail(display = "Http client error: {}", _0)] HttpClient(String),
    #[fail(display = "Unauthorized")] Unauthorized(String),
    #[fail(display = "Unknown error: {}", _0)] Unknown(String),
}

impl From<HttpError> for ServiceError {
    fn from(err: HttpError) -> Self {
        ServiceError::HttpClient(format!("{:?}", err))
    }
}

impl From<ServiceError> for ControllerError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::NotFound => ControllerError::NotFound,
            ServiceError::Unauthorized(msg) => ControllerError::BadRequest(ServiceError::Unauthorized(msg).into()),
            ServiceError::HttpClient(msg) => ControllerError::InternalServerError(ServiceError::HttpClient(msg).into()),
            ServiceError::Unknown(msg) => ControllerError::InternalServerError(ServiceError::Unknown(msg).into()),
        }
    }
}
