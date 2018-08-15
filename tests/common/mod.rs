extern crate rand;
extern crate std;

use lib;

use self::rand::Rng;
use std::fs::File;
use std::io::Read;
use std::sync::mpsc::channel;
use std::thread;

pub fn setup() -> String {
    let (tx, rx) = channel::<bool>();
    let mut rng = rand::thread_rng();
    let port = rng.gen_range(50000, 60000);
    thread::spawn({
        let tx = tx.clone();
        move || {
            let config = lib::Config::new().expect("Can't load app config!");
            lib::start_server(config, Some(port), move || {
                let _ = tx.send(true);
            });
        }
    });
    rx.recv().unwrap();

    format!("http://localhost:{}", port)
}

pub fn read_static_file(name: &str) -> Vec<u8> {
    let mut file = File::open(format!("tests/static_files/{}", name)).unwrap();
    let mut buf = Vec::new();
    let _ = file.read_to_end(&mut buf);
    buf
}
