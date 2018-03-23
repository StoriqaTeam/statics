//! Statics is a microservice responsible for uploading different static assets like images, videos, etc.
//! The layered structure of the app is
//!
//! `Application -> Controller -> Service -> HttpClient`
//!
//! Currently available routes:
//!
//! - `GET /healthcheck` - returns `"ok"` if the server is live
//! - `POST /images` - accepts multipart HTTP requests with `png` / `jpeg` images

extern crate statics_lib;

fn main() {
    let config = statics_lib::config::Config::new().expect("Can't load app config!");
    statics_lib::start_server(config);
}
