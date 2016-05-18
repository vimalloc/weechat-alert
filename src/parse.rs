use std::str::from_utf8;
use std::mem::transmute;

use message::Object;
use errors::WeechatError;
use errors::WeechatError::ParseError;

/// Extract binary data into weechat message objects.
pub struct Parse {
    pub object: Object,
    pub bytes_read: usize,
}

impl Parse {
    /// Given a byte array which contains an encoded array (of some Object
    /// type), pull out everything from the array and return it as a vector of
    /// Objects. The protocl for this is:
    ///
    /// bytes 0 - 3: String (Datatype). Ex: 'str', 'int', 'tim', etc,
    /// bytes 3 - 7: Integer (Number of elements in array)
    /// bytes 7 - ?: Elements of the array
    ///
    /// Note: A NULL array is valid. It is simply an array with the number of
    ///       elements being zero. Because anyone using this will likely be
    ///       iterating over the array, in this case we are encoding a NULL
    ///       array as an empty array, instead of having an Array be of type
    ///       Option.
    pub fn array(data: &[u8]) -> Result<Parse, WeechatError> {
        if data.len() < 7 {
            return Err(ParseError("Not enough bytes to have an array".to_string()));
        }
        let arr_type = try!(from_utf8(&data[0..3]));
        let num_elements = try!(bytes_to_i32(&data[3..7]));
        let mut array: Vec<Object> = Vec::new();

        let mut cur_pos = 7;  // Start position for data array elements
        for _ in 0..num_elements {
            let extracted = match arr_type {
                "chr" => try!(Parse::character(&data[cur_pos..])),
                "int" => try!(Parse::integer(&data[cur_pos..])),
                "lon" => try!(Parse::long(&data[cur_pos..])),
                "str" => try!(Parse::string(&data[cur_pos..])),
                "buf" => try!(Parse::buffer(&data[cur_pos..])),
                "ptr" => try!(Parse::pointer(&data[cur_pos..])),
                "tim" => try!(Parse::time(&data[cur_pos..])),
                "arr" => try!(Parse::array(&data[cur_pos..])),
                _     => return Err(ParseError("Bad type for array".to_string())),
            };
            cur_pos += extracted.bytes_read;
            array.push(extracted.object);
        }

        Ok(Parse {
            object: Object::Arr(array),
            bytes_read: cur_pos
        })
    }

    /// Given a byte array which contains an encoded buffer, pull the buffer out
    /// and return it. The protocol for buffers are:
    ///
    /// bytes 0 - 4: signed integer, size of buffer
    /// bytes 4 - ?: The actual buffer
    ///
    /// Note: An empty buffer is valid, in this cass length will be 0. A NULL
    ///       buffer is also valid, it has length of -1.
    pub fn buffer(data: &[u8]) -> Result<Parse, WeechatError> {
        // Sanity checks
        if data.len() < 4 {
            return Err(ParseError("Not enough bytes to parse buffer".to_string()));
        }

        // Get the start and end limits for this string
        let mut start = 0;
        let mut end = 4;
        let buf_size = try!(bytes_to_i32(&data[start..end]));
        start = end;
        end += buf_size as usize;
        if data.len() >= end {
            return Err(ParseError("Buffer larger then availiable bytes".to_string()));
        }

        // Pull out and return the string
        let buf_object = match buf_size {
            -1 => None,              // Null buffer
            0  => Some(Vec::new()),  // Empty buffer
            _  => {
                let mut buf = Vec::new();
                buf.clone_from_slice(&data[start..end]);
                Some(buf)
            }
        };
        Ok(Parse{
            object: Object::Buf(buf_object),
            bytes_read: end
        })
    }

    /// Given a byte array which contains an encoded char, pull the char out.
    pub fn character(data: &[u8]) -> Result<Parse, WeechatError> {
        if data.len() < 1 {
            return Err(ParseError("Not enough bytes to parse character".to_string()));
        }
        Ok(Parse {
            object: Object::Chr(data[0] as char),
            bytes_read: 1,
        })
    }

    /// Given a byte array which contains an encoded integer, pull the int out.
    pub fn integer(data: &[u8]) -> Result<Parse, WeechatError> {
        if data.len() < 4 {
            return Err(ParseError("Not enough bytes to parse int".to_string()));
        }
        Ok(Parse {
            object: Object::Int(try!(bytes_to_i32(&data[0..4]))),
            bytes_read: 4,
        })
    }

    /// Given a byte array which contains an encoded long integer, pull it out.
    ///
    /// The long integer is encoded as a string, instead of bytes (like the
    /// integer encoding). The protocl for this is:
    ///
    /// bytes 0: The length of the encoded long integer (number of chars)
    /// bytes 1 - ?: A string representing the long (ex "1234567890")
    pub fn long(data: &[u8]) -> Result<Parse, WeechatError> {
        if data.len() < 2 {
            return Err(ParseError("Not enough bytes to parse long".to_string()));
        }
        let long_size = data[0] as i8;
        let start = 1;
        let end = start + long_size as usize;
        if data.len() < end {
            return Err(ParseError("Long larger then available bytes".to_string()));
        }

        let long_str = try!(from_utf8(&data[start..end]));
        let long: i64 = match long_str.parse() {
            Err(_) => return Err(ParseError("String to long conversion failed".to_string())),
            Ok(l)  => l,
        };
        Ok(Parse {
            object: Object::Lon(long),
            bytes_read: end,
        })
    }

    /// Given a byte array which contains an ecnoded pointer, pull the pointer
    /// out and return it. The protocol for pointers are:
    ///
    /// byte 0: i8, size of pointer
    /// bytes 1 - ?: pointer
    ///
    /// Note: A null poniter is valid. It will have size 1, and the pointer
    ///       object of 0
    pub fn pointer(data: &[u8]) -> Result<Parse, WeechatError> {
        if data.len() < 2 {
            return Err(ParseError("Not enough bytes to parse pointer".to_string()));
        }

        let ptr_size = data[0] as i8;
        let start = 1;
        let end = start + ptr_size as usize;
        if data.len() < end {
            return Err(ParseError("Pointer larger then availiable bytes".to_string()));
        }

        // Pull out pointer, check if it's null
        let ptr = try!(from_utf8(&data[start..end])).to_string();
        let object = if ptr.len() == 1 && ptr == "0" { None } else { Some(ptr) };
        Ok(Parse {
            object: Object::Ptr(object),
            bytes_read: end,
        })
    }

    /// Given a byte array which contains an encoded str, pull the string out
    /// and return it. The protocol from strings are:
    ///
    /// bytes 0 - 4: signed integer, size of string
    /// bytes 4 - ?: The actual string message
    ///
    /// Note: An empty string is valid, in this cass length will be 0. A NULL
    ///       string is also valid, it has length of -1.
    pub fn string(data: &[u8]) -> Result<Parse, WeechatError> {
        // Sanity checks
        if data.len() < 4 {
            return Err(ParseError("Not enough bytes to parse string".to_string()));
        }

        // Get the start and end limits for this string
        let mut start = 0;
        let mut end = 4;
        let str_size = try!(bytes_to_i32(&data[start..end]));
        start = end;
        end += str_size as usize;
        if data.len() < end {
            return Err(ParseError("String larger then availiable bytes".to_string()));
        }

        // Pull out and return the string
        let string_object = match str_size as i32 {
            -1 => None,                  // Null string
            0  => Some("".to_string()),  // Empty string
            _  => Some(try!(from_utf8(&data[start..end])).to_string()),
        };
        Ok(Parse{
            object: Object::Str(string_object),
            bytes_read: end
        })
    }

    /// Given a byte array which contains an encoded time, pull it out.
    ///
    /// The time is encoded as a string, and represents a unix (epoch) timestamp.
    /// The protocl for this is:
    ///
    /// bytes 0: The length of the encoded time string (number of chars)
    /// bytes 1 - ?: A string representing the timestamp (ex "1321993456")
    pub fn time(data: &[u8]) -> Result<Parse, WeechatError> {
        if data.len() < 2 {
            return Err(ParseError("Not enough bytes parse time".to_string()));
        }
        let time_size = data[0] as i8;
        let start = 1;
        let end = start + time_size as usize;
        if data.len() < end {
            return Err(ParseError("Not enough bytes to extract time".to_string()));
        }

        let time_str = try!(from_utf8(&data[start..end]));
        let timestamp: i32 = match time_str.parse() {
            Err(_) => return Err(ParseError("String to i32 conversion failed".to_string())),
            Ok(ts) => ts,
        };
        Ok(Parse {
            object: Object::Tim(timestamp),
            bytes_read: end,
        })
    }
}

/// Converts a 4 byte array slice into a 32 bit signed integer. The bytes
/// are assumed to be encoded in a big-endian format
fn bytes_to_i32(byte_array: &[u8]) -> Result<i32, WeechatError> {
    if byte_array.len() != 4 {
        return Err(WeechatError::ParseError("Cannot cast bytes to i32".to_string()));
    }

    // Re-arrange bytes from big to little-endian (so we can transmute them)
    let mut bytes: [u8; 4] = [0, 0, 0, 0];
    bytes[0] = byte_array[3];
    bytes[1] = byte_array[2];
    bytes[2] = byte_array[1];
    bytes[3] = byte_array[0];

    // Do the casting
    unsafe {
        Ok(transmute::<[u8; 4], i32>(bytes))
    }
}
