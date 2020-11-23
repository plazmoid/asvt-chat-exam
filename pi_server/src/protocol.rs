use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_while},
    combinator::eof,
    error::VerboseError,
    multi::separated_list0,
    sequence::separated_pair,
    IResult,
};
use std::collections::HashMap;

use crate::api::{Args, Command};

const SEP: &str = "|";

type Data = str;
type IVerbResult<Left, Parsed> = IResult<Left, Parsed, VerboseError<Left>>;

fn is_alpha(c: char) -> bool {
    let chr = c as u8;
    (chr >= 0x41 && chr <= 0x5A) || (chr >= 0x61 && chr <= 0x7A)
}

fn parse_args(s: &Data) -> IVerbResult<&Data, Vec<(&Data, &Data)>> {
    let arg_line = separated_pair(take_while(is_alpha), tag("="), is_not(SEP));
    separated_list0(tag(SEP), arg_line)(s)
}

pub fn parse_request(s: &Data) -> IVerbResult<&Data, Command> {
    let (s, cmd) = is_not(SEP)(s)?;
    let (s, separator) = alt((tag(SEP), eof))(s)?;
    let (s, args) = if separator == SEP {
        parse_args(s)?
    } else {
        (s, vec![])
    };
    let args: Args = {
        let mut _args = HashMap::new();
        args.into_iter().for_each(|(k, v)| {
            _args.insert(k.trim(), v);
        });
        _args
    };
    let command = Command { cmd, args };
    Ok((s, command))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_command_parse() {
        let cmd = "SENDALL|MSG=qwe|TO=asde zxc";
        let expected = Command {
            cmd: "SENDALL",
            args: {
                let mut args: HashMap<&str, &str> = HashMap::new();
                args.insert("MSG", "qwe");
                args.insert("TO", "asde zxc");
                args
            },
        };
        let (_, result) = parse_request(cmd).unwrap();
        assert_eq!(expected, result);
    }
}
