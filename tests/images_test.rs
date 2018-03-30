extern crate statics_lib;
extern crate hyper;
extern crate futures;
extern crate stq_http;
extern crate tokio_core;

pub mod common;

use futures::future::Future;
use hyper::Uri;
use std::str::FromStr;
use stq_http::request_util::read_body;

#[test]
fn it_adds_two() {
    let mut context = common::setup();
    let url = Uri::from_str(&format!("{}/healthcheck", context.base_url)).unwrap();
    let response = context.core.run(context.client.get(url).and_then(|resp| read_body(resp.body()))).unwrap();
    assert_eq!(response, "\"Ok\"");
}
