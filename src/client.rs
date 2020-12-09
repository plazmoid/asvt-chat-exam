use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::panic;
use std::thread::sleep;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    api::process_command, config::*, db::ClientDB, error::SError, protocol::parse_request,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum CliTask {
    // date, from, msg
    SendMsg(String, String, String),
    Exit,
}

fn try_append_username(uid: Uuid, addr: &SocketAddr) -> String {
    match ClientDB::get_username(uid) {
        Some(n) => format!("{} ({})", addr, n),
        None => format!("{}", addr),
    }
}

pub struct Client {
    conn: TcpStream,
    addr: SocketAddr,
    uid: Uuid,
}

impl Client {
    pub fn handle(stream: TcpStream, addr: SocketAddr) {
        let client_uid = ClientDB::add_client(addr);
        let instance = Client {
            conn: stream,
            addr: addr.clone(),
            uid: client_uid,
        };
        panic::catch_unwind(|| instance._handle_req()).ok();
    }

    fn _handle_req(mut self) {
        let mut silence_counter = 0; // instead of not working with non-blocking sockets timeout
        let mut drop_trigger = false;
        let mut data = [0u8; CMD_BUF_SIZE];
        self.conn
            .set_nonblocking(true)
            .expect("Can't make socket non-blocking");
        // self.conn
        //     .set_read_timeout(Some(Duration::from_secs(SILENT_CONN_TIMEOUT)))
        //     .expect("Can't set timeout");
        loop {
            data.iter_mut().for_each(|e| *e = 0u8);
            let read_result = Read::by_ref(&mut self.conn)
                .take(CMD_BUF_SIZE as u64)
                .read(&mut data);
            match read_result {
                Ok(size) => {
                    silence_counter = 0;
                    if size == 0 {
                        info!(
                            "Connection with {} is closed",
                            try_append_username(self.uid, &self.addr)
                        );
                        break;
                    }
                    if size < CMD_BUF_SIZE {
                        if drop_trigger {
                            drop_trigger = false;
                        }
                    }
                    if drop_trigger {
                        continue;
                    }
                    if size == CMD_BUF_SIZE {
                        drop_trigger = true;
                    }

                    let cmd = String::from_utf8_lossy(&data).into_owned();
                    let cmd = cmd.trim_matches(char::from(0)).trim().to_string();
                    if cmd.len() == 0 {
                        continue;
                    }
                    let _log_msg = format!(
                        "Cmd from {}: {}",
                        try_append_username(self.uid, &self.addr),
                        &cmd
                    );
                    let response = parse_request(&cmd)
                        .map_err(|e| SError::SyntaxError(e.to_string()))
                        .and_then(|(_, c)| process_command(c, self.uid, &self.addr));
                    let response = match response {
                        Ok(resp) => {
                            if cmd.to_lowercase() != "ping" {
                                info!("{}", _log_msg);
                            }
                            format!("{}{}", SUCCESS, resp)
                        }
                        Err(e) => {
                            error!("{} ({})", _log_msg, &e);
                            format!("{}{}", FAIL, e)
                        }
                    };
                    self.send_response(response);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    if silence_counter / (1000 / HALT_MS as usize) >= SILENT_CONN_TIMEOUT {
                        self.send_response(TIMEOUT_MSG);
                        self.shutdown();
                    }
                    sleep(Duration::from_millis(HALT_MS));
                    silence_counter += 1;
                }
                Err(e) => {
                    error!("Error in {} occured: {}", self.addr, e.to_string());
                    break;
                }
            }
            self.apply_jobs();
        }
    }

    fn apply_jobs(&mut self) {
        match ClientDB::get_all_client_jobs(self.uid) {
            Some(jobs) => {
                jobs.into_iter().for_each(|job| match job {
                    CliTask::Exit => {
                        self.send_response(SHUTDOWN_MSG);
                        self.shutdown();
                    }
                    CliTask::SendMsg(date, sender, msg) => {
                        let full_msg = format!(
                            "MSGFROM [{} {}] ({}): {}",
                            date,
                            sender,
                            msg.chars().count(),
                            msg
                        );
                        self.send_response(full_msg)
                    }
                });
            }
            None => (),
        }
    }

    fn send_response<S: Into<String>>(&mut self, data: S) {
        self.conn.write((data.into() + "\n").as_bytes()).ok();
    }

    fn shutdown(&mut self) {
        self.conn.shutdown(Shutdown::Both).ok();
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        if ClientDB::is_logged_in(self.uid) {
            ClientDB::set_online_status(self.uid, false);
        } else {
            ClientDB::remove_cli(self.uid);
        }
    }
}
