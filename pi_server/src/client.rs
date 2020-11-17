use std::io::{self, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::thread::sleep;
use std::time::Duration;

use crate::api::process_command;
use crate::db::ClientDB;
use crate::protocol::parse_request;

const CMD_BUF_SIZE: usize = 256;
const SILENT_CONN_TIMEOUT: u64 = 40;

#[derive(Debug, Clone)]
pub enum CliJob {
    // from, msg
    SendMsg(String, String),
    Exit,
}

fn try_append_username(addr: &SocketAddr) -> String {
    match ClientDB::get_username(addr) {
        Some(n) => format!("{} ({})", addr, n),
        None => format!("{}", addr),
    }
}

pub struct Client {
    conn: TcpStream,
    addr: SocketAddr,
}

impl Client {
    pub fn handle(stream: TcpStream) {
        let addr = stream.peer_addr().unwrap();
        let instance = Client {
            conn: stream,
            addr: addr.clone(),
        };
        ClientDB::init(addr);
        instance._handle_req()
    }

    fn _handle_req(mut self) {
        let mut drop_trigger = false;
        let mut data = [0u8; CMD_BUF_SIZE];
        self.conn
            .set_nonblocking(true)
            .expect("Can't make socket non-blocking");
        self.conn
            .set_read_timeout(Some(Duration::from_secs(SILENT_CONN_TIMEOUT)))
            .expect("Can't set timeout");
        loop {
            data.iter_mut().for_each(|e| *e = 0u8);
            let read_result = Read::by_ref(&mut self.conn)
                .take(CMD_BUF_SIZE as u64)
                .read(&mut data);
            match read_result {
                Ok(size) => {
                    if size == 0 {
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

                    let cmd = String::from_utf8_lossy(&data).into_owned().to_string();
                    let cmd = cmd.trim_matches(char::from(0)).trim().to_string();
                    if cmd.len() == 0 {
                        continue;
                    }
                    let _log_msg =
                        format!("Cmd from {}: {}", try_append_username(&self.addr), &cmd);
                    let response = parse_request(&cmd).map(|(_, c)| process_command(c, &self.addr));
                    let response = match response {
                        Ok(resp) => match resp {
                            Ok(resp) => {
                                info!("{}", _log_msg);
                                resp
                            }
                            Err(e) => {
                                error!("{}\n{}", _log_msg, &e);
                                format!("Error: {}", e)
                            }
                        },
                        Err(e) => {
                            error!("{}\nSyntax error: {}", _log_msg, &e);
                            format!("Error: Syntax error: {}", e)
                        }
                    };
                    self.send_response(response);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    sleep(Duration::from_millis(10));
                }
                Err(e) => {
                    error!("Error in {} occured: {}", self.addr, e.to_string());
                    self.shutdown();
                }
            }
            self.apply_jobs();
        }
    }

    fn apply_jobs(&mut self) {
        ClientDB::get_all_client_jobs(&self.addr)
            .into_iter()
            .for_each(|job| match job {
                CliJob::Exit => self.shutdown(),
                CliJob::SendMsg(sender, msg) => {
                    let full_msg = format!("Message from {}: {}", sender, msg);
                    self.send_response(full_msg)
                }
            });
    }

    fn send_response(&mut self, data: String) {
        self.conn.write((data + "\n").as_bytes()).ok();
    }

    fn shutdown(&self) {
        self.conn.shutdown(Shutdown::Both).ok();
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        ClientDB::remove_cli(&self.addr);
    }
}
