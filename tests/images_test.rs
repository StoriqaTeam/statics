extern crate statics_lib;
extern crate hyper;
extern crate futures;
extern crate stq_http;
extern crate tokio_core;

use std::thread;
use std::time;
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
    println!("Url: {:?}", url);
    // let response = context.client.get(url).and_then(|resp| read_body(resp.body())).wait().unwrap();
    let response = context.client.get(url);
    println!("Response: {:?}", response);
    // assert_eq!(response, "ok");
}

struct Context {
    pub client: HttpClient,
    pub base_url: String,
    core: Core,
}

fn setup() -> Context {
    thread::Builder::new().name("Server thread".to_string()).spawn(|| {
        let config = statics_lib::config::Config::new().expect("Can't load app config!");
        statics_lib::start_server(config);
    });
    thread::sleep(time::Duration::from_millis(1000));
    let core = Core::new().expect("Unexpected error creating event loop core");
    let client = Client::new(&core.handle());
    Context { client, base_url: "http://localhost:8000".to_string(), core }
}
