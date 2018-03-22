//! `Controller` is a top layer that handles all http-related
//! stuff like reading bodies, parsing params, forming a response.
//! Basically it provides inputs to `Service` layer and converts outputs
//! of `Service` layer to http responses

pub mod multipart_utils;
pub mod routes;
pub mod utils;

use std::sync::Arc;
use std::io::Read;

use futures::stream::Stream;
use futures::future;
use futures::future::Future;
use hyper;
use hyper::{Get, Post};
use hyper::server::Request;
use multipart::server::Multipart;
// use hyper::header::Authorization;

use stq_http::controller::Controller;
use stq_http::errors::ControllerError;
use stq_http::request_util::serialize_future;
use stq_http::request_util::ControllerFuture;
use stq_http::client::ClientHandle;
use stq_router::RouteParser;

use self::routes::Route;
use config::Config;
use services::system::{SystemService, SystemServiceImpl};
use services::s3::S3;

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

                Box::new(
                    read_bytes(req.body())
                        .map_err(|e| ControllerError::UnprocessableEntity(e.into()))
                        .and_then(move |bytes| {
                            let multipart_wrapper = multipart_utils::MultipartRequest::new(method, headers, bytes);
                            let multipart_entity = match Multipart::from_request(multipart_wrapper) {
                                Err(_) => {
                                    return Box::new(future::err::<String, ControllerError>(
                                        ControllerError::UnprocessableEntity(multipart_utils::MultipartError::Parse.into()),
                                    )) as ControllerFuture
                                }
                                Ok(mp) => mp,
                            };
                            let mut field = match multipart_entity.into_entry().into_result() {
                                Ok(Some(field)) => field,
                                _ => {
                                    return Box::new(future::err::<String, ControllerError>(
                                        ControllerError::UnprocessableEntity(multipart_utils::MultipartError::Parse.into()),
                                    )) as ControllerFuture
                                }
                            };
                            let content_type: String = field
                                .headers
                                .content_type
                                .map(|ct| ct.subtype().as_str().to_string())
                                .unwrap_or("unknown".to_string());
                            let mut data: Vec<u8> = Vec::new();
                            let _ = field.data.read_to_end(&mut data);
                            let result: ControllerFuture = Box::new(
                                s3.upload_image(&content_type[..], data)
                                    .map(|name| format!("{{\"url\": \"{}\"}}", name))
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
