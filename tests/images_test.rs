extern crate futures;
extern crate hyper;
extern crate statics_lib;
extern crate stq_http;
extern crate tokio_core;
extern crate mime;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

pub mod common;

use futures::future::Future;
use hyper::{Uri, Request, Method};
use hyper::header::{ContentLength, ContentType};
use std::str::FromStr;
use stq_http::request_util::read_body;
use common::Context;
use futures::future;
use futures::Stream;

#[derive(Serialize, Deserialize)]
struct UrlResponse {
    url: String,
}

#[test]
fn images_post() {
    let mut context = common::setup();
    let original_filename = "image-328x228.png";
    let original_bytes = common::read_static_file(original_filename);
    let mut body = b"-----------------------------2132006148186267924133397521\r\nContent-Disposition: form-data; name=\"file\"; filename=\"image-328x228.png\nContent-Type: image/png\r\n\r\n".to_vec();
    body.extend(original_bytes);
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
    let url = serde_json::from_str::<UrlResponse>(&response).unwrap().url;
    let mut_ctx = &mut context;
    let futures: Vec<_> = ["original", "thumb", "small", "medium", "large"].iter().map(|size| {
            fetch_image_from_s3_and_file(mut_ctx, original_filename, &url, size)
                .map(|(local, remote)| {
                    assert_eq!(local, remote);
                })
    }).collect();
    let _ = mut_ctx.core.run(future::join_all(futures));
}

fn fetch_image_from_s3_and_file(context: &mut Context, filename: &str, url: &str, size: &str) -> Box<Future<Item = (Vec<u8>, Vec<u8>), Error = hyper::Error>> {
        let filename = add_size_to_url(filename, size);
        let url = add_size_to_url(url, size);
        let uri = Uri::from_str(&url).unwrap();
        Box::new(
            context
                .client
                .get(uri)
                .and_then(|resp| read_bytes(resp.body()))
                .map(move |remote_bytes| {
                    let local_bytes = common::read_static_file(&filename);
                    (remote_bytes, local_bytes)
                })
        )
}

fn add_size_to_url(url: &str, size: &str) -> String {
    if size == "original" { return url.to_string() };
    url.replace(".png", &format!("-{}.png", size))
}

/// Reads body of request and response in Future format
pub fn read_bytes(body: hyper::Body) -> Box<Future<Item = Vec<u8>, Error = hyper::Error>> {
    Box::new(body.fold(Vec::new(), |mut acc, chunk| {
        acc.extend_from_slice(&*chunk);
        future::ok::<_, hyper::Error>(acc)
    }))
}
