use std::collections::HashMap;

use errors::WeechatError;
use errors::WeechatError::ParseError;
use message;
use parse::Parse;


/// A list of key/value mappings of data received from relay. This data conststs
/// of the paths and the keys declared in the weechat relay messages protocol:
/// https://weechat.org/files/doc/devel/weechat_relay_protocol.en.html
#[derive(Debug)]
pub struct HData {
    pub data: Vec<HashMap<String, message::Object>>
}


impl HData {
    /// Takes an array of bytes that encode an HData and returns a parsed HData
    /// object.
    ///
    /// This should not include the leading "hda" string that identifies
    /// the object as an HData, ie the bytes should start right after the
    /// identifying "hda" string.
    ///
    /// You can see the protocol for encoding an hdata object here:
    /// https://weechat.org/files/doc/devel/weechat_relay_protocol.en.html#object_hdata
    pub fn new(bytes: &[u8]) -> Result<HData, WeechatError> {
        let mut cur_pos = 0; // Rolling counter of where we are in the byte array

        // Parse out paths
        let parsed = try!(Parse::string(&bytes[cur_pos..]));
        let paths = try!(parsed.object.as_not_null_str());
        let paths: Vec<String> = paths.split(',').map(|s| s.to_string()).collect();
        cur_pos += parsed.bytes_read;

        // Parse out key names and types
        let parsed = try!(Parse::string(&bytes[cur_pos..]));
        let keys = try!(parsed.object.as_not_null_str());
        let keys: Vec<String> = keys.split(',').map(|s| s.to_string()).collect();
        cur_pos += parsed.bytes_read;

        // Number of items in this hdata
        let parsed = try!(Parse::integer(&bytes[cur_pos..]));
        let num_hdata_items = try!(parsed.object.as_integer());
        cur_pos += parsed.bytes_read;

        // Store pointers and keys for each item
        let mut data_list = Vec::new();
        for _ in 0..num_hdata_items {
            let mut key_value_map = HashMap::new();

            // Pull out path pointers
            for path_name in &paths {
                let parsed = try!(Parse::pointer(&bytes[cur_pos..]));
                key_value_map.insert(path_name.clone(), parsed.object);
                cur_pos += parsed.bytes_read;
            }

            // Pull out the data for all of the keys
            for key in &keys {
                let key_parse: Vec<&str> = key.split(':').collect();
                let key_name = key_parse[0];
                let key_type = key_parse[1];
                let parsed = match key_type {
                    "arr" => try!(Parse::array(&bytes[cur_pos..])),
                    "buf" => try!(Parse::buffer(&bytes[cur_pos..])),
                    "chr" => try!(Parse::character(&bytes[cur_pos..])),
                    "int" => try!(Parse::integer(&bytes[cur_pos..])),
                    "lon" => try!(Parse::long(&bytes[cur_pos..])),
                    "ptr" => try!(Parse::pointer(&bytes[cur_pos..])),
                    "str" => try!(Parse::string(&bytes[cur_pos..])),
                    "tim" => try!(Parse::time(&bytes[cur_pos..])),
                    _     => return Err(ParseError("Bad type for key".to_string())),
                };
                key_value_map.insert(String::from(key_name), parsed.object);
                cur_pos += parsed.bytes_read;
            }

            // And finally, add this item to the hdata list
            data_list.push(key_value_map);
        }

        Ok(HData {
            data: data_list,
        })
    }
}
