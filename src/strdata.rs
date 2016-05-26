use errors::WeechatError;
use parse::Parse;

/// String data received from a weechat message
#[derive(Debug)]
pub struct StrData {
    data: Option<String>
}

impl StrData {
    /// Takes an array of bytes that encode a StrData, and returns a parse StrData object
    ///
    /// This should not include the leading "str" string that identifies
    /// the object as a StrData, ie the bytes should start right after the
    /// identifying "str" string.
    pub fn new(bytes: &[u8]) -> Result<StrData, WeechatError> {
        let parsed = try!(Parse::string(bytes));
        let s = try!(parsed.object.as_str()).map(|s| s.to_string());
        if bytes.len() != parsed.bytes_read {
            Err(WeechatError::ParseError("Not all bytes in message consumed".to_string()))
        } else {
            Ok(StrData{ data: s })
        }
    }
}
