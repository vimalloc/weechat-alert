use conversions::bytes_to_i32;
use errors::WeechatError;

// How many bytes make up the message header
// TODO put this in just one place, or hand off actually reading data from the
//      socket to message header
const HEADER_LENGTH: usize = 5;

/// Holds header information for data received from relay
pub struct MessageHeader {
    pub length: usize,
    pub compression: bool,
}


impl MessageHeader {
    /// Takes a new message received by the relay, and parses out the header for it
    pub fn new(data: &[u8]) -> Result<MessageHeader, WeechatError> {
        // Headers has length of full message, we need to chop off the
        // legth of the header as we have already read that from the socket
        let total_msg_length = try!(bytes_to_i32(&data[0..4]));
        let length = total_msg_length as usize - HEADER_LENGTH;

        // Pull compression out of bytes, and verify it's 1 or 0
        let compression = match data[4] {
            0 => false,
            1 => true,
            _ => return Err(WeechatError::ParseError("Bad compression byte".to_string())),
        };

        // Create the struct
        Ok(MessageHeader {
            length: length,
            compression: compression,
        })
    }
}
