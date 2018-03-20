//! `Controller` is a top layer that handles all http-related
//! stuff like reading bodies, parsing params, forming a response.
//! Basically it provides inputs to `Service` layer and converts outputs
//! of `Service` layer to http responses

pub mod routes;
pub mod utils;

use std::sync::Arc;

use future;
use hyper::{Get};
use hyper::server::Request;
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
            s3
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

        match (req.method(), self.route_parser.test(req.path())) {
            // GET /healthcheck
            (&Get, Some(Route::Healthcheck)) => serialize_future(system_service.healthcheck()),

            // Fallback
            _ => Box::new(future::err(ControllerError::NotFound)),
        }
    }
}
