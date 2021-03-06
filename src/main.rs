#![allow(unused_must_use)]
use std::env;
use std::fs::OpenOptions;
use std::net::TcpListener;
use std::panic;
use std::process;
use std::thread;

mod api;
mod client;
mod config;
mod db;
mod error;
mod protocol;
mod utils;

use client::Client;
use config::*;
use db::ClientDB;
use utils::daemonize;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;
extern crate simplelog;
use signal_hook::{iterator::Signals, SIGHUP, SIGINT, SIGTERM};
use simplelog::*;

fn init_logger(show_stderr: bool) {
    let log_cfg = ConfigBuilder::new()
        .set_time_format_str("%x %X")
        .set_time_to_local(true)
        .build();
    let logfile = OpenOptions::new()
        .append(true)
        .create(true)
        .open(LOGFILE)
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

fn init_statics() {
    ClientDB::_lock_read();
}

fn init_sighandlers() {
    let signals = Signals::new(&[SIGINT, SIGTERM, SIGHUP]).unwrap();
    thread::spawn(move || {
        for sig in signals.forever() {
            match sig {
                SIGINT | SIGTERM => {
                    info!("Gracefully stopping...");
                    info!("Syncing db");
                    ClientDB::sync_db();
                    info!("Done");
                    process::exit(0);
                }
                SIGHUP => {
                    info!("SIGHUP received");
                    info!("Syncing db");
                    ClientDB::sync_db();
                    info!("Done");
                }
                _ => unreachable!(),
            }
        }
    });
}

fn set_panic_hook() {
    panic::set_hook(Box::new(|info| error!("Critical: {}", info)));
}

fn listen() {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", PORT)).unwrap();
    info!("Listening on port {}", PORT);
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let addr = match stream.peer_addr() {
                    Ok(a) => a,
                    Err(_) => continue,
                };
                info!("New connection: {}", &addr);
                thread::spawn(move || Client::handle(stream, addr));
            }
            Err(e) => {
                error!("Error: {}", e);
            }
        }
    }
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
    set_panic_hook();
    init_statics();
    init_sighandlers();
    listen();
}
