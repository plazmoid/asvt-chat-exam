use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::api::process_command;
use crate::protocol::parse_request;

const CMD_BUF_SIZE: usize = 256;
const SILENT_CONN_TIMEOUT: u64 = 40;

lazy_static! {
    static ref DB: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));
}

pub struct Client {
    name: Option<String>,
    conn: TcpStream,
}

impl Client {
    pub fn handle(stream: TcpStream) {
        let instance = Client {
            name: None,
            conn: stream,
        };
        instance._handle_req()
    }

    fn _handle_req(mut self) {
        let mut drop_trigger = false;
        let mut data = [0u8; CMD_BUF_SIZE];
        self.conn
            .set_read_timeout(Some(Duration::from_secs(SILENT_CONN_TIMEOUT)))
            .expect("Can't set timeout");
        //send_response(&mut stream, HELP_MSG.to_string());
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
                    info!("Cmd from {}: {}", self.conn.peer_addr().unwrap(), &cmd);
                    let response = parse_request(&cmd).map(|(_, c)| process_command(c, &self.conn));
                    let response = match response {
                        Ok(resp) => match resp {
                            Ok(resp) => resp,
                            Err(e) => format!("Error: {}", e),
                        },
                        Err(e) => format!("Error: Syntax error in '{}': {}", cmd, e.to_string()),
                    };
                    self.send_response(response);
                }
                Err(e) => {
                    error!(
                        "Error in {} occured: {}",
                        self.conn.peer_addr().unwrap(),
                        e.to_string()
                    );
                    self.shutdown();
                }
            }
        }
    }

    pub fn send_response(&mut self, data: String) {
        self.conn.write((data + "\n").as_bytes()).ok();
    }

    pub fn shutdown(&self) {
        self.conn.shutdown(Shutdown::Both).ok();
    }
}
