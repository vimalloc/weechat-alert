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
        let extracted = extract_string(data);
        let identifier = match extracted.value {
            DataType::Str(Some(s)) => s,
            _                      => panic!("identifier should be non-null DataType::Str"),
        };

        // Next 3 bytes determin type of data in this command (hdata or str).
        // Parse the data type depending
        let start = extracted.bytes_read;
        let end = start + 3;
        let msg_type = match from_utf8(&data[start..end]).unwrap() {
            "str" => MessageData::binary_to_strdata(&data[end..]),
            "hda" => MessageType::HData(HData::new(&data[end..])),
            _     => panic!("Received unknown message type"),
        };

        // Return our struct
        Ok(MessageData {
            identifier: String::from(identifier),
            data: msg_type,
        })
    }

    // TODO move you into new class as well, string_data
    fn binary_to_strdata(data: &[u8]) -> MessageType {
        let extracted = extract_string(data);
        match extracted.value {
            DataType::Str(s) => MessageType::StrData(s),
            _                => panic!("Extracted is not DataType::Str"),
        }
    }
}
