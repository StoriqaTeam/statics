//! Users is a microservice responsible for authentication and managing user profiles.
//! This create is for running the service from `users_lib`. See `users_lib` for details.

extern crate statics_lib;

fn main() {
    let config = statics_lib::config::Config::new().expect("Can't load app config!");
    statics_lib::start_server(config);
}
