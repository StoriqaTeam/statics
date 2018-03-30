use statics_lib;

use std::thread;
use std::time;
use hyper::{Client};
use hyper::client::HttpConnector;
use tokio_core::reactor::Core;

type HttpClient = Client<HttpConnector>;

pub struct Context {
    pub client: HttpClient,
    pub base_url: String,
    pub core: Core,
}

pub fn setup() -> Context {
    thread::spawn(|| {
        let config = statics_lib::config::Config::new().expect("Can't load app config!");
        statics_lib::start_server(config);
    });
    thread::sleep(time::Duration::from_millis(1000));
    let core = Core::new().expect("Unexpected error creating event loop core");
    let client = Client::new(&core.handle());
    Context { client, base_url: "http://localhost:8000".to_string(), core }
}
