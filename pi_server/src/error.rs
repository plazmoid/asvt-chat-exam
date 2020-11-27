use rusqlite::Error as SQLError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SError {
    #[error("Too fast")]
    DOS,

    #[error("Login already exists")]
    LoginAlreadyExists,

    #[error("Name is too long (> 20 chars)")]
    NameIsTooLong,

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

    #[error(transparent)]
    DBError(#[from] SQLError),

    #[error("Syntax error: {}", .0)]
    SyntaxError(String),

    #[error("FIX IT !1!11ONE")]
    FixIt,
}
