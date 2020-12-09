use crate::{client::CliTask, config::*, db::ClientDB, error::SError};
use chrono::prelude::*;
use regex::Regex;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;
use uuid::Uuid;

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
    pub uid: Uuid,
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
        format!(
            "v. {} \nAvailable commands: {}",
            env!("CARGO_PKG_VERSION"),
            cmds.join(", ")
        )
    }

    fn check_admin(uid: Uuid) -> RResult<()> {
        let caller = match ClientDB::get_username(uid) {
            Some(c) => c,
            None => return Err(SError::NoSuchUser),
        };
        if caller == ADMIN {
            Ok(())
        } else {
            Err(SError::UnknownCommand)
        }
    }

    fn check_login(uid: Uuid) -> RResult<()> {
        if !ClientDB::is_logged_in(uid) {
            Err(SError::NotLoggedIn)
        } else {
            Ok(())
        }
    }

    pub fn flush_jobs(h: HandleInfo) -> HResult {
        Self::check_login(h.uid).and(Self::check_admin(h.uid))?;
        let user = h.args.get("username").unwrap().to_string();
        let uid = match ClientDB::get_client_by_username(&user) {
            Some(r) => r,
            None => return Err(SError::NoSuchUser),
        };
        let jobs_cnt = match ClientDB::get_all_client_jobs(uid) {
            Some(j) => j.len(),
            None => 0,
        };
        Ok(jobs_cnt.to_string().into())
    }

    pub fn del_user(h: HandleInfo) -> HResult {
        Self::check_login(h.uid).and(Self::check_admin(h.uid))?;
        let user = h.args.get("username").unwrap().to_string();
        let uid = match ClientDB::get_client_by_username(&user) {
            Some(r) => r,
            None => match ClientDB::get_uid(&user.parse().map_err(|_| SError::NoSuchUser)?) {
                Some(u) => u,
                None => return Err(SError::NoSuchUser),
            },
        };
        ClientDB::add_task(uid, CliTask::Exit);
        thread::sleep(Duration::from_secs(1));
        ClientDB::remove_cli(uid);
        Ok(().into())
    }

    pub fn login(h: HandleInfo) -> HResult {
        let username = h.args.get("username").unwrap().to_string();
        let password = h.args.get("password").unwrap().to_string();
        if !LOGIN_RULE.is_match(&username) {
            return Err(SError::InvalidLogin);
        }
        ClientDB::set_login(h.uid, h.addr, username, password).map(HandleResult::from)
    }

    pub fn get_help(_: HandleInfo) -> HResult {
        Ok(API::_help().into())
    }

    pub fn cli_exit(h: HandleInfo) -> HResult {
        ClientDB::add_task(h.uid, CliTask::Exit).map(HandleResult::from)
    }

    pub fn get_users(h: HandleInfo) -> HResult {
        let mut users = ClientDB::get_all_users(h.uid);
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
        Self::check_login(h.uid)?;
        let sender = match ClientDB::get_username(h.uid) {
            Some(s) => s,
            None => h.addr.to_string(),
        };
        let sender = sender + " (to all)";
        let message = h.args.get("msg").unwrap().to_string();
        let date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let task = CliTask::SendMsg(date, sender, message);
        ClientDB::add_broadcast_task(h.uid, task).map(HandleResult::from)
    }

    pub fn send_to(h: HandleInfo) -> HResult {
        Self::check_login(h.uid)?;
        let receiver = h.args.get("username").unwrap().to_string();
        let receiver = match ClientDB::get_client_by_username(&receiver) {
            Some(r) => r,
            None => return Err(SError::NoSuchUser),
        };
        let sender = match ClientDB::get_username(h.uid) {
            Some(s) => s,
            None => h.addr.to_string(),
        };
        let message = h.args.get("msg").unwrap().to_string();
        let date = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let task = CliTask::SendMsg(date, sender, message);
        ClientDB::add_task(receiver, task).map(HandleResult::from)
    }

    pub fn ping(_: HandleInfo) -> HResult {
        Ok(().into())
    }

    pub fn echo(h: HandleInfo) -> HResult {
        Ok(h.args.get("msg").unwrap().to_string().into())
    }
}

pub fn process_command(cmd: Command, uid: Uuid, addr: &SocketAddr) -> RResult<String> {
    let (required_args, handler): &(Vec<&str>, Handler) =
        match RULES.get(cmd.cmd.to_uppercase().trim()) {
            Some(m) => m,
            None => return Err(SError::UnknownCommand),
        };
    ClientDB::check_cmd_timeout(uid)?;
    let cmd_arg_names = cmd.args.keys().collect::<Vec<&&str>>();
    for argn in required_args.iter() {
        if !cmd_arg_names.contains(&argn) {
            return Err(SError::WrongArgs(required_args.join(", ")));
        }
    }
    let h_info = HandleInfo {
        args: cmd.args,
        addr,
        uid,
    };
    handler(h_info).map(|r| r.0)
}
