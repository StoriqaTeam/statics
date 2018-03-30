extern crate futures;
extern crate hyper;
extern crate statics_lib;
extern crate stq_http;
extern crate tokio_core;
extern crate multipart;

pub mod common;

use futures::future::Future;
use hyper::{Uri, Request, Method};
use hyper::header::ContentLength;
use std::str::FromStr;
use stq_http::request_util::read_body;
use multipart::client::hyper::{content_type as multipart_content_type};

#[test]
fn images_post() {
    let mut context = common::setup();
    let bytes = common::read_static_file("image-328x228.png");
    let body = r#"
        -----------------------------12640807573495631281739717751
        Content-Disposition: form-data; name="file"; filename="image-328x228.png"
        Content-Type: image/png

    "#
        .into_bytes()
        .extend(bytes)
        .extend(b"Content-Type: multipart/form-data; boundary=---------------------------12640807573495631281739717751");
    let boundary = "---------------------------12640807573495631281739717751";
    let url = Uri::from_str(&format!("{}/images", context.base_url)).unwrap();
    let mut req = Request::new(Method::Post, url);
    req.headers_mut().set(multipart_content_type(boundary));
    req.headers_mut().set(ContentLength(body.len() as u64));
    req.set_body(body);
    let response = context
        .core
        .run(
            context
                .client
                .request(req)
                .and_then(|resp| read_body(resp.body())),
        )
        .unwrap();
    println!("{}", response);
    // assert_eq!(response, "\"Ok\"");
}

// -----------------------------12640807573495631281739717751
// Content-Disposition: form-data; name="file"; filename="image-328x228.png"
// Content-Type: image/png
// -----------------------------12640807573495631281739717751--

// Content-Type: multipart/form-data; boundary=---------------------------12640807573495631281739717751
