

mod weechat {
    use std::collections::HashMap;
    use std::net::Shutdown;
    use std::io::prelude::*;
    use std::net::TcpStream;
    use std::str::from_utf8;
    use std::mem;
    use std::io;
    use std::error::Error;
    use std::fmt;

    const HEADER_LENGTH: usize = 5;

    pub struct Relay {
        host: String,
        port: i32,
        password: String,
    }

    struct MessageHeader {
        length: usize,
        compression: bool,
    }

    struct MessageData {
        identifier: String,
        data: MessageType,
    }

    enum MessageType {
        StrData(Option<String>),
        HData(HData),
    }

    struct HData {
        paths: Vec<PPath>,
        keys: HashMap<String, DataType>
    }

    struct PPath {
        path: String,
        pointer: String,
    }

    enum DataType {
        Buf(String),        // TODO make this a vector of u8 bytes instead
        Chr(char),
        Int(i32),
        Lon(i64),
        Ptr(Option<String>),
        Str(Option<String>),
        Tim(i32),
        Arr(Vec<DataType>),
    }

    struct ExtractedData {
        value: DataType,
        bytes_read: usize,
    }

    #[derive(Debug)]
    pub enum WeechatError {
        Io(io::Error),  // Errors reading, writing, or connecting to socket
        BadPassword,    // Bad password for weechat init protocol
        NoDataHandler(String),  // Received data we don't know how to deal with
    }

    impl From<io::Error> for WeechatError {
        fn from(err: io::Error) -> WeechatError {
            WeechatError::Io(err)
        }
    }

    impl fmt::Display for WeechatError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                WeechatError::Io(ref err) => err.fmt(f),
                WeechatError::BadPassword => write!(f, "Invalid password"),
                WeechatError::NoDataHandler(ref s) => write!(f, "No handler found for {}", s)
            }
        }
    }

    impl Error for WeechatError {
        fn description(&self) -> &str {
            match *self {
                WeechatError::Io(ref err)      => err.description(),
                WeechatError::BadPassword      => "Invalid username or password",
                WeechatError::NoDataHandler(_) =>  "No handler found"
            }
        }
    }

    /// Converts a 4 byte array slice into a 32 bit signed integer. The bytes
    /// are assumed to be encoded in a big-endian format
    fn bytes_to_i32(byte_array: &[u8]) -> i32 {
        assert!(byte_array.len() == 4, "Array isn't 4 bytes, cannot cast to int");

        // Re-arrange bytes from big to little-endian (so we can transmute them)
        let mut bytes: [u8; 4] = [0, 0, 0, 0];
        bytes[0] = byte_array[3];
        bytes[1] = byte_array[2];
        bytes[2] = byte_array[1];
        bytes[3] = byte_array[0];

        // Do the casting
        unsafe {
            mem::transmute::<[u8; 4], i32>(bytes)
        }
    }

    // TODO for all these methods, maybe it would be clearer or make more sense
    //      if we passed the start position (and end pos?) in as args, instead of
    //      sending in smaller array slices to these functions? I don't know right
    //      off hand, but it's somethign to think on

    /// Given a byte array which contains an encoded str, pull the string out
    /// and return it. The protocol from strings are:
    ///
    /// bytes 0 - 4: signed integer, size of string
    /// bytes 4 - ?: The actual string message
    ///
    /// Note: An empty string is valid, in this cass length will be 0. A NULL
    ///       string is also valid, it has length of -1. This is encoded in the
    ///       ExtractedData struct
    fn extract_string(data: &[u8]) -> ExtractedData {
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
    fn extract_pointer(data: &[u8]) -> ExtractedData {
        assert!(data.len() >= 2, "Not enough bytes in array to extract pointer");
        let ptr_size = data[0] as i8;
        let start = 1;
        let end = start + ptr_size as usize;
        assert!(data.len() >= end, "Not enough bytes in array to extract string");

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

    /// Given a byte array which contains an encodec char, pull the char out.
    ///
    /// Returns a tuple, where the first element is the character read and the
    /// second element is how much data was read from the byte array
    fn extract_char(data: &[u8]) -> ExtractedData {
        assert!(data.len() >= 1, "Not enough bytes in array to extract char");
        ExtractedData {
            value: DataType::Chr(data[0] as char),
            bytes_read: 1,

        }
    }

    impl Relay {
        pub fn new(host: String, port: i32, password: String) -> Relay {
             Relay {
                host: host,
                port: port,
                password: password,
            }
        }

        fn connect_relay(&self) -> Result<TcpStream, WeechatError> {
            // The initial tpc connection to the server
            let addr = format!("{}:{}", self.host, self.port);
            match TcpStream::connect(&*addr) {
                Ok(stream) => Ok(stream),
                Err(e)     => Err(WeechatError::Io(e))
            }
        }

        fn send_cmd(&self, mut stream: &TcpStream, mut cmd_str: String) -> Result<(), WeechatError> {
            // Relay must end in \n per spec
            if !cmd_str.ends_with("\n") {
                cmd_str.push('\n');
            }
            try!(stream.write_all(cmd_str.as_bytes()));
            Ok(())
        }

        fn recv_msg(&self, mut stream: &TcpStream) -> Result<MessageData, WeechatError> {
            // header is first 5 bytes. The first 4 are the length, and the last
            // one is if compression is enabled or not
            let mut buffer = [0; HEADER_LENGTH];
            try!(stream.read_exact(&mut buffer));
            let header = MessageHeader::new(&buffer);

            // Now that we have the header, get the rest of the message.
            let mut data = vec![0; header.length];
            try!(stream.read_exact(data.as_mut_slice()));
            MessageData::new(data.as_slice())
        }

        fn init_relay(&self, stream: &TcpStream) -> Result<(), WeechatError> {
            // If initing the relay failed (due to a bad password) the protocol
            // will not actually send us a message saying that, it will just
            // silently disconnect the socket. To check this, we will do a ping
            // pong right after initing, which if the password is bad should
            // result in no bytes being read from the socket (UnexpectedEof)
            let cmd_str = format!("init password={},compression=off", self.password);
            try!(self.send_cmd(stream, cmd_str));
            let _ = self.send_cmd(stream, String::from("ping foo"));

            // UnexpectedEof means that a bad password was sent in. Any other
            // error is something unexpected, so just bail out for now. If it
            // is an IoError, pass it back to the caller so they can deal wtih
            // it. If it's anything else, it should never happen, so it likely
            // indicates a bug in our program. Panic it
            match self.recv_msg(stream) {
                Err(e) => match e {
                    WeechatError::BadPassword      => panic!("BadPassword should not exist here"),
                    WeechatError::NoDataHandler(_) => panic!("NoDataHandler should not exist here"),
                    WeechatError::Io(err)          => match err.kind() {
                        io::ErrorKind::UnexpectedEof => Err(WeechatError::BadPassword),
                        _                            => Err(WeechatError::Io(err)),
                    },
                },
                Ok(msg_data) => {
                    match msg_data.identifier.as_ref() {
                        "_pong" => {},
                        _       => panic!("Received something besides pong after init"),
                    }
                    match msg_data.data {
                        MessageType::StrData(Some(ref s)) if s == "foo" => Ok(()),
                        MessageType::StrData(Some(ref s))               => panic!("bad pong msg {}", s),
                        MessageType::StrData(None)                      => panic!("Null pong msg"),
                        MessageType::HData(_)                           => panic!("Pong received hdata"),
                    }
                }
            }
        }

        /// Tell weechat we are done, and close our socket. TcpStream can no
        /// longer be used after a call to close_relay. Any errors here are ignored
        fn close_relay(&self, mut stream: &TcpStream) {
            let cmd_str = String::from("quit");
            let _ = self.send_cmd(stream, cmd_str);
            let _ = stream.flush();
            let _ = stream.shutdown(Shutdown::Both);
        }

        fn run_loop(&self, stream: &TcpStream) -> Result<(), WeechatError> {
            try!(self.init_relay(stream));

            // We only need to sync buffers to get highlights. We don't need
            // nicklist or anything like that
            let cmd_str = String::from("sync * buffer");
            try!(self.send_cmd(stream, cmd_str));

            loop {
                let mgs = try!(self.recv_msg(stream));
            }
        }

        pub fn run(&self) -> Result<(), WeechatError> {
            let stream = &try!(self.connect_relay());
            let result = self.run_loop(stream);
            self.close_relay(stream);
            result
        }
    }

    impl MessageHeader {
        pub fn new(data: &[u8]) -> MessageHeader {
            // Headers has length of full message, we need to chop off the
            // legth of the header as we have already read that from the socket
            let total_msg_length = bytes_to_i32(&data[0..4]);
            let length = total_msg_length as usize - HEADER_LENGTH;

            // Pull compression out of bytes, and verify it's 1 or 0
            let compression = match data[4] {
                0 => false,
                1 => true,
                _ => panic!("Compression byte is neither 0 or 1"),
            };

            // Create the struct
            MessageHeader {
                length: length,
                compression: compression,
            }
        }
    }

    impl MessageData {

        pub fn new(data: &[u8]) -> Result<MessageData, WeechatError> {
            // First thing encoded in the binary data is the identifier for
            // what this command is
            let extracted = extract_string(data);
            let identifier = match extracted.value {
                DataType::Str(Some(s)) => s,
                _                      => panic!("identifier should be non-null DataType::Str"),
            };

            // Next 3 bytes determin type of data in this command (hdata or str).
            // Parse the data type depending
            let start = extracted.bytes_read;
            let end = start + 3;
            let msg_type = match from_utf8(&data[start..end]).unwrap() {
                "str" => MessageData::binary_to_strdata(&data[end..]),
                "hda" => MessageData::binary_to_hdata(&data[end..]),
                _     => panic!("Received unknown message type"),
            };

            // Return our struct
            Ok(MessageData {
                identifier: String::from(identifier),
                data: msg_type,
            })
        }

        fn binary_to_strdata(data: &[u8]) -> MessageType {
            let extracted = extract_string(data);
            match extracted.value {
                DataType::Str(s) => MessageType::StrData(s),
                _                => panic!("Extracted is not DataType::Str"),
            }
        }

        fn binary_to_hdata(data: &[u8]) -> MessageType {
            let mut cur_pos = 0; // Rolling counter of where we are in the byte array
            let mut ppaths = Vec::new(); // list of pointer path structs
            let mut key_value_map = HashMap::new();  // keys to value mapper

            // Parse out paths
            let extracted = extract_string(&data[cur_pos..]);
            cur_pos += extracted.bytes_read;
            let paths: Vec<String> = match extracted.value {
                DataType::Str(Some(ref s)) => s.split(',').map(|s| String::from(s)).collect(),
                _                          => panic!("Paths should be non-null DataType::Str"),
            };

            // Parse out key names and types
            let extracted = extract_string(&data[cur_pos..]);
            cur_pos += extracted.bytes_read;
            let keys: Vec<String> = match extracted.value {
                DataType::Str(Some(ref s)) => s.split(',').map(|s| String::from(s)).collect(),
                _                          => panic!("Keys should be non-null DataType::Str"),
            };

            // Number of items in this hdata
            let num_hdata_items = bytes_to_i32(&data[cur_pos..cur_pos+4]);
            cur_pos += 4;

            // Pull out path pointers
            for path in paths {
                let extracted = extract_pointer(&data[cur_pos..]);
                cur_pos += extracted.bytes_read;
                match extracted.value {
                    DataType::Ptr(Some(p)) => ppaths.push(PPath{
                                                             path: String::from(path),
                                                             pointer: p
                                                          }),
                    _                      => panic!("Pointer should be not-null DataType::Ptr"),
                };
            }

            // Finally, we pull out the data for all of the keys that we have
            for key in keys {
                let key_parse: Vec<&str> = key.split(':').collect();
                let key_name = key_parse[0];
                let key_type = key_parse[1];

                let value = match key_type {
                    "chr" => DataType::Chr('a'),
                    "int" => DataType::Int(1),
                    "lon" => DataType::Lon(1),
                    "str" => DataType::Str(Some(String::from("foobar"))),
                    "buf" => DataType::Buf(String::from("foobar")),
                    "ptr" => DataType::Ptr(Some(String::from("1a2b3d4d5"))),
                    "tim" => DataType::Tim(1321993456),
                    "arr" => DataType::Arr(Vec::new()),
                    _     => panic!("Received invalid key type"),
                };
                key_value_map.insert(String::from(key_name), value);
            }

            // Debug, see what the rest of the data looks like
            /*
            println!("Start byets:");
            for byte in &data[start..end] {
                print!("{} ", byte);
            }
            println!("\nByets finished!\n\n");
            */

            MessageType::HData(HData {
                paths: ppaths,
                keys: key_value_map,
            })
        }
    }
}

fn main() {
    // TODO move these into a conf file somewhere
    let host = String::from("weechat.vimalloc.com");
    let port = 8001;
    let password = String::from("porter2pears");

    // Run our program
    let relay =  weechat::Relay::new(host, port, password);
    match relay.run() {
        Err(e) => println!("Error: {}", e),
        Ok(_) => ()
    }
}
