use std::collections::HashMap;

use conversions::bytes_to_i32;
use message_data::{DataType, extract_char, extract_time, extract_int, extract_string,
                   extract_pointer, extract_long, extract_buffer, extract_array};
use errors::WeechatError;


/// A list of key/value mappings of data received from relay. This data conststs
/// of the paths and the keys declared in the weechat relay messages protocol:
/// https://weechat.org/files/doc/devel/weechat_relay_protocol.en.html
pub struct HData {
    pub data: Vec<HashMap<String, DataType>>
}


impl HData {
    /// Takes an array of bytes where the HData starts and returns an HData
    /// object. This should not include the leading "hda" string that identifies
    /// the object as an HData, ie the bytes should start right after the
    /// identifying "hda" string.
    ///
    /// You can see the protocol for encoding an hdata object here:
    /// https://weechat.org/files/doc/devel/weechat_relay_protocol.en.html#object_hdata
    pub fn new(data: &[u8]) -> Result<HData, WeechatError> {
        let mut cur_pos = 0; // Rolling counter of where we are in the byte array
        let mut data_list: Vec<HashMap<String, DataType>> = Vec::new();  // resulting hdata

        // Parse out paths
        let extracted = try!(extract_string(&data[cur_pos..]));
        cur_pos += extracted.bytes_read;
        let paths: Vec<String> = match extracted.value {
            DataType::Str(Some(ref s)) => s.split(',').map(|s| String::from(s)).collect(),
            _ => return Err(WeechatError::ParseError("Invalid Path type".to_string())),
        };

        // Parse out key names and types
        let extracted = try!(extract_string(&data[cur_pos..]));
        cur_pos += extracted.bytes_read;
        let keys: Vec<String> = match extracted.value {
            DataType::Str(Some(ref s)) => s.split(',').map(|s| String::from(s)).collect(),
            _ => return Err(WeechatError::ParseError("Invalid key type".to_string())),
        };

        // Number of items in this hdata
        let num_hdata_items = try!(bytes_to_i32(&data[cur_pos..cur_pos+4]));
        cur_pos += 4;

        // Get the data for each item
        for _ in 0..num_hdata_items {
            // Store for this item
            let mut key_value_map: HashMap<String, DataType> = HashMap::new();

            // Pull out path pointers
            for path_name in &paths {
                let extracted = try!(extract_pointer(&data[cur_pos..]));
                cur_pos += extracted.bytes_read;
                key_value_map.insert(path_name.clone(), extracted.value);
            }

            // Pull out the data for all of the keys
            for key in &keys {
                let key_parse: Vec<&str> = key.split(':').collect();
                let key_name = key_parse[0];
                let key_type = key_parse[1];
                let extracted = match key_type {
                    "chr" => try!(extract_char(&data[cur_pos..])),
                    "int" => try!(extract_int(&data[cur_pos..])),
                    "lon" => try!(extract_long(&data[cur_pos..])),
                    "str" => try!(extract_string(&data[cur_pos..])),
                    "buf" => try!(extract_buffer(&data[cur_pos..])),
                    "ptr" => try!(extract_pointer(&data[cur_pos..])),
                    "tim" => try!(extract_time(&data[cur_pos..])),
                    "arr" => try!(extract_array(&data[cur_pos..])),
                    _     => return Err(WeechatError::ParseError("Bad type for key".to_string())),
                };
                cur_pos += extracted.bytes_read;
                key_value_map.insert(String::from(key_name), extracted.value);
            }

            // And finally, add this item to the return data
            data_list.push(key_value_map);
        }

        Ok(HData {
            data: data_list,
        })
    }
}
