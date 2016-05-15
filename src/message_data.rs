use std::str::from_utf8;
use std::fmt;

use conversions::bytes_to_i32;
use errors::WeechatError;
use errors::WeechatError::ParseError;

/// All possible types of data that can be returned from a weechat message
pub enum DataType {
    Buf(Option<Vec<u8>>),
    Chr(char),
    Int(i32),
    Lon(i64),
    Ptr(Option<String>),
    Str(Option<String>),
    Tim(i32),
    Arr(Vec<DataType>),
}

/// A simple display for DataTypes (all of the data types that can be returned
/// as a value in an HDAta object). This is primarily used for debugging
impl fmt::Display for DataType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DataType::Str(Some(ref s)) => write!(f, "\"{}\"", s),
            DataType::Ptr(Some(ref p)) => write!(f, "0x{}", p),
            DataType::Buf(Some(ref b)) => {
                                              try!(write!(f, "[ "));
                                              for byte in b {
                                                  try!(write!(f, "{}, ", byte));
                                              }
                                              write!(f, "]")
                                          }
            DataType::Buf(None)  => write!(f, "null"),
            DataType::Str(None)  => write!(f, "null"),
            DataType::Ptr(None)  => write!(f, "0x0"),
            DataType::Chr(ref c) => write!(f, "{} ('{}')", *c as u8, c),
            DataType::Int(ref i) => write!(f, "{}", i),
            DataType::Lon(ref l) => write!(f, "{}", l),
            DataType::Tim(ref t) => write!(f, "{}", t),
            DataType::Arr(ref d) => {
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

/// Struct used to help extract usable data from the binary data returned via
/// a weechat message
pub struct ExtractedData {
    pub value: DataType,
    pub bytes_read: usize,
}

/// Given a byte array which contains an encoded str, pull the string out
/// and return it. The protocol from strings are:
///
/// bytes 0 - 4: signed integer, size of string
/// bytes 4 - ?: The actual string message
///
/// Note: An empty string is valid, in this cass length will be 0. A NULL
///       string is also valid, it has length of -1.
pub fn extract_string(data: &[u8]) -> Result<ExtractedData, WeechatError> {
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
    let string_value = match str_size as i32 {
        -1 => None,                  // Null string
        0  => Some("".to_string()),  // Empty string
        _  => Some(try!(from_utf8(&data[start..end])).to_string()),
    };
    Ok(ExtractedData{
        value: DataType::Str(string_value),
        bytes_read: end
    })
}

/// Given a byte array which contains an ecnoded pointer, pull the pointer
/// out and return it. The protocol for pointers are:
///
/// byte 0: i8, size of pointer
/// bytes 1 - ?: pointer
///
/// Note: A null poniter is valid. It will have size 1, and the pointer
///       value of 0
pub fn extract_pointer(data: &[u8]) -> Result<ExtractedData, WeechatError> {
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
    let value = if ptr.len() == 1 && ptr == "0" { None } else { Some(ptr) };
    Ok(ExtractedData {
        value: DataType::Ptr(value),
        bytes_read: end,
    })
}

/// Given a byte array which contains an encoded char, pull the char out.
pub fn extract_char(data: &[u8]) -> Result<ExtractedData, WeechatError> {
    if data.len() < 1 {
        return Err(ParseError("Not enough bytes to parse char".to_string()));
    }
    Ok(ExtractedData {
        value: DataType::Chr(data[0] as char),
        bytes_read: 1,
    })
}

/// Given a byte array which contains an encoded integer, pull the int out.
pub fn extract_int(data: &[u8]) -> Result<ExtractedData, WeechatError> {
    if data.len() < 4 {
        return Err(ParseError("Not enough bytes to parse int".to_string()));
    }
    Ok(ExtractedData {
        value: DataType::Int(try!(bytes_to_i32(&data[0..4]))),
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
pub fn extract_long(data: &[u8]) -> Result<ExtractedData, WeechatError> {
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
    Ok(ExtractedData {
        value: DataType::Lon(long),
        bytes_read: end,
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
pub fn extract_buffer(data: &[u8]) -> Result<ExtractedData, WeechatError> {
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
    let buf_value = match buf_size {
        -1 => None,              // Null buffer
        0  => Some(Vec::new()),  // Empty buffer
        _  => {
                let mut buf = Vec::new();
                buf.clone_from_slice(&data[start..end]);
                Some(buf)
              }
    };
    Ok(ExtractedData{
        value: DataType::Buf(buf_value),
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
pub fn extract_time(data: &[u8]) -> Result<ExtractedData, WeechatError> {
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
    Ok(ExtractedData {
        value: DataType::Tim(timestamp),
        bytes_read: end,
    })
}

/// Given a byte array which contains an encoded array (of some DataType
/// type), pull out everything from the array and return it as a vector of
/// DataTypes. The protocl for this is:
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
pub fn extract_array(data: &[u8]) -> Result<ExtractedData, WeechatError> {
    if data.len() < 7 {
        return Err(ParseError("Not enough bytes to have an array".to_string()));
    }
    let arr_type = try!(from_utf8(&data[0..3]));
    let num_elements = try!(bytes_to_i32(&data[3..7]));
    let mut array: Vec<DataType> = Vec::new();

    let mut cur_pos = 7;  // Start position for data array elements
    for _ in 0..num_elements {
        let extracted = match arr_type {
            "chr" => try!(extract_char(&data[cur_pos..])),
            "int" => try!(extract_int(&data[cur_pos..])),
            "lon" => try!(extract_long(&data[cur_pos..])),
            "str" => try!(extract_string(&data[cur_pos..])),
            "buf" => try!(extract_buffer(&data[cur_pos..])),
            "ptr" => try!(extract_pointer(&data[cur_pos..])),
            "tim" => try!(extract_time(&data[cur_pos..])),
            "arr" => try!(extract_array(&data[cur_pos..])),
            _     => return Err(ParseError("Bad type for array".to_string())),
        };
        cur_pos += extracted.bytes_read;
        array.push(extracted.value);
    }

    Ok(ExtractedData {
        value: DataType::Arr(array),
        bytes_read: cur_pos
    })
}
