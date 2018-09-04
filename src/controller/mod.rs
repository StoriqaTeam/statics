//! `Controller` is a top layer that handles all http-related
//! stuff like reading bodies, parsing params, forming a response.
//! Basically it provides inputs to `Service` layer and converts outputs
//! of `Service` layer to http responses

pub mod multipart_utils;
pub mod routes;
pub mod utils;

use std::io::Read;
use std::sync::Arc;

use failure;
use failure::Fail;
use futures::future;
use futures::prelude::*;
use hyper;
use hyper::header::{Authorization, Bearer};
use hyper::server::Request;
use hyper::Headers;
use hyper::Post;
use image;
use jsonwebtoken::{decode, Algorithm, Validation};
use multipart::server::Multipart;

use stq_http::client::ClientHandle;
use stq_http::controller::{Controller, ControllerFuture};
use stq_http::request_util::serialize_future;
use stq_router::RouteParser;

use self::routes::Route;
use config::Config;
use errors::*;
use services::s3::S3;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JWTPayload {
    pub user_id: i32,
    pub exp: i64,
}

pub fn verify_token(jwt_key: Vec<u8>, leeway: i64, headers: &Headers) -> Box<Future<Item = JWTPayload, Error = failure::Error>> {
    Box::new(
        future::result(
            headers
                .get::<Authorization<Bearer>>()
                .map(|auth| auth.clone())
                .ok_or_else(|| format_err!("Missing token").context(Error::Unauthorized).into()),
        ).and_then(move |auth| {
            let token = auth.0.token.as_ref();

            let validation = Validation {
                leeway,
                ..Validation::new(Algorithm::RS256)
            };
            decode::<JWTPayload>(token, &jwt_key, &validation)
                .map_err(|e| format_err!("Failed to parse JWT token: {}", e).context(Error::Unauthorized).into())
        })
            .map(|t| t.claims),
    )
}

/// Controller handles route parsing and calling `Service` layer
pub struct ControllerImpl {
    pub config: Config,
    pub jwt_public_key: Vec<u8>,
    pub route_parser: Arc<RouteParser<Route>>,
    pub client: ClientHandle,
    pub s3: Arc<S3>,
}

impl ControllerImpl {
    /// Create a new controller based on services
    pub fn new(config: Config, jwt_public_key: Vec<u8>, client: ClientHandle, s3: Arc<S3>) -> Self {
        let route_parser = Arc::new(routes::create_route_parser());
        Self {
            config,
            jwt_public_key,
            route_parser,
            client,
            s3,
        }
    }
}

impl Controller for ControllerImpl {
    /// Handle a request and get future response
    fn call(&self, req: Request) -> ControllerFuture {
        let s3 = self.s3.clone();

        let result: ControllerFuture = match (req.method(), self.route_parser.test(req.path())) {
            // POST /images
            (&Post, Some(Route::Images)) => serialize_future({
                let method = req.method().clone();
                let headers = req.headers().clone();

                debug!("Received image upload request");

                future::ok(())
                    .and_then({
                        let headers = headers.clone();
                        let leeway = self.config.jwt.leeway;
                        let jwt_key = self.jwt_public_key.clone();
                        move |_| verify_token(jwt_key, leeway, &headers)
                    })
                    .and_then(|_user_id| {
                        read_bytes(req.body()).map_err(|e| e.context("Failed to read request body").context(Error::Network).into())
                    })
                    .and_then(move |bytes| {
                        debug!("Read payload bytes");
                        let multipart_wrapper = multipart_utils::MultipartRequest::new(method, headers, bytes);
                        Multipart::from_request(multipart_wrapper).map_err(|_| {
                            format_err!("Couldn't convert request body to multipart")
                                .context(Error::Parse)
                                .into()
                        })
                    })
                    .and_then(|multipart_entity| match multipart_entity.into_entry().into_result() {
                        Ok(Some(field)) => Ok(field),
                        _ => Err(format_err!("Parsed multipart, but couldn't read the next entry")
                            .context(Error::Parse)
                            .into()),
                    })
                    .and_then(|mut field| {
                        let mut data: Vec<u8> = Vec::new();
                        let _ = field.data.read_to_end(&mut data);
                        image::guess_format(&data)
                            .map_err(|e| e.context("Invalid image format").context(Error::Image).into())
                            .map(|format| (format, data))
                            .into_future()
                    })
                    .and_then(move |(format, data)| {
                        Box::new(
                            s3.upload_image(format, data)
                                .map(|name| json!({ "url": name }))
                                .map_err(|e| e.context(Error::Image).into()),
                        )
                    })
            }),

            // Fallback
            _ => serialize_future::<String, _, _>(Err(Error::NotFound)),
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
