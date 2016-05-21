use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::from_utf8;
use std::collections::HashMap;

use hdata::HData;
use errors::WeechatError;
use errors::WeechatError::ParseError;
use parse::Parse;
use strdata::StrData;


/// Holds header information for data received from relay
#[derive(Debug)]
pub struct Header {
    /// Size of the message body (not including header size)
    pub length: usize,
    /// Flag if zlib compression is enabled
    pub compression: bool,
}

impl Header {
    /// Takes a new message received by the relay, and parses out the header for it
    ///
    /// The header protocol has the first 4 bytes make an integer which is the,
    /// total size of the message, and a single byte which represents if zlib
    /// compression is enabled for the rest of the message
    pub fn new(bytes: &[u8]) -> Result<Header, WeechatError> {
        let mut cur_pos = 0; // Rolling counter of where we are in the byte array

        // Grab the message length
        let parsed = try!(Parse::integer(bytes));
        let total_msg_length = try!(parsed.object.as_integer());
        cur_pos += parsed.bytes_read;

        // Grab the compression character
        let parsed = try!(Parse::character(&bytes[cur_pos..]));
        let compression = try!(parsed.object.as_character());
        let compression = match compression as u8 {
            0 => false,
            1 => true,
            _ => return Err(WeechatError::ParseError("Bad compression byte".to_string())),
        };
        cur_pos += parsed.bytes_read;

        // Headers has length of full message, we need to chop off the
        // legth of the header as we have already read that from the socket
        let length = total_msg_length as usize - cur_pos;

        // Create the struct
        Ok(Header {
            length: length,
            compression: compression,
        })
    }
}

/// Message received from weechat
#[derive(Debug)]
pub struct Message {
    /// Identifier of the message. For a complete list of identifiers, see:
    /// https://weechat.org/files/doc/devel/weechat_relay_protocol.en.html#message_identifier
    pub identifier: String,
    /// Data contained in this message
    data_type: Type,
}

/// Possible types of messages received from relay (almost every message, excluding pongs,
/// will use HData)
#[derive(Debug)]
pub enum Type {
    StrData(StrData),
    HData(HData),
}

impl Message {
    pub fn new(bytes: &[u8]) -> Result<Message, WeechatError> {
        // First thing encoded is the identifier for what this command is
        let parsed = try!(Parse::string(bytes));
        let identifier = try!(parsed.object.as_not_null_str());

        // Next 3 bytes determin type of data in this command (hdata or str).
        let start = parsed.bytes_read;
        let end = start + 3;
        let msg_type = match try!(from_utf8(&bytes[start..end])) {
            "str" => Type::StrData(try!(StrData::new(&bytes[end..]))),
            "hda" => Type::HData(try!(HData::new(&bytes[end..]))),
            _ => return Err(WeechatError::ParseError("Unknown message type".to_string())),
        };

        // Return our struct
        Ok(Message {
            identifier: String::from(identifier),
            data_type: msg_type,
        })
    }

    /// Returns the contents of this message as an HData (if it is an HData)
    pub fn as_hdata(&self) -> Result<&HData, WeechatError> {
        match self.data_type {
            Type::HData(ref hdata) => Ok(hdata),
            _                      => Err(ParseError("Message is not an hdata".to_string())),
        }
    }

    /// Returns the contents of this message as a StrData (if it is a StrData)
    pub fn as_strdata(&self) -> Result<&StrData, WeechatError> {
        match self.data_type {
            Type::StrData(ref strdata) => Ok(strdata),
            _                          => Err(ParseError("Message is not a strdata".to_string())),
        }
    }
}

/// All possible types of data that can be returned from a weechat message
/// See: https://weechat.org/files/doc/devel/weechat_relay_protocol.en.html#objects
#[derive(Debug, PartialEq, Eq)]
pub enum Object {
    Arr(Vec<Object>),
    Buf(Option<Vec<u8>>),
    Chr(char),
    Htb(HashMap<Object, Object>),
    Int(i32),
    Lon(i64),
    Ptr(Option<String>),
    Str(Option<String>),
    Tim(i32),
}

// I need to implement this to get nested hash tables to work. Derive hash
// on Object isn't working for it, so I'm manually doing it here
impl Hash for Object {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match *self {
            Object::Arr(ref x) => x.hash(state),
            Object::Buf(ref x) => x.hash(state),
            Object::Chr(ref x) => x.hash(state),
            Object::Htb(ref x) => format!("{:?}", x).hash(state), // Not ideal
            Object::Int(ref x) => x.hash(state),
            Object::Lon(ref x) => x.hash(state),
            Object::Ptr(ref x) => x.hash(state),
            Object::Str(ref x) => x.hash(state),
            Object::Tim(ref x) => x.hash(state),
        };
    }
}

impl Object {
    pub fn as_array(&self) -> Result<&[Object], WeechatError> {
        match *self {
            Object::Arr(ref arr) => Ok(arr.as_slice()),
            _                    => Err(ParseError("Item is not an array".to_string())),
        }
    }

    /// Returns this data as a buffer if it is a buffer.
    pub fn as_buffer(&self) -> Result<Option<&[u8]>, WeechatError> {
        match *self {
            Object::Buf(Some(ref vec)) => Ok(Some(vec.as_slice())),
            Object::Buf(None)          => Ok(None),
            _                          => Err(ParseError("Item is not a buffer".to_string()))
        }
    }

    /// Returns this data as a buffer if it is a non-null buffer. Note: null != empty
    pub fn as_not_null_buffer(&self) -> Result<&[u8], WeechatError> {
        try!(self.as_buffer().map(|b| b.ok_or(ParseError("Buffer is null".to_string()))))
    }

    /// Returns this data as a character if it is a character.
    pub fn as_character(&self) -> Result<char, WeechatError> {
        match *self {
            Object::Chr(c) => Ok(c),
            _              => Err(ParseError("Item is not a character".to_string()))
        }
    }

    /// Returns this data as a integer if it is a integer.
    pub fn as_integer(&self) -> Result<i32, WeechatError> {
        match *self {
            Object::Int(i) => Ok(i),
            _              => Err(ParseError("Item is not a integer".to_string()))
        }
    }

    /// Returns this data as a long if it is a long.
    pub fn as_long(&self) -> Result<i64, WeechatError> {
        match *self {
            Object::Lon(l) => Ok(l),
            _              => Err(ParseError("Item is not a long".to_string()))
        }
    }

    /// Returns this data as a pointer if it is a pointer (pointer is encoded as a str).
    pub fn as_pointer(&self) -> Result<Option<&str>, WeechatError> {
        match *self {
            Object::Ptr(Some(ref p)) => Ok(Some(p)),
            Object::Ptr(None)        => Ok(None),
            _                        => Err(ParseError("Item is not a buffer".to_string()))
        }
    }

    /// Returns this data as a pointer if it is a non-null pointer (pointer is
    /// encoded as a str). Note: null != empty
    pub fn as_not_null_pointer(&self) -> Result<&str, WeechatError> {
        try!(self.as_pointer().map(|p| p.ok_or(ParseError("pointer is null".to_string()))))
    }

    /// Returns this data as a string if it is a string.
    pub fn as_str(&self) -> Result<Option<&str>, WeechatError> {
        match *self {
            Object::Str(Some(ref s)) => Ok(Some(s)),
            Object::Str(None)        => Ok(None),
            _                        => Err(ParseError("Item is not a buffer".to_string()))
        }
    }

    /// Returns this data as a string if it is a non-null string. Note: null != empty
    pub fn as_not_null_str(&self) -> Result<&str, WeechatError> {
        try!(self.as_str().map(|s| s.ok_or(ParseError("String is null".to_string()))))
    }

    /// Returns this data as an epoch time if it is a time (encdoed as an i32)
    pub fn as_time(&self) -> Result<i32, WeechatError> {
        match *self {
            Object::Tim(t) => Ok(t),
            _              => Err(ParseError("Item is not a time".to_string()))
        }
    }
}

/// A simple display for Objects (all of the data types that can be returned
/// as object in an HDAta). This is primarily used for debugging
impl fmt::Display for Object {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Object::Str(Some(ref s)) => write!(f, "\"{}\"", s),
            Object::Ptr(Some(ref p)) => write!(f, "0x{}", p),
            Object::Buf(Some(ref b)) => {
                try!(write!(f, "[ "));
                for byte in b {
                    try!(write!(f, "{}, ", byte));
                }
                write!(f, "]")
            }
            Object::Buf(None)  => write!(f, "null"),
            Object::Str(None)  => write!(f, "null"),
            Object::Ptr(None)  => write!(f, "0x0"),
            Object::Chr(ref c) => write!(f, "{} ('{}')", *c as u8, c),
            Object::Int(ref i) => write!(f, "{}", i),
            Object::Lon(ref l) => write!(f, "{}", l),
            Object::Tim(ref t) => write!(f, "{}", t),
            Object::Htb(ref h) =>  {
                try!(write!(f, "{{ "));
                for (key, value) in h {
                    try!(key.fmt(f));
                    try!(write!(f, ": "));
                    try!(value.fmt(f));
                    try!(write!(f, ", "));
                }
                write!(f, "}}")
            }
            Object::Arr(ref d) => {
                try!(write!(f, "[ "));
                for i in d {
                    try!(i.fmt(f));
                    try!(write!(f, ", "));
                }
                write!(f, "]")
            },
        }
    }
}

