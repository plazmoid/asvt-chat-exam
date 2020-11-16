use std::env;
use std::fs::OpenOptions;
use std::net::TcpListener;
use std::thread;

mod utils;
use utils::daemonize;

mod api;
mod client;
mod protocol;

use client::Client;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;
extern crate simplelog;
use simplelog::*;

const PORT: &str = "81";

fn init_logger(show_stderr: bool) {
    let log_cfg = ConfigBuilder::new()
        .set_time_format_str("%x %X")
        .set_time_to_local(true)
        .build();
    let logfile = OpenOptions::new()
        .append(true)
        .create(true)
        .open("pi_server.log")
        .unwrap();
    let mut loggers: Vec<Box<dyn simplelog::SharedLogger>> = vec![WriteLogger::new(
        LevelFilter::Debug,
        log_cfg.clone(),
        logfile,
    )];
    if show_stderr {
        loggers.push(TermLogger::new(
            LevelFilter::Info,
            log_cfg,
            TerminalMode::Stderr,
        ))
    }
    CombinedLogger::init(loggers).unwrap();
}

fn main() {
    let mut is_daemon = false;
    if let Some(arg) = env::args().nth(1) {
        if arg == "-d" {
            if let Ok(pid) = daemonize() {
                is_daemon = true;
                debug!("Forked to background (pid {})", pid);
            }
        }
    }
    init_logger(!is_daemon);
    let listener = TcpListener::bind(format!("0.0.0.0:{}", PORT)).unwrap();
    info!("Listening on port {}", PORT);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                info!("New connection: {}", stream.peer_addr().unwrap());
                thread::spawn(move || Client::handle(stream));
            }
            Err(e) => {
                error!("Error: {}", e);
            }
        }
    }
}
