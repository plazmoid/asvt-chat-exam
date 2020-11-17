use crate::client::CliJob;
use crate::db::ClientDB;
use std::collections::HashMap;
use std::net::SocketAddr;

pub type RResult<T> = std::result::Result<T, String>;
pub type Args<'s> = HashMap<&'s str, &'s str>;
type Handler = fn(HandleInfo) -> RResult<String>;

#[derive(PartialEq, Debug)]
pub struct Command<'cmd> {
    pub cmd: &'cmd str,
    pub args: Args<'cmd>,
}

struct HandleInfo<'cmd> {
    pub args: Args<'cmd>,
    pub addr: &'cmd SocketAddr,
}

lazy_static! {
    static ref RULES: HashMap<&'static str, (Vec<&'static str>, Handler)> = {
        let mut rules = HashMap::new();
        rules.insert("HELP", (vec![], get_help as Handler));
        rules.insert("PING", (vec![], ping as Handler));
        rules.insert("ECHO", (vec!["msg"], echo as Handler));
        rules.insert("USERS", (vec![], get_users as Handler));
        rules.insert("LOGIN", (vec!["username"], login as Handler));
        rules.insert("SENDTO", (vec!["username", "msg"], send_to as Handler));
        rules.insert("SENDALL", (vec!["msg"], send_to_all as Handler));
        rules.insert("EXIT", (vec![], cli_exit as Handler));
        rules
    };
}

fn _help() -> String {
    let mut cmds = RULES.keys().map(|k| *k).collect::<Vec<&str>>();
    cmds.sort();
    format!("Available commands: {}", cmds.join(", "))
}

fn login(h: HandleInfo) -> RResult<String> {
    let login = h.args.get("username").unwrap();
    ClientDB::set_login(h.addr, login.to_string()).map(|_| format!("Now you are {}", login))
}

fn get_help(_: HandleInfo) -> RResult<String> {
    Ok(_help())
}

fn cli_exit(h: HandleInfo) -> RResult<String> {
    ClientDB::add_job(h.addr, CliJob::Exit).map(|_| "Bye".to_string())
}

fn get_users(h: HandleInfo) -> RResult<String> {
    let users = ClientDB::get_all_users(h.addr);
    Ok(users.join("\n"))
}

fn send_to_all(h: HandleInfo) -> RResult<String> {
    let sender = match ClientDB::get_username(h.addr) {
        Some(s) => s,
        None => h.addr.to_string(),
    };
    let sender = sender + " (to all)";
    let message = h.args.get("msg").unwrap().to_string();
    ClientDB::add_broadcast_job(h.addr, CliJob::SendMsg(sender, message))
        .map(|_| "Sent!".to_string())
}

fn send_to(h: HandleInfo) -> RResult<String> {
    let receiver = h.args.get("username").unwrap().to_string();
    let receiver = match ClientDB::get_client_by_username(receiver) {
        Some(r) => r,
        None => return Err("No such user".to_string()),
    };
    let message = h.args.get("msg").unwrap().to_string();
    let sender = match ClientDB::get_username(h.addr) {
        Some(s) => s,
        None => h.addr.to_string(),
    };
    let job = CliJob::SendMsg(sender, message);
    ClientDB::add_job(&receiver, job).map(|_| "Sent!".to_string())
}

fn ping(_: HandleInfo) -> RResult<String> {
    Ok(String::from("pong"))
}

fn echo(h: HandleInfo) -> RResult<String> {
    Ok(h.args.get("msg").unwrap().to_string())
}

pub fn process_command(cmd: Command, addr: &SocketAddr) -> RResult<String> {
    let (required_args, handler): &(Vec<&str>, Handler) =
        match RULES.get(cmd.cmd.to_uppercase().trim()) {
            Some(m) => m,
            None => return Err(format!("Unknown command")),
        };
    let cmd_arg_names = cmd.args.keys().collect::<Vec<&&str>>();
    for argn in required_args.iter() {
        match cmd_arg_names.contains(&argn) {
            true => (),
            false => return Err(format!("Missing args: {}", required_args.join(", "))),
        }
    }
    let h_info = HandleInfo {
        args: cmd.args,
        addr,
    };
    handler(h_info)
}
