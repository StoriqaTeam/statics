extern crate chrono;
extern crate futures;
extern crate futures_timer;
extern crate hyper;
extern crate jsonwebtoken;
extern crate mime;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate statics_lib as lib;
extern crate stq_http;
extern crate tokio_core;

pub mod common;

use common::Context;
use futures::future;
use futures::future::Future;
use futures::Stream;
use futures_timer::FutureExt;
use hyper::header::{Authorization, Bearer, ContentLength, ContentType};
use hyper::StatusCode;
use hyper::{Method, Request, Uri};
use std::str::FromStr;
use stq_http::request_util::read_body;

#[derive(Serialize, Deserialize)]
struct UrlResponse {
    url: String,
}

#[derive(Default)]
struct UploadTester {
    original_filename: Option<String>,
    boundary: Option<String>,
    content_length: Option<u64>,
    content_type: Option<String>,
    jwt_token: Option<String>,
    response_status: Option<StatusCode>,
}

impl UploadTester {
    fn test(self) {
        let mut context = common::setup();
        let original_filename = &self.original_filename
            .unwrap_or("image-328x228.png".to_string());
        let original_bytes = common::read_static_file(original_filename);
        let mut body = Vec::new();
        body.extend(
            b"-----------------------------2132006148186267924133397521\r\nContent-Disposition: form-data; name=\"file\"; filename=\""
                .into_iter(),
        );
        body.extend(original_filename.clone().into_bytes().into_iter());
        body.extend(b"\nContent-Type: ".into_iter());
        body.extend(
            self.content_type
                .unwrap_or("image/png".to_string())
                .into_bytes()
                .into_iter(),
        );
        body.extend(b"\r\n\r\n".into_iter());
        body.extend(original_bytes);
        body.extend(b"\r\n-----------------------------2132006148186267924133397521--\r\n".into_iter());
        let boundary = self.boundary
            .unwrap_or("---------------------------2132006148186267924133397521".to_string());
        let mime = format!("multipart/form-data; boundary={}", &boundary)
            .parse::<mime::Mime>()
            .unwrap();
        let url = Uri::from_str(&format!("{}/images", context.base_url)).unwrap();
        let mut req = Request::new(Method::Post, url);
        req.headers_mut().set(Authorization::<Bearer>(Bearer {
            token: self.jwt_token.unwrap_or("eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJ1c2VyX2lkIjozLCJleHAiOjE1MjQyMjU4OTh9.O0OtQXgAJtgEgJ2luvQJWJBu1qWVafUvyk5dxMmr-1Nrcgk_IoIllQm1p_lY4j2VnWHdQGjHKTZgN6YmmnEDtcPaKQX7nsF73r378f3bIEnenwdMiqzNjwSgdG-Ke9WLzY3oOsbbjuIs5wv2FQvygvydzDzfYAg_BM02rRmDQSR6bRsHayjL2c9kV2ImGRJynjSQgwDSTubu3NnJmUHf66F5XtsC8aYCxBWJKSkNOXYNIF1oqw-59MmV3QppwEfICuaQQyGif_gxBAoXVonQGPByhI74lk-3rS5f6O2Yr09fUr0WyqkIgsKUXJC_JQwPbf7OWMDNLOdV2aKirpLraQ".into()),
        }));
        req.headers_mut().set(ContentType(mime));
        req.headers_mut().set(ContentLength(
            self.content_length.unwrap_or(body.len() as u64),
        ));
        req.set_body(body);

        let timeout = std::time::Duration::from_secs(10);

        println!("Sending request");
        let response = context
            .core
            .run(context.client.request(req).timeout(timeout))
            .unwrap();
        println!("Received response");

        assert_eq!(
            response.status(),
            self.response_status.unwrap_or(StatusCode::Ok)
        );

        if response.status() == StatusCode::Ok {
            let body = context.core.run(read_body(response.body())).unwrap();
            let url = serde_json::from_str::<UrlResponse>(&body).unwrap().url;
            let futures: Vec<_> = ["original", "thumb", "small", "medium", "large"]
                .into_iter()
                .map(|size| {
                    fetch_image_from_s3_and_file(&mut context, original_filename, &url, size).map(|(local, remote)| {
                        assert_eq!(local, remote);
                    })
                })
                .collect();
            context.core.run(future::join_all(futures)).unwrap();
        }
    }
}

#[test]
fn images_post() {
    UploadTester {
        ..Default::default()
    }.test()
}

#[test]
fn images_post_invalid_token() {
    UploadTester {
        jwt_token: Some("hello".into()),
        response_status: Some(StatusCode::BadRequest),
        ..Default::default()
    }.test()
}

#[test]
fn images_post_invalid_boundary() {
    UploadTester {
        boundary: Some("abeceda".into()),
        response_status: Some(StatusCode::UnprocessableEntity),
        ..Default::default()
    }.test()
}

#[test]
fn images_post_invalid_content_type() {
    UploadTester {
        content_type: Some("image/svg".into()),
        response_status: Some(StatusCode::UnprocessableEntity),
        ..Default::default()
    }.test()
}

fn fetch_image_from_s3_and_file(
    context: &mut Context,
    filename: &str,
    url: &str,
    size: &str,
) -> Box<Future<Item = (Vec<u8>, Vec<u8>), Error = hyper::Error>> {
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
            }),
    )
}

fn add_size_to_url(url: &str, size: &str) -> String {
    if size == "original" {
        return url.to_string();
    };
    url.replace(".png", &format!("-{}.png", size))
}

/// Reads body of request and response in Future format
pub fn read_bytes(body: hyper::Body) -> Box<Future<Item = Vec<u8>, Error = hyper::Error>> {
    Box::new(body.fold(Vec::new(), |mut acc, chunk| {
        acc.extend_from_slice(&*chunk);
        future::ok::<_, hyper::Error>(acc)
    }))
}
