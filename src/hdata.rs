use std::collections::HashMap;

use message_data::{DataType, extract_char, extract_time, extract_int, extract_string,
                   extract_pointer, extract_long, extract_buffer, extract_array};
use errors::WeechatError;
use errors::WeechatError::ParseError;


/// A list of key/value mappings of data received from relay. This data conststs
/// of the paths and the keys declared in the weechat relay messages protocol:
/// https://weechat.org/files/doc/devel/weechat_relay_protocol.en.html
#[derive(Debug)]
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

        // Parse out paths
        let extracted = try!(extract_string(&data[cur_pos..]));
        let paths = try!(extracted.value.as_not_null_str());
        let paths: Vec<String> = paths.split(',').map(|s| s.to_string()).collect();
        cur_pos += extracted.bytes_read;

        // Parse out key names and types
        let extracted = try!(extract_string(&data[cur_pos..]));
        let keys = try!(extracted.value.as_not_null_str());
        let keys: Vec<String> = keys.split(',').map(|s| s.to_string()).collect();
        cur_pos += extracted.bytes_read;

        // Number of items in this hdata
        let extracted = try!(extract_int(&data[cur_pos..]));
        let num_hdata_items = try!(extracted.value.as_integer());
        cur_pos += extracted.bytes_read;

        // Store pointers and keys for each item
        let mut data_list: Vec<HashMap<String, DataType>> = Vec::new();
        for _ in 0..num_hdata_items {
            let mut key_value_map: HashMap<String, DataType> = HashMap::new();

            // Pull out path pointers
            for path_name in &paths {
                let extracted = try!(extract_pointer(&data[cur_pos..]));
                key_value_map.insert(path_name.clone(), extracted.value);
                cur_pos += extracted.bytes_read;
            }

            // Pull out the data for all of the keys
            for key in &keys {
                let key_parse: Vec<&str> = key.split(':').collect();
                let key_name = key_parse[0];
                let key_type = key_parse[1];
                let extracted = match key_type {
                    "arr" => try!(extract_array(&data[cur_pos..])),
                    "buf" => try!(extract_buffer(&data[cur_pos..])),
                    "chr" => try!(extract_char(&data[cur_pos..])),
                    "int" => try!(extract_int(&data[cur_pos..])),
                    "lon" => try!(extract_long(&data[cur_pos..])),
                    "ptr" => try!(extract_pointer(&data[cur_pos..])),
                    "str" => try!(extract_string(&data[cur_pos..])),
                    "tim" => try!(extract_time(&data[cur_pos..])),
                    _     => return Err(ParseError("Bad type for key".to_string())),
                };
                key_value_map.insert(String::from(key_name), extracted.value);
                cur_pos += extracted.bytes_read;
            }

            // And finally, add this item to the hdata list
            data_list.push(key_value_map);
        }

        Ok(HData {
            data: data_list,
        })
    }
}
