use message_data::{extract_int, extract_char};
use errors::WeechatError;

/// Holds header information for data received from relay
pub struct MessageHeader {
    pub length: usize,
    pub compression: bool,
}


impl MessageHeader {
    /// Takes a new message received by the relay, and parses out the header for it
    ///
    /// The header protocol has the first 4 bytes make an integer which is the,
    /// total size of the message, and a single byte which represents if zlib
    /// compression is enabled for the rest of the message
    pub fn new(data: &[u8]) -> Result<MessageHeader, WeechatError> {
        let mut cur_pos = 0; // Rolling counter of where we are in the byte array

        // Grab the message length
        let extracted = try!(extract_int(data));
        let total_msg_length = try!(extracted.value.as_integer());
        cur_pos += extracted.bytes_read;

        // Grab the compression character
        let extracted = try!(extract_char(&data[cur_pos..]));
        let compression = try!(extracted.value.as_character());
        let compression = match compression as u8 {
            0 => false,
            1 => true,
            _ => return Err(WeechatError::ParseError("Bad compression data".to_string())),
        };
        cur_pos += extracted.bytes_read;

        // Headers has length of full message, we need to chop off the
        // legth of the header as we have already read that from the socket
        let length = total_msg_length as usize - cur_pos;

        // Create the struct
        Ok(MessageHeader {
            length: length,
            compression: compression,
        })
    }
}
