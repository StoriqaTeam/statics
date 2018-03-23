//! Module containgin all the app routes
//! Currently it's
//! - `GET /healthcheck` - returns `ok` if the server is live
//! - `POST /images` - accepts multipart HTTP requests with png / jpeg images
use stq_router::RouteParser;

/// List of all routes with params for the app
#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Healthcheck,
    Images,
}

/// Creates global app route parser
pub fn create_route_parser() -> RouteParser<Route> {
    let mut router = RouteParser::default();

    // Healthcheck
    router.add_route(r"^/healthcheck$", || Route::Healthcheck);

    // Images upload route
    router.add_route(r"^/images$", || Route::Images);

    router
}
