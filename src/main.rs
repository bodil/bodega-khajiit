#[macro_use]
extern crate log;
extern crate dotenv;
extern crate egg_mode;
extern crate env_logger;
extern crate futures;
extern crate hyper;
extern crate hyper_rustls;
extern crate image;
extern crate postgres;
extern crate rusttype;
extern crate tokio_core;

use dotenv::dotenv;

mod config;
mod http;
mod process;
mod render;
mod state;
mod web;

fn main() {
    dotenv().ok();
    env_logger::init();

    loop {
        match process::main() {
            Err(err) => error!("{:?}", err),
            _ => {
                trace!("Process exited without incident.");
                break;
            }
        }
        warn!("Restarting process.");
    }
}
