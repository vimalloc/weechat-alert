use std::str::from_utf8;
use std::mem::transmute;
use std::collections::HashMap;

use message::Object;
use errors::WeechatError;
use errors::WeechatError::ParseError;

/// Parses binary data into weechat message objects.
pub struct Parse {
    /// Object type of this data
    pub object: Object,
    /// Number of bytes read from the byte array to parse this data
    pub bytes_read: usize,
}

impl Parse {
    fn parse_type(data_type: &str, bytes: &[u8]) -> Result<Parse, WeechatError> {
         Ok(match data_type {
            "chr" => try!(Parse::character(bytes)),
            "int" => try!(Parse::integer(bytes)),
            "lon" => try!(Parse::long(bytes)),
            "str" => try!(Parse::string(bytes)),
            "buf" => try!(Parse::buffer(bytes)),
            "ptr" => try!(Parse::pointer(bytes)),
            "tim" => try!(Parse::time(bytes)),
            "arr" => try!(Parse::array(bytes)),
            "htb" => try!(Parse::hashtable(bytes)),
            _     => return Err(ParseError("Unknown data type".to_string())),
        })
    }

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
    pub fn array(bytes: &[u8]) -> Result<Parse, WeechatError> {
        if bytes.len() < 7 {
            return Err(ParseError("Not enough bytes to have an array".to_string()));
        }
        let arr_type = try!(from_utf8(&bytes[0..3]));
        let num_elements = try!(bytes_to_i32(&bytes[3..7]));
        let mut array: Vec<Object> = Vec::new();

        let mut cur_pos = 7;  // Start position for bytes array elements
        for _ in 0..num_elements {
            let parsed = try!(Parse::parse_type(arr_type, &bytes[cur_pos..]));
            cur_pos += parsed.bytes_read;
            array.push(parsed.object);
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
    pub fn buffer(bytes: &[u8]) -> Result<Parse, WeechatError> {
        // Sanity checks
        if bytes.len() < 4 {
            return Err(ParseError("Not enough bytes to parse buffer".to_string()));
        }

        // Get the start and end limits for this string
        let mut start = 0;
        let mut end = 4;
        let buf_size = try!(bytes_to_i32(&bytes[start..end]));
        start = end;
        end += buf_size as usize;
        if bytes.len() >= end {
            return Err(ParseError("Buffer larger then availiable bytes".to_string()));
        }

        // Pull out and return the string
        let buf_object = match buf_size {
            -1 => None,              // Null buffer
            0  => Some(Vec::new()),  // Empty buffer
            _  => {
                let mut buf = Vec::new();
                buf.clone_from_slice(&bytes[start..end]);
                Some(buf)
            }
        };
        Ok(Parse{
            object: Object::Buf(buf_object),
            bytes_read: end
        })
    }

    /// Given a byte array which contains an encoded char, pull the char out.
    pub fn character(bytes: &[u8]) -> Result<Parse, WeechatError> {
        if bytes.len() < 1 {
            return Err(ParseError("Not enough bytes to parse character".to_string()));
        }
        Ok(Parse {
            object: Object::Chr(bytes[0] as char),
            bytes_read: 1,
        })
    }

    /// Given a byte array which contains an encoded hashtable, pull it out.
    ///
    /// The protocol for hash tables are:
    /// Str: Type of the keys
    /// Str: Type of the values
    /// Int: Number of items
    /// Items
    pub fn hashtable(bytes: &[u8]) -> Result<Parse, WeechatError> {
        if bytes.len() < 10 {
            return Err(ParseError("Not enough bytes to have a hashtable".to_string()));
        }
        let key_type = try!(from_utf8(&bytes[0..3]));
        let value_type = try!(from_utf8(&bytes[3..6]));
        let num_entries = try!(bytes_to_i32(&bytes[6..10]));
        let mut map: HashMap<Object, Object> = HashMap::new();

        let mut cur_pos = 10;  // Start position for hashmap elements
        for _ in 0..num_entries {
            let parsed_key = try!(Parse::parse_type(key_type, &bytes[cur_pos..]));
            cur_pos += parsed_key.bytes_read;

            let parsed_value = try!(Parse::parse_type(value_type, &bytes[cur_pos..]));
            cur_pos += parsed_value.bytes_read;

            map.insert(parsed_key.object, parsed_value.object);
        }

        Ok(Parse {
            object: Object::Htb(map),
            bytes_read: cur_pos,
        })
    }

    /// Given a byte array which contains an encoded integer, pull the int out.
    pub fn integer(bytes: &[u8]) -> Result<Parse, WeechatError> {
        if bytes.len() < 4 {
            return Err(ParseError("Not enough bytes to parse int".to_string()));
        }
        Ok(Parse {
            object: Object::Int(try!(bytes_to_i32(&bytes[0..4]))),
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
    pub fn long(bytes: &[u8]) -> Result<Parse, WeechatError> {
        if bytes.len() < 2 {
            return Err(ParseError("Not enough bytes to parse long".to_string()));
        }
        let long_size = bytes[0] as i8;
        let start = 1;
        let end = start + long_size as usize;
        if bytes.len() < end {
            return Err(ParseError("Long larger then available bytes".to_string()));
        }

        let long_str = try!(from_utf8(&bytes[start..end]));
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
    pub fn pointer(bytes: &[u8]) -> Result<Parse, WeechatError> {
        if bytes.len() < 2 {
            return Err(ParseError("Not enough bytes to parse pointer".to_string()));
        }

        let ptr_size = bytes[0] as i8;
        let start = 1;
        let end = start + ptr_size as usize;
        if bytes.len() < end {
            return Err(ParseError("Pointer larger then availiable bytes".to_string()));
        }

        // Pull out pointer, check if it's null
        let ptr = try!(from_utf8(&bytes[start..end])).to_string();
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
    pub fn string(bytes: &[u8]) -> Result<Parse, WeechatError> {
        // Sanity checks
        if bytes.len() < 4 {
            return Err(ParseError("Not enough bytes to parse string".to_string()));
        }

        // Get the start and end limits for this string
        let mut start = 0;
        let mut end = 4;
        let str_size = try!(bytes_to_i32(&bytes[start..end]));
        start = end;
        end += str_size as usize;
        if bytes.len() < end {
            return Err(ParseError("String larger then availiable bytes".to_string()));
        }

        // Pull out and return the string
        let string_object = match str_size as i32 {
            -1 => None,                  // Null string
            0  => Some("".to_string()),  // Empty string
            _  => Some(try!(from_utf8(&bytes[start..end])).to_string()),
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
    pub fn time(bytes: &[u8]) -> Result<Parse, WeechatError> {
        if bytes.len() < 2 {
            return Err(ParseError("Not enough bytes parse time".to_string()));
        }
        let time_size = bytes[0] as i8;
        let start = 1;
        let end = start + time_size as usize;
        if bytes.len() < end {
            return Err(ParseError("Not enough bytes to parse time".to_string()));
        }

        let time_str = try!(from_utf8(&bytes[start..end]));
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
