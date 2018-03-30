extern crate futures;
extern crate hyper;
extern crate statics_lib;
extern crate stq_http;
extern crate tokio_core;
extern crate mime;

pub mod common;

use futures::future::Future;
use hyper::{Uri, Request, Method};
use hyper::header::{ContentLength, ContentType};
use std::str::FromStr;
use stq_http::request_util::read_body;

// fn multipart_mime(bound: &str) -> Mime {
//     Mime(
//         TopLevel::Multipart, SubLevel::Ext("form-data".into()),
//         vec![(Attr::Ext("boundary".into()), Value::Ext(bound.into()))]
//     )
// }

// pub struct Mime {
//     source: Source,
//     slash: usize,
//     plus: Option<usize>,
//     params: ParamSource,
// }


#[test]
fn images_post() {
    let mut context = common::setup();
    let bytes = common::read_static_file("image-328x228.png");
    let mut body = b"-----------------------------2132006148186267924133397521\r\nContent-Disposition: form-data; name=\"file\"; filename=\"image-328x228.png\nContent-Type: image/png\r\n\r\n".to_vec();
    body.extend(bytes);
    body.extend(b"\r\n-----------------------------2132006148186267924133397521--\r\n".into_iter());
    let boundary = "---------------------------2132006148186267924133397521";
    let mime = format!("multipart/form-data; boundary={}", boundary).parse::<mime::Mime>().unwrap();
    let url = Uri::from_str(&format!("{}/images", context.base_url)).unwrap();
    let mut req = Request::new(Method::Post, url);
    req.headers_mut().set(ContentType(mime));
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
    // assert_eq!(response, "\"Ok\"");
}

// -----------------------------12640807573495631281739717751
// Content-Disposition: form-data; name="file"; filename="image-328x228.png"
// Content-Type: image/png
// -----------------------------12640807573495631281739717751--

// Content-Type: multipart/form-data; boundary=---------------------------12640807573495631281739717751
