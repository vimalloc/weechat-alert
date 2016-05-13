use std::io;
use std::fmt;
use std::error::Error;

/// The error type used by this code base
#[derive(Debug)]
pub enum WeechatError {
    Io(io::Error),  // Errors reading, writing, or connecting to socket
    BadPassword,    // Bad password for weechat init protocol
    NoDataHandler(String),  // Received data we don't know how to deal with
}

/// Convert io::Error to WeechatErrors
impl From<io::Error> for WeechatError {
    fn from(err: io::Error) -> WeechatError {
        WeechatError::Io(err)
    }
}

/// Display WeechatErrors
impl fmt::Display for WeechatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WeechatError::Io(ref err)          => err.fmt(f),
            WeechatError::BadPassword          => write!(f, "Invalid password"),
            WeechatError::NoDataHandler(ref s) => write!(f, "No handler found for {}", s)
        }
    }
}

/// Error trait for WeechatErrors
impl Error for WeechatError {
    fn description(&self) -> &str {
        match *self {
            WeechatError::Io(ref err)      => err.description(),
            WeechatError::BadPassword      => "Invalid username or password",
            WeechatError::NoDataHandler(_) => "No handler found"
        }
    }
}
