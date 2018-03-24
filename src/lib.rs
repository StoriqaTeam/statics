//! Statics is a microservice responsible for uploading different static assets like images, videos, etc.
//! The layered structure of the app is
//!
//! `Application -> Controller -> Service -> HttpClient`
//!
//! Currently available routes:
//!
//! - `GET /healthcheck` - returns `"ok"` if the server is live
//! - `POST /images` - accepts multipart HTTP requests with `png` / `jpeg` images.
//! Returns `{"url": <url of uploaded image>}`. You can also use prefix with this url
//! to get different sizes: thumb - 40 pixels, small - 80 pixels, medium - 320 pixels,
//! large - 640 pixels. Example: `https://s3.amazonaws.com/storiqa-dev/img-2IpSsAjuxB8C.png` is original image,
//! `https://s3.amazonaws.com/storiqa-dev/img-2IpSsAjuxB8C-large.png` is large image.


extern crate base64;
extern crate config as config_crate;
extern crate env_logger;
#[macro_use]
extern crate failure;
extern crate futures;
extern crate futures_cpupool;
extern crate hyper;
extern crate hyper_tls;
extern crate image;
extern crate jsonwebtoken;
#[macro_use]
extern crate log;
extern crate mime;
extern crate multipart;
extern crate rand;
extern crate rusoto_core;
extern crate rusoto_s3;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate stq_http;
extern crate stq_router;
extern crate tokio_core;

pub mod config;
pub mod controller;
pub mod services;

use std::sync::Arc;
use std::process;

use futures::{Future, Stream};
use futures::future;
// use futures_cpupool::CpuPool;
use hyper::server::Http;
use tokio_core::reactor::Core;

use stq_http::client::Config as HttpConfig;
use stq_http::controller::Application;

use config::Config;
use services::s3::S3;

/// Starts new web service from provided `Config`
pub fn start_server(config: Config) {
    // Prepare logger
    env_logger::init().unwrap();

    // Prepare reactor
    let mut core = Core::new().expect("Unexpected error creating event loop core");
    let handle = Arc::new(core.handle());

    let http_config = HttpConfig {
        http_client_retries: config.client.http_client_retries,
        http_client_buffer_size: config.client.http_client_buffer_size,
    };
    let client = stq_http::client::Client::new(&http_config, &handle);
    let client_handle = client.handle();
    let client_stream = client.stream();
    handle.spawn(client_stream.for_each(|_| Ok(())));

    let s3 = Arc::new(
        S3::new(
            &config.s3.key,
            &config.s3.secret,
            &config.s3.bucket,
            &handle,
        ).unwrap(),
    );

    // Prepare server
    let address = config
        .server
        .address
        .parse()
        .expect("Address must be set in configuration");

    // Prepare CPU pool
    // let cpu_pool = CpuPool::new(thread_count);

    let serve = Http::new()
        .serve_addr_handle(&address, &handle, move || {
            let controller = Box::new(controller::ControllerImpl::new(
                config.clone(),
                client_handle.clone(),
                s3.clone(),
            ));

            // Prepare application
            let app = Application { controller };

            Ok(app)
        })
        .unwrap_or_else(|why| {
            error!("Http Server Initialization Error: {}", why);
            process::exit(1);
        });

    let handle_arc2 = handle.clone();
    handle.spawn(
        serve
            .for_each(move |conn| {
                handle_arc2.spawn(
                    conn.map(|_| ())
                        .map_err(|why| error!("Server Error: {:?}", why)),
                );
                Ok(())
            })
            .map_err(|_| ()),
    );

    info!("Listening on http://{}", address);
    core.run(future::empty::<(), ()>()).unwrap();
}
