use crate::client::CliTask;
use crate::db::ClientDB;
use crate::error::SError;
use chrono::prelude::*;
use std::collections::HashMap;
use std::net::SocketAddr;

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
        rules
    };
}

pub struct API;

impl API {
    fn _help() -> String {
        let mut cmds = RULES.keys().map(|k| *k).collect::<Vec<&str>>();
        cmds.sort();
        format!("v. 0.3.2 \nAvailable commands: {}", cmds.join(", "))
    }

    pub fn login(h: HandleInfo) -> HResult {
        let username = h.args.get("username").unwrap().to_string();
        let password = h.args.get("password").unwrap().to_string();
        if username.chars().count() > 20 {
            return Err(SError::NameIsTooLong);
        }
        ClientDB::set_login(h.addr, username, password).map(HandleResult::from)
    }

    pub fn get_help(_: HandleInfo) -> HResult {
        Ok(API::_help().into())
    }

    pub fn cli_exit(h: HandleInfo) -> HResult {
        ClientDB::add_task(h.addr, CliTask::Exit, false).map(HandleResult::from)
    }

    pub fn get_users(h: HandleInfo) -> HResult {
        let users = ClientDB::get_all_users(&h.addr);
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
        let receiver = match ClientDB::get_client_by_username(receiver) {
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
        ClientDB::add_task(&receiver, task, true).map(HandleResult::from)
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
