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
    // let futures = ["original", "thumb", "small", "medium", "large"].iter().map(|size| {
    //     let filename = add_size_to_url(original_filename, size);
    //     let url = add_size_to_url(url, size);
    //     let response = context
    //                 .client
    //                 .get(url)
    //                 .and_then(|resp| read_body(resp.body())),
    //         )
    //         .unwrap();
    // })
    println!("{}", url);
    // assert_eq!(response, "\"Ok\"");
    let (local, remote) = fetch_image_from_s3_and_file(&mut context, original_filename, &url, "original");
    assert_eq!(local, remote);
    let (local, remote) = fetch_image_from_s3_and_file(&mut context, original_filename, &url, "thumb");
    assert_eq!(local, remote);

    // let (local, remote) = fetch_image_from_s3_and_file(context, original_filename, &url, "thumb");
    // assert_eq!(local, remote);
    // let (local, remote) = fetch_image_from_s3_and_file(context, original_filename, &url, "small");
    // assert_eq!(local, remote);
    // let (local, remote) = fetch_image_from_s3_and_file(context, original_filename, &url, "medium");
    // assert_eq!(local, remote);
    // let (local, remote) = fetch_image_from_s3_and_file(context, original_filename, &url, "large");
    // assert_eq!(local, remote);

}

fn fetch_image_from_s3_and_file(context: &mut Context, filename: &str, url: &str, size: &str) -> (Vec<u8>, Vec<u8>) {
        let filename = add_size_to_url(filename, size);
        let url = add_size_to_url(url, size);
        println!("TOTO: {}", url);
        let uri = Uri::from_str(&url).unwrap();
        let response = context.core.run(
            context
                .client
                .get(uri)
                .and_then(|resp| read_bytes(resp.body()))
        ).unwrap();
        let file = common::read_static_file(&filename);
        (response, file)
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
