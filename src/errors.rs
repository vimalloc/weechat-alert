use std::io;
use std::fmt;
use std::str::Utf8Error;
use std::error::Error;

/// The error type used by this code base
#[derive(Debug)]
pub enum WeechatError {
    Io(io::Error),  // Errors reading, writing, or connecting to socket
    BadPassword,    // Bad password for weechat init protocol
    ParseError(String),     // Recieved unparsable bytes from a weechat message
}

/// Convert io::Error to WeechatErrors
impl From<io::Error> for WeechatError {
    fn from(err: io::Error) -> WeechatError {
        WeechatError::Io(err)
    }
}

/// Convert io::Error to WeechatErrors
impl From<Utf8Error> for WeechatError {
    fn from(_: Utf8Error) -> WeechatError {
        WeechatError::ParseError("Parsed invalid utf8 string".to_string())
    }
}

/// Display WeechatErrors
impl fmt::Display for WeechatError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            WeechatError::Io(ref err)          => err.fmt(f),
            WeechatError::BadPassword          => write!(f, "Invalid password"),
            WeechatError::ParseError(ref s)    => write!(f, "Parse error: {}", s),
        }
    }
}

/// Error trait for WeechatErrors
impl Error for WeechatError {
    fn description(&self) -> &str {
        match *self {
            WeechatError::Io(ref err)      => err.description(),
            WeechatError::BadPassword      => "Invalid username or password",
            WeechatError::ParseError(_)    => "Message parse error",
        }
    }
}
