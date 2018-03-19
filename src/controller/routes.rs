use stq_router::RouteParser;

/// List of all routes with params for the app
#[derive(Clone, Debug, PartialEq)]
pub enum Route {
    Healthcheck,
}

pub fn create_route_parser() -> RouteParser<Route> {
    let mut router = RouteParser::default();

    // Healthcheck
    router.add_route(r"^/healthcheck$", || Route::Healthcheck);

    router
}
