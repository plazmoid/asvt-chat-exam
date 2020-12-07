use thiserror::Error;

#[derive(Error, Debug)]
pub enum SError {
    #[error("Already logged in")]
    AlreadyLoggedIn,

    #[error("Too fast")]
    DOS,

    #[error("Login already exists")]
    LoginAlreadyExists,

    #[error(
        "Invalid login: only printable ascii/rus chars allowed, ':' is forbidden, 20 chars at max"
    )]
    InvalidLogin,

    #[error("Please log in")]
    NotLoggedIn,

    #[error("No such user")]
    NoSuchUser,

    #[error("Unknown command")]
    UnknownCommand,

    #[error("Required args: {}", .0)]
    WrongArgs(String),

    #[error("Wrong password")]
    WrongPassword,

    #[error("Syntax error: {}", .0)]
    SyntaxError(String),
}
