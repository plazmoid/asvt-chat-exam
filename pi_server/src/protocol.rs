use nom::{
    branch,
    bytes::complete::{is_not, tag, take_while},
    error::context,
    multi::separated_list0,
    sequence::separated_pair,
    IResult,
};

use crate::api::Command;

const SEP: &str = "|";
type Data = str;

fn is_alpha(c: char) -> bool {
    let chr = c as u8;
    (chr >= 0x41 && chr <= 0x5A) || (chr >= 0x61 && chr <= 0x7A)
}

fn parse_args(s: &Data) -> IResult<&Data, Vec<(&Data, &Data)>> {
    let arg_line = separated_pair(take_while(is_alpha), tag("="), is_not(SEP));
    separated_list0(tag(SEP), arg_line)(s)
}

fn parse_command(s: &Data) -> IResult<&Data, &Data> {
    take_while(is_alpha)(s)
}

pub fn parse_request(s: &Data) -> IResult<&Data, Command> {
    let (s, cmd) = parse_command(&s)?;
    let (s, _) = context("Delim", tag(SEP))(s)?;
    let (s, args) = parse_args(s)?;
    let command = Command {
        cmd: String::from(cmd),
        args: args
            .into_iter()
            .map(|(k, v)| (String::from(k), String::from(v)))
            .collect(),
    };
    Ok((s, command))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_command_parse() {
        let cmd = "SENDALL|MSG=qwe|TO=asde zxc";
        let expected = Command {
            cmd: "SENDALL".to_string(),
            args: vec![
                ("MSG".to_string(), "qwe".to_string()),
                ("TO".to_string(), "asde zxc".to_string()),
            ],
        };
        let (_, result) = parse_request(cmd).unwrap();
        assert_eq!(expected, result);
    }
}
