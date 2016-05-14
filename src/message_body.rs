use std::str::from_utf8;

use errors::WeechatError;
use hdata::HData;
use message_data::{DataType, extract_string};

/// Holds body information for data received from relay
pub struct MessageData {
    pub identifier: String,
    pub data: MessageType,
}

/// Possible types of messages received from relay (everything besides ping
/// will use HData at present)
pub enum MessageType {
    StrData(Option<String>),
    HData(HData),
}


impl MessageData {
    pub fn new(data: &[u8]) -> Result<MessageData, WeechatError> {
        // First thing encoded in the binary data is the identifier for
        // what this command is
        let extracted = try!(extract_string(data));
        let identifier = match extracted.value {
            DataType::Str(Some(s)) => s,
            _ => return Err(WeechatError::ParseError("Invalid identifier".to_string())),
        };

        // Next 3 bytes determin type of data in this command (hdata or str).
        // Parse the data type depending
        let start = extracted.bytes_read;
        let end = start + 3;
        let msg_type = match try!(from_utf8(&data[start..end])) {
            "str" => try!(MessageData::binary_to_strdata(&data[end..])),
            "hda" => MessageType::HData(try!(HData::new(&data[end..]))),
            _ => return Err(WeechatError::ParseError("Unknown message type".to_string())),
        };

        // Return our struct
        Ok(MessageData {
            identifier: String::from(identifier),
            data: msg_type,
        })
    }

    // TODO move you into new class as well, string_data
    fn binary_to_strdata(data: &[u8]) -> Result<MessageType, WeechatError> {
        let extracted = try!(extract_string(data));
        match extracted.value {
            DataType::Str(s) => Ok(MessageType::StrData(s)),
            _ => return Err(WeechatError::ParseError("Invalid datatype".to_string())),
        }
    }
}
