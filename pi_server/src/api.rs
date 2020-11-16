use std::collections::HashMap;
use std::net::TcpStream;

pub type RResult<T> = std::result::Result<T, String>;
pub type Args<'s> = HashMap<&'s str, &'s str>;
type Handler = fn(Args) -> RResult<String>;

#[derive(PartialEq, Debug)]
pub struct Command<'cmd> {
    pub cmd: &'cmd str,
    pub args: Args<'cmd>,
}

lazy_static! {
    static ref RULES: HashMap<&'static str, (Vec<&'static str>, Handler)> = {
        let mut rules = HashMap::new();
        rules.insert("HELP", (vec![], get_help as Handler));
        rules.insert("PING", (vec![], ping as Handler));
        rules.insert("ECHO", (vec!["MSG"], echo as Handler));
        rules.insert("USERS", (vec![], get_users as Handler));
        /*rules.insert("LOGIN", (vec!["USERNAME"], get_help as Handler));
        rules.insert("SENDTO", (vec!["USERNAME", "MSG"], get_help as Handler));
        rules.insert("SENDALL", (vec!["MSG"], get_help as Handler));
        rules.insert("EXIT", (vec![], get_help as Handler));*/
        rules
    };
}

fn _help() -> String {
    let mut cmds = RULES.keys().map(|k| *k).collect::<Vec<&str>>();
    cmds.sort();
    format!("Available commands: {}", cmds.join(", "))
}

fn get_help(_: Args) -> RResult<String> {
    Ok(_help())
}

fn get_users(_: Args) -> RResult<String> {
    let users = vec!["Dan", "Can", "Ban", "Ian"];
    Ok(users.join("\n"))
}

fn ping(_: Args) -> RResult<String> {
    Ok(String::from("pong"))
}

fn echo(args: Args) -> RResult<String> {
    Ok(args.get("MSG").unwrap().to_string())
}

pub fn process_command(cmd: Command, stream: &TcpStream) -> RResult<String> {
    let (required_args, handler): &(Vec<&str>, Handler) = match RULES.get(cmd.cmd.trim()) {
        Some(m) => m,
        None => return Err(format!("Unknown command. \n{}", _help())),
    };
    let cmd_arg_names = cmd.args.keys().collect::<Vec<&&str>>();
    for argn in required_args.iter() {
        match cmd_arg_names.contains(&argn) {
            true => (),
            false => return Err(format!("Missing args: {}", required_args.join("\n"))),
        }
    }
    handler(cmd.args)
}
