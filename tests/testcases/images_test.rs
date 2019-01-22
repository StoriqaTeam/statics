use futures::future;
use futures::future::Future;
use futures::Stream;
use futures_timer::FutureExt;
use hyper::client::HttpConnector;
use hyper::header::{Authorization, Bearer, ContentLength, ContentType};
use hyper::Client;
use hyper::StatusCode;
use hyper::{Method, Request, Uri};
use hyper_tls::HttpsConnector;
use std::str::FromStr;
use stq_http::request_util::read_body;
use tokio_core::reactor::Core;

type HttpClient = Client<HttpsConnector<HttpConnector>>;

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
    fn test(self, base_url: &str, core: &mut Core, client: &HttpClient) {
        let original_filename = &self.original_filename.unwrap_or("image-328x228.png".to_string());
        let original_bytes = super::common::read_static_file(original_filename);
        let mut body = Vec::new();
        body.extend(
            b"-----------------------------2132006148186267924133397521\r\nContent-Disposition: form-data; name=\"file\"; filename=\""
                .into_iter(),
        );
        body.extend(original_filename.clone().into_bytes().into_iter());
        body.extend(b"\nContent-Type: ".into_iter());
        body.extend(self.content_type.unwrap_or("image/png".to_string()).into_bytes().into_iter());
        body.extend(b"\r\n\r\n".into_iter());
        body.extend(original_bytes);
        body.extend(b"\r\n-----------------------------2132006148186267924133397521--\r\n".into_iter());
        let boundary = self
            .boundary
            .unwrap_or("---------------------------2132006148186267924133397521".to_string());
        let mime = format!("multipart/form-data; boundary={}", &boundary)
            .parse::<mime::Mime>()
            .unwrap();
        let url = Uri::from_str(&format!("{}/images", base_url)).unwrap();
        let mut req = Request::new(Method::Post, url);
        req.headers_mut().set(Authorization::<Bearer>(Bearer {
            token: self.jwt_token.unwrap_or("eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiJ9.eyJ1c2VyX2lkIjozLCJleHAiOjE1MjQyMjU4OTh9.O0OtQXgAJtgEgJ2luvQJWJBu1qWVafUvyk5dxMmr-1Nrcgk_IoIllQm1p_lY4j2VnWHdQGjHKTZgN6YmmnEDtcPaKQX7nsF73r378f3bIEnenwdMiqzNjwSgdG-Ke9WLzY3oOsbbjuIs5wv2FQvygvydzDzfYAg_BM02rRmDQSR6bRsHayjL2c9kV2ImGRJynjSQgwDSTubu3NnJmUHf66F5XtsC8aYCxBWJKSkNOXYNIF1oqw-59MmV3QppwEfICuaQQyGif_gxBAoXVonQGPByhI74lk-3rS5f6O2Yr09fUr0WyqkIgsKUXJC_JQwPbf7OWMDNLOdV2aKirpLraQ".into()),
        }));
        req.headers_mut().set(ContentType(mime));
        req.headers_mut()
            .set(ContentLength(self.content_length.unwrap_or(body.len() as u64)));
        req.set_body(body);

        let timeout = std::time::Duration::from_secs(10);

        println!("Sending request");
        let response = core.run(client.request(req).timeout(timeout)).unwrap();
        println!("Received response: {:?}", response);
        let status = response.status();
        let body = core.run(read_body(response.body())).unwrap();
        println!("... with body {:?}", body);

        assert_eq!(status, self.response_status.unwrap_or(StatusCode::Ok));

        if status == StatusCode::Ok {
            let url = serde_json::from_str::<UrlResponse>(&body).unwrap().url;
            let futures: Vec<_> = ["original", "thumb", "small", "medium", "large"]
                .into_iter()
                .map(|size| {
                    fetch_image_from_s3_and_file(&client, original_filename, &url, size).map(|(local, remote)| {
                        assert_eq!(local, remote);
                    })
                })
                .collect();
            core.run(future::join_all(futures)).unwrap();
        }
    }
}

#[ignore]
#[test]
fn test_services() {
    let base_url = super::common::setup();

    let mut core = Core::new().expect("Unexpected error creating event loop core");
    let client = ::hyper::Client::configure()
        .connector(HttpsConnector::new(4, &core.handle()).unwrap())
        .build(&core.handle());

    println!("Testing happy path");
    UploadTester { ..Default::default() }.test(&base_url, &mut core, &client);

    println!("Testing invalid token");
    UploadTester {
        jwt_token: Some("hello".into()),
        response_status: Some(StatusCode::BadRequest),
        ..Default::default()
    }
    .test(&base_url, &mut core, &client);

    println!("Testing invalid boundary");
    UploadTester {
        boundary: Some("abeceda".into()),
        response_status: Some(StatusCode::UnprocessableEntity),
        ..Default::default()
    }
    .test(&base_url, &mut core, &client);

    println!("Testing invalid content type");
    UploadTester {
        content_type: Some("image/svg".into()),
        response_status: Some(StatusCode::UnprocessableEntity),
        ..Default::default()
    }
    .test(&base_url, &mut core, &client);
}

fn fetch_image_from_s3_and_file(
    client: &HttpClient,
    filename: &str,
    url: &str,
    size: &str,
) -> Box<Future<Item = (Vec<u8>, Vec<u8>), Error = hyper::Error>> {
    let filename = add_size_to_url(filename, size);
    let url = add_size_to_url(url, size);
    let uri = Uri::from_str(&url).unwrap();
    Box::new(client.get(uri).and_then(|resp| read_bytes(resp.body())).map(move |remote_bytes| {
        let local_bytes = super::common::read_static_file(&filename);
        (remote_bytes, local_bytes)
    }))
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
