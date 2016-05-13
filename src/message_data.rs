use std::str::from_utf8;

use conversions::bytes_to_i32;

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
pub fn extract_string(data: &[u8]) -> ExtractedData {
    // Sanity checks
    assert!(data.len() >= 4, "Not enough bytes in array to extract string");

    // Get the start and end limits for this string
    let mut start = 0;
    let mut end = 4;
    let str_size = bytes_to_i32(&data[start..end]);
    start = end;
    end += str_size as usize;
    assert!(data.len() >= end, "Not enough bytes in array to extract string");

    // Pull out and return the string
    match str_size {
        0  => ExtractedData {  // Empty string
                  value: DataType::Str(Some(String::from(""))),
                  bytes_read: end
              },
        -1 => ExtractedData {  // Null string
                  value: DataType::Str(None),
                  bytes_read: end
              },
        _  => ExtractedData {  // Normal string
                 value: DataType::Str(Some(String::from(from_utf8(&data[start..end]).unwrap()))),
                 bytes_read: end
              },
    }
}

/// Given a byte array which contains an ecnoded pointer, pull the pointer
/// out and return it. The protocol for pointers are:
///
/// byte 0: i8, size of pointer
/// bytes 1 - ?: pointer
///
/// Note: A null poniter is valid. It will have size 1, and the pointer
///       value of 0
pub fn extract_pointer(data: &[u8]) -> ExtractedData {
    assert!(data.len() >= 2, "Not enough bytes in array to extract pointer");
    let ptr_size = data[0] as i8;
    let start = 1;
    let end = start + ptr_size as usize;
    assert!(data.len() >= end, "Not enough bytes in array to extract pointer");

    // Pull out pointer, check if it's null
    let ptr = String::from(from_utf8(&data[start..end]).unwrap());
    let value;
    if ptr.len() == 1 && ptr == "0" {
        value = DataType::Ptr(None);
    } else {
        value = DataType::Ptr(Some(ptr));
    }

    ExtractedData {
        value: value,
        bytes_read: end,
    }
}

/// Given a byte array which contains an encoded char, pull the char out.
pub fn extract_char(data: &[u8]) -> ExtractedData {
    assert!(data.len() >= 1, "Not enough bytes in array to extract char");
    ExtractedData {
        value: DataType::Chr(data[0] as char),
        bytes_read: 1,

    }
}

/// Given a byte array which contains an encoded integer, pull the int out.
pub fn extract_int(data: &[u8]) -> ExtractedData {
    assert!(data.len() >= 4, "Not enough bytes in array to extract int");
    ExtractedData {
        value: DataType::Int(bytes_to_i32(&data[0..4])),
        bytes_read: 4,
    }
}

/// Given a byte array which contains an encoded long integer, pull it out.
///
/// The long integer is encoded as a string, instead of bytes (like the
/// integer encoding). The protocl for this is:
///
/// bytes 0: The length of the encoded long integer (number of chars)
/// bytes 1 - ?: A string representing the long (ex "1234567890")
pub fn extract_long(data: &[u8]) -> ExtractedData {
    assert!(data.len() >= 2, "Not enough bytes in array to extract long");
    let long_size = data[0] as i8;
    let start = 1;
    let end = start + long_size as usize;
    assert!(data.len() >= end, "Not enough bytes in array to extract long");

    let long_str = from_utf8(&data[start..end]).unwrap();
    let long: i64 = long_str.parse().unwrap();
    ExtractedData {
        value: DataType::Lon(long),
        bytes_read: end,
    }
}

/// Given a byte array which contains an encoded buffer, pull the buffer out
/// and return it. The protocol for buffers are:
///
/// bytes 0 - 4: signed integer, size of buffer
/// bytes 4 - ?: The actual buffer
///
/// Note: An empty buffer is valid, in this cass length will be 0. A NULL
///       buffer is also valid, it has length of -1.
pub fn extract_buffer(data: &[u8]) -> ExtractedData {
    // Sanity checks
    assert!(data.len() >= 4, "Not enough bytes in array to extract buffer");

    // Get the start and end limits for this string
    let mut start = 0;
    let mut end = 4;
    let buf_size = bytes_to_i32(&data[start..end]);
    start = end;
    end += buf_size as usize;
    assert!(data.len() >= end, "Not enough bytes in array to extract buffer");

    // Pull out and return the string
    match buf_size {
        0  => ExtractedData {  // Empty string
                  value: DataType::Buf(Some(Vec::new())),
                  bytes_read: end
              },
        -1 => ExtractedData {  // Null string
                  value: DataType::Buf(None),
                  bytes_read: end
              },
        _  => {
                  let mut buf: Vec<u8> = Vec::new();
                  buf.clone_from_slice(&data[start..end]);
                  ExtractedData {  // Normal string
                      value: DataType::Buf(Some(buf)),
                      bytes_read: end
                  }
              }
    }
}

/// Given a byte array which contains an encoded time, pull it out.
///
/// The time is encoded as a string, and represents a unix (epoch) timestamp.
/// The protocl for this is:
///
/// bytes 0: The length of the encoded time string (number of chars)
/// bytes 1 - ?: A string representing the timestamp (ex "1321993456")
pub fn extract_time(data: &[u8]) -> ExtractedData {
    assert!(data.len() >= 2, "Not enough bytes in array to extract time");
    let time_size = data[0] as i8;
    let start = 1;
    let end = start + time_size as usize;
    assert!(data.len() >= end, "Not enough bytes in array to extract time");

    let time_str = from_utf8(&data[start..end]).unwrap();
    let timestamp: i32 = time_str.parse().unwrap();
    ExtractedData {
        value: DataType::Tim(timestamp),
        bytes_read: end,
    }
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
pub fn extract_array(data: &[u8]) -> ExtractedData {
    assert!(data.len() >= 7, "Not enough bytes to have an array");
    let arr_type = from_utf8(&data[0..3]).unwrap();
    let num_elements = bytes_to_i32(&data[3..7]);
    let mut array: Vec<DataType> = Vec::new();

    let mut cur_pos = 7;  // Start position for data array elements
    for _ in 0..num_elements {
        let extracted = match arr_type {
            "chr" => extract_char(&data[cur_pos..]),
            "int" => extract_int(&data[cur_pos..]),
            "lon" => extract_long(&data[cur_pos..]),
            "str" => extract_string(&data[cur_pos..]),
            "buf" => extract_buffer(&data[cur_pos..]),
            "ptr" => extract_pointer(&data[cur_pos..]),
            "tim" => extract_time(&data[cur_pos..]),
            "arr" => extract_array(&data[cur_pos..]),
            _     => panic!("Received invalid arr type"),
        };
        cur_pos += extracted.bytes_read;
        array.push(extracted.value);
    }

    ExtractedData {
        value: DataType::Arr(array),
        bytes_read: cur_pos
    }
}
