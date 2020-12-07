use crate::{client::CliTask, config::*, db::ClientDB, error::SError};
use chrono::prelude::*;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

pub type RResult<T> = std::result::Result<T, SError>;
pub type HResult = RResult<HandleResult>;
pub type Args<'s> = HashMap<&'s str, &'s str>;
type Handler = fn(HandleInfo) -> HResult;

#[derive(PartialEq, Debug)]
pub struct Command<'cmd> {
    pub cmd: &'cmd str,
    pub args: Args<'cmd>,
}

pub struct HandleResult(pub String);

impl From<String> for HandleResult {
    fn from(s: String) -> HandleResult {
        HandleResult(s)
    }
}

impl From<&'static str> for HandleResult {
    fn from(s: &'static str) -> HandleResult {
        HandleResult(s.to_string())
    }
}

impl From<()> for HandleResult {
    fn from(_: ()) -> HandleResult {
        HandleResult(String::new())
    }
}

pub struct HandleInfo<'cmd> {
    pub args: Args<'cmd>,
    pub addr: &'cmd SocketAddr,
}

lazy_static! {
    static ref RULES: HashMap<&'static str, (Vec<&'static str>, Handler)> = {
        let mut rules = HashMap::new();
        rules.insert("HELP", (vec![], API::get_help as Handler));
        rules.insert("PING", (vec![], API::ping as Handler));
        rules.insert("ECHO", (vec!["msg"], API::echo as Handler));
        rules.insert("USERS", (vec![], API::get_users as Handler));
        rules.insert(
            "LOGIN",
            (vec!["username", "password"], API::login as Handler),
        );
        rules.insert("SEND", (vec!["username", "msg"], API::send_to as Handler));
        rules.insert("SNDALL", (vec!["msg"], API::send_to_all as Handler));
        rules.insert("EXIT", (vec![], API::cli_exit as Handler));
        rules.insert("_DELUSER", (vec!["username"], API::del_user as Handler));
        rules.insert("_FLUSH", (vec!["username"], API::flush_jobs as Handler));
        rules
    };
    static ref LOGIN_RULE: Regex = Regex::new(r"^[\x20-\x39\x3B-\x7Eа-яёА-ЯЁ]{1,20}$").unwrap();
}

pub struct API;

impl API {
    fn _help() -> String {
        let mut cmds = RULES
            .keys()
            .filter_map(|k| if k.starts_with('_') { None } else { Some(*k) })
            .collect::<Vec<&str>>();
        cmds.sort();
        format!("v. 0.3.6 \nAvailable commands: {}", cmds.join(", "))
    }

    fn check_admin(user: &SocketAddr) -> RResult<()> {
        let caller = ClientDB::get_username(user).unwrap();
        if caller == ADMIN {
            Ok(())
        } else {
            Err(SError::UnknownCommand)
        }
    }

    pub fn flush_jobs(h: HandleInfo) -> HResult {
        Self::check_admin(&h.addr)?;
        let user = h.args.get("username").unwrap().to_string();
        let user_addr = match ClientDB::get_client_by_username(&user) {
            Some(r) => r,
            None => return Err(SError::NoSuchUser),
        };
        let jobs_cnt = match ClientDB::get_all_client_jobs(&user_addr) {
            Some(j) => j.len(),
            None => 0,
        };
        Ok(jobs_cnt.to_string().into())
    }

    pub fn del_user(h: HandleInfo) -> HResult {
        Self::check_admin(&h.addr)?;
        let user = h.args.get("username").unwrap().to_string();
        let user_addr = match ClientDB::get_client_by_username(&user) {
            Some(r) => r,
            None => user.parse().map_err(|_| SError::NoSuchUser)?,
        };
        ClientDB::add_task(&user_addr, CliTask::Exit);
        thread::sleep(Duration::from_secs(1));
        ClientDB::remove_cli(&user_addr);
        Ok(().into())
    }

    pub fn login(h: HandleInfo) -> HResult {
        let username = h.args.get("username").unwrap().to_string();
        let password = h.args.get("password").unwrap().to_string();
        if !LOGIN_RULE.is_match(&username) {
            return Err(SError::InvalidLogin);
        }
        ClientDB::set_login(h.addr, username, password).map(HandleResult::from)
    }

    pub fn get_help(_: HandleInfo) -> HResult {
        Ok(API::_help().into())
    }

    pub fn cli_exit(h: HandleInfo) -> HResult {
        ClientDB::add_task(h.addr, CliTask::Exit).map(HandleResult::from)
    }

    pub fn get_users(h: HandleInfo) -> HResult {
        let mut users = ClientDB::get_all_users(&h.addr);
        users.sort_by(|a, b| {
            if a.ends_with(ONLINE) ^ b.ends_with(ONLINE) {
                if a.ends_with(ONLINE) {
                    Ordering::Less
                } else {
                    if b.ends_with(ONLINE) {
                        Ordering::Greater
                    } else {
                        unreachable!()
                    }
                }
            } else {
                a.cmp(b)
            }
        });
        Ok(users.join("\n").into())
    }

    pub fn send_to_all(h: HandleInfo) -> HResult {
        if !ClientDB::is_logged_in(&h.addr) {
            return Err(SError::NotLoggedIn);
        }
        let sender = match ClientDB::get_username(&h.addr) {
            Some(s) => s,
            None => h.addr.to_string(),
        };
        let sender = sender + " (to all)";
        let message = h.args.get("msg").unwrap().to_string();
        let date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let task = CliTask::SendMsg(date, sender, message);
        ClientDB::add_broadcast_task(h.addr, task).map(HandleResult::from)
    }

    pub fn send_to(h: HandleInfo) -> HResult {
        if !ClientDB::is_logged_in(&h.addr) {
            return Err(SError::NotLoggedIn);
        }
        let receiver = h.args.get("username").unwrap().to_string();
        let receiver = match ClientDB::get_client_by_username(&receiver) {
            Some(r) => r,
            None => return Err(SError::NoSuchUser),
        };
        let sender = match ClientDB::get_username(&h.addr) {
            Some(s) => s,
            None => h.addr.to_string(),
        };
        let message = h.args.get("msg").unwrap().to_string();
        let date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let task = CliTask::SendMsg(date, sender, message);
        ClientDB::add_task(&receiver, task).map(HandleResult::from)
    }

    pub fn ping(_: HandleInfo) -> HResult {
        Ok(().into())
    }

    pub fn echo(h: HandleInfo) -> HResult {
        Ok(h.args.get("msg").unwrap().to_string().into())
    }
}

pub fn process_command(cmd: Command, addr: &SocketAddr) -> RResult<String> {
    let (required_args, handler): &(Vec<&str>, Handler) =
        match RULES.get(cmd.cmd.to_uppercase().trim()) {
            Some(m) => m,
            None => return Err(SError::UnknownCommand),
        };
    ClientDB::check_cmd_timeout(addr, true)?;
    let cmd_arg_names = cmd.args.keys().collect::<Vec<&&str>>();
    for argn in required_args.iter() {
        if !cmd_arg_names.contains(&argn) {
            return Err(SError::WrongArgs(required_args.join(", ")));
        }
    }
    let h_info = HandleInfo {
        args: cmd.args,
        addr: addr,
    };
    handler(h_info).map(|r| r.0)
}
