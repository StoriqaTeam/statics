//! `Controller` is a top layer that handles all http-related
//! stuff like reading bodies, parsing params, forming a response.
//! Basically it provides inputs to `Service` layer and converts outputs
//! of `Service` layer to http responses

pub mod multipart_utils;
pub mod routes;
pub mod utils;

use std::io::Read;
use std::str::FromStr;
use std::sync::Arc;

use futures::future;
use futures::prelude::*;
use hyper;
use hyper::Headers;
use hyper::header::{Authorization, Bearer};
use hyper::server::Request;
use hyper::{Get, Post};
use jsonwebtoken::{decode, Validation};
use multipart::server::Multipart;
// use hyper::header::Authorization;

use stq_http::client::ClientHandle;
use stq_http::controller::Controller;
use stq_http::errors::ControllerError;
use stq_http::request_util::ControllerFuture;
use stq_http::request_util::serialize_future;
use stq_router::RouteParser;

use self::routes::Route;
use config::Config;
use services::error::ServiceError;
use services::s3::S3;
use services::system::{SystemService, SystemServiceImpl};
use services::types::ImageFormat;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JWTPayload {
    pub user_id: i32,
}

pub fn verify_token(jwt_secret_key: String, headers: &Headers) -> Box<Future<Item = JWTPayload, Error = ControllerError>> {
    Box::new(
        future::result(
            headers
                .get::<Authorization<Bearer>>()
                .map(|auth| auth.clone())
                .ok_or_else(|| ControllerError::BadRequest(ServiceError::Unauthorized("Missing token".into()).into())),
        ).and_then(move |auth| {
            let token = auth.0.token.as_ref();
            decode::<JWTPayload>(token, jwt_secret_key.as_ref(), &Validation::default())
                .map_err(|e| ControllerError::BadRequest(ServiceError::Unauthorized(format!("Failed to parse JWT token: {}", e)).into()))
        })
            .map(|t| t.claims),
    )
}

/// Controller handles route parsing and calling `Service` layer
pub struct ControllerImpl {
    pub config: Config,
    pub route_parser: Arc<RouteParser<Route>>,
    pub client: ClientHandle,
    pub s3: Arc<S3>,
}

impl ControllerImpl {
    /// Create a new controller based on services
    pub fn new(config: Config, client: ClientHandle, s3: Arc<S3>) -> Self {
        let route_parser = Arc::new(routes::create_route_parser());
        Self {
            config,
            route_parser,
            client,
            s3,
        }
    }
}

impl Controller for ControllerImpl {
    /// Handle a request and get future response
    fn call(&self, req: Request) -> ControllerFuture {
        // let headers = req.headers().clone();
        // let auth_header = headers.get::<Authorization<String>>();
        // let user_id = auth_header
        //     .map(move |auth| auth.0.clone())
        //     .and_then(|id| i32::from_str(&id).ok());

        let system_service = SystemServiceImpl::new();
        let s3 = self.s3.clone();

        let result: ControllerFuture = match (req.method(), self.route_parser.test(req.path())) {
            // GET /healthcheck
            (&Get, Some(Route::Healthcheck)) => serialize_future(system_service.healthcheck()),

            // POST /images
            (&Post, Some(Route::Images)) => {
                let method = req.method().clone();
                let headers = req.headers().clone();

                debug!("Received image upload request");

                Box::new(
                    future::ok(())
                        .and_then({
                            let headers = headers.clone();
                            let secret_key = self.config.jwt.secret_key.clone();
                            move |_| verify_token(secret_key, &headers)
                        })
                        .and_then(|_user_id| read_bytes(req.body()).map_err(|e| ControllerError::UnprocessableEntity(e.into())))
                        .and_then(move |bytes| {
                            debug!("Read payload bytes");
                            let multipart_wrapper = multipart_utils::MultipartRequest::new(method, headers, bytes);
                            let multipart_entity = match Multipart::from_request(multipart_wrapper) {
                                Err(_) => {
                                    return Box::new(future::err::<String, ControllerError>(ControllerError::UnprocessableEntity(
                                        multipart_utils::MultipartError::Parse("Couldn't convert request body to multipart".to_string())
                                            .into(),
                                    ))) as ControllerFuture
                                }
                                Ok(mp) => mp,
                            };
                            let mut field = match multipart_entity.into_entry().into_result() {
                                Ok(Some(field)) => field,
                                _ => {
                                    return Box::new(future::err::<String, ControllerError>(ControllerError::UnprocessableEntity(
                                        multipart_utils::MultipartError::Parse(
                                            "Parsed multipart, but couldn't read the next entry".to_string(),
                                        ).into(),
                                    ))) as ControllerFuture
                                }
                            };
                            let format: Result<ImageFormat, ControllerError> = field
                                .headers
                                .content_type
                                .ok_or(ControllerError::UnprocessableEntity(
                                    multipart_utils::MultipartError::Parse(
                                        "Parsed and read entry, but couldn't read content-type".to_string(),
                                    ).into(),
                                ))
                                .and_then(|ct| ImageFormat::from_str(ct.subtype().as_str()).map_err(|e| e.into()));
                            let format = match format {
                                Ok(format) => format,
                                Err(e) => return Box::new(future::err::<String, _>(e)),
                            };
                            let mut data: Vec<u8> = Vec::new();
                            let _ = field.data.read_to_end(&mut data);
                            let result: ControllerFuture = Box::new(
                                s3.upload_image(format, data)
                                    .map(|name| json!({"url": name}).to_string())
                                    .map_err(|e| ControllerError::UnprocessableEntity(e.into())),
                            );
                            result
                        }),
                )
            }

            // Fallback
            _ => Box::new(future::err(ControllerError::NotFound)),
        };
        result
    }
}

/// Reads body of request and response in Future format
pub fn read_bytes(body: hyper::Body) -> Box<Future<Item = Vec<u8>, Error = hyper::Error>> {
    Box::new(body.fold(Vec::new(), |mut acc, chunk| {
        acc.extend_from_slice(&*chunk);
        future::ok::<_, hyper::Error>(acc)
    }))
}
