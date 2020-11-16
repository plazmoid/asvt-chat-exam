use std::collections::HashMap;
use std::net::{Shutdown, TcpStream};

#[derive(PartialEq, Debug)]
pub struct Command {
    pub cmd: String,
    pub args: Vec<(String, String)>,
}

pub type RResult<T> = std::result::Result<T, String>;

type Handler = fn(Command) -> RResult<String>;

lazy_static! {
    static ref RULES: HashMap<&'static str, (Vec<&'static str>, Handler)> = {
        let mut rules = HashMap::new();
        rules.insert("HELP", (vec![], get_help as Handler));
        rules.insert("PING", (vec![], ping as Handler));
        rules.insert("ECHO", (vec!["MSG"], echo as Handler));
        rules.insert("USERS", (vec![], get_help as Handler));
        rules.insert("LOGIN", (vec!["USERNAME"], get_help as Handler));
        rules.insert("SENDTO", (vec!["USERNAME", "MSG"], get_help as Handler));
        rules.insert("SENDALL", (vec!["MSG"], get_help as Handler));
        rules.insert("EXIT", (vec![], get_help as Handler));
        rules
    };
}

fn get_users() -> RResult<String> {
    let users = vec!["Dan", "Can", "Ban", "Ian"];
    Ok(users.join("\n"))
}

fn get_help(_: Command) -> RResult<String> {
    let msg = "Available commands:
ECHO <msg>
HELP
PING
USERS
";
    Ok(msg.to_string())
}

fn ping(_: Command) -> RResult<String> {
    Ok(String::new())
}

fn echo(cmd: Command) -> RResult<String> {
    cmd.
}

pub fn process_command(cmd: Command, stream: &TcpStream) -> RResult<String> {
    Ok(String::from("Ok"))
}
