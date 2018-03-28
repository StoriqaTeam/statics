extern crate statics_lib;
extern crate hyper;
extern crate futures;
extern crate stq_http;
extern crate tokio_core;

use std::str::FromStr;
use hyper::{Client, Uri};
use hyper::client::HttpConnector;
use futures::future::Future;
use stq_http::request_util::read_body;
use tokio_core::reactor::Core;

type HttpClient = Client<HttpConnector>;

#[test]
fn it_adds_two() {
    let context = setup();
    let url = Uri::from_str(&format!("{}/healthcheck", context.base_url)).unwrap();
    let response = context.client.get(url).and_then(|resp| read_body(resp.body())).wait().unwrap();
    assert_eq!(response, "ok");
}

struct Context {
    pub client: HttpClient,
    pub base_url: String,
}

fn setup() -> Context {
    let config = statics_lib::config::Config::new().expect("Can't load app config!");
    statics_lib::start_server(config);
    let core = Core::new().expect("Unexpected error creating event loop core");
    let client = Client::new(&core.handle());
    Context { client, base_url: "http://localhost:8000".to_string() }
}
