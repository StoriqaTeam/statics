extern crate hyper_tls;
extern crate rand;

use lib;

use self::hyper_tls::HttpsConnector;
use self::rand::Rng;
use hyper::Client;
use hyper::client::HttpConnector;
use std::fs::File;
use std::io::Read;
use std::sync::mpsc::channel;
use std::thread;
use tokio_core::reactor::Core;

type HttpClient = Client<HttpsConnector<HttpConnector>>;

pub struct Context {
    pub client: HttpClient,
    pub base_url: String,
    pub core: Core,
}

pub fn setup() -> Context {
    let _ = lib::log::log_environment().try_init();
    let (tx, rx) = channel::<bool>();
    let mut rng = rand::thread_rng();
    let port = rng.gen_range(50000, 60000);
    thread::spawn({
        let tx = tx.clone();
        move || {
            let config = lib::config::Config::new().expect("Can't load app config!");
            lib::start_server(config, Some(port.to_string()), move || {
                let _ = tx.send(true);
            });
        }
    });
    rx.recv().unwrap();
    let core = Core::new().expect("Unexpected error creating event loop core");
    let client = ::hyper::Client::configure()
        .connector(HttpsConnector::new(4, &core.handle()).unwrap())
        .build(&core.handle());
    Context {
        client,
        base_url: format!("http://localhost:{}", port),
        core,
    }
}

pub fn read_static_file(name: &str) -> Vec<u8> {
    let mut file = File::open(format!("tests/static_files/{}", name)).unwrap();
    let mut buf = Vec::new();
    let _ = file.read_to_end(&mut buf);
    buf
}
