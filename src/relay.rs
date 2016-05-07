
// TODO handle if it receives an invalid password

mod weechat {
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
        stream: TcpStream,
    }

    struct MessageHeader {
        length: usize,
        compression: bool,
    }

    struct MessageData {
        identifier: String,
        data: DataType,
    }

    enum DataType {
        StrData(String),
        Hdata(i32),  // TODO build an hdata struct
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
                WeechatError::BadPassword => write!(f, "Invalid username or password"),
                WeechatError::NoDataHandler(ref s) => write!(f, "No handler found for {}", s)
            }
		}
    }

    impl Error for WeechatError {
        fn description(&self) -> &str {
            match *self {
                WeechatError::Io(ref err)          => err.description(),
                WeechatError::BadPassword          => "Invalid username or password",
                WeechatError::NoDataHandler(ref s) =>  "No handler found"
            }
        }
    }

    /// Converts a 4 byte array slice into a 32 bit signed integer. The bytes
    /// are assumed to be encoded in a big-endian format
    fn bytes_to_int(byte_array: &[u8]) -> i32 {
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

    impl Relay {
        pub fn new(host: String, port: i32, password: String) -> Result<Relay, WeechatError> {
            // TODO match stream and return weechat error if something is wrong
            let stream = try!(Relay::connect_relay(host.as_ref(), port));
            let mut relay = Relay {
                host: host,
                port: port,
                password: password,
                stream: stream
            };

            // TODO this should return a WeechatRelay error
            relay.init_relay();
            return Ok(relay);
        }

        fn connect_relay(host: &str, port: i32) -> io::Result<TcpStream> {
            // The initial tpc connection to the server
            let addr = format!("{}:{}", host, port);
            TcpStream::connect(&*addr)
        }

        fn send_cmd(&mut self, mut cmd_str: String) {
            // Relay must end in \n per spec
            if !cmd_str.ends_with("\n") {
                cmd_str.push('\n');
            }
            let _ = self.stream.write_all(cmd_str.as_bytes());
        }

        fn recv_msg(&mut self) -> io::Result<MessageData> {
            // header is first 5 bytes. The first 4 are the length, and the last
            // one is if compression is enabled or not
            let mut buffer = [0; HEADER_LENGTH];
            try!(self.stream.read_exact(&mut buffer));
            let header = MessageHeader::new(&buffer);

            // Now that we have the header, get the rest of the message.
            let mut data = vec![0; header.length];
            try!(self.stream.read_exact(data.as_mut_slice()));
            Ok(MessageData::new(data.as_slice()))
        }


        // TODO has to return nothing or a weechat error
        fn init_relay(&mut self) {
            // If initing the relay failed (due to a bad password) the protocol
            // will not actually send us a message saying that, it will just
            // silently disconnect the socket. To check this, we will do a ping
            // pong right after initing, which if the password is bad should
            // result in no bytes being read from the socket (UnexpectedEof)
            let cmd_str = format!("init password={},compression=off", self.password);
            self.send_cmd(cmd_str);
            let ping_msg = "foobarbaz";
            self.ping(ping_msg);
            let result = self.recv_msg();

            // We don't really need to check that the ping data is correct here,
            // but it doesn't hurt anything (and this match statement is neat!)
            match result {
                Err(e) => match e.kind() {
                    io::ErrorKind::UnexpectedEof => panic!("Bad password"),
                    _                            => panic!("{}", e),
                },
                Ok(msg_data) => {
                    match msg_data.identifier.as_ref() {
                        "_pong" => {},
                        _       => panic!("Did not receive pong"),
                    }
                    match msg_data.data {
                        DataType::StrData(ref s) if s == ping_msg  => {},
                        DataType::StrData(ref s)                   => panic!("bad pong msg: {}", s),
                        DataType::Hdata(_)                         => panic!("Pong received hdata"),
                    }
                }
            }
        }

        fn close_relay(&mut self) {
            let cmd_str = String::from("quit");
            self.send_cmd(cmd_str);
        }

        fn ping(&mut self, msg: &str) {
            let cmd_str = String::from(format!("ping {}", msg));
            self.send_cmd(cmd_str);
        }

        pub fn run(&mut self) {
            self.init_relay();
            /*
            while true {
                recv();
                if recv failed, try to reconnect
                else handle recv
            }
            */
            self.close_relay();
        }
    }

    impl MessageHeader {
        pub fn new(data: &[u8]) -> MessageHeader {
            // Headers has length of full message, we need to chop off the
            // legth of the header as we have already read that from the socket
            let total_msg_length = bytes_to_int(&data[0..4]);
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

        pub fn new(data: &[u8]) -> MessageData {
            // First 4 bytes are the integer length of the command name
            let identifier_length = bytes_to_int(&data[0..4]);
            let start_pos = 4;
            let end_pos = 4 + identifier_length as usize;
            let identifier = from_utf8(&data[start_pos..end_pos]).unwrap();

            // The rest of the received data that needs to be sent to the handler
            let cmd_data = &data[end_pos..];

            // Parse out the data for this message
            let dt = match identifier {
                "_pong" => { MessageData::parse_pong(&cmd_data) }
                _       => { panic!(format!("unsupported command: {}", identifier)); }
            };

            // Return our struct
            MessageData {
                identifier: String::from(identifier),
                data: dt,
            }
        }

        fn parse_pong(data: &[u8]) -> DataType {
            let result: String = MessageData::extract_string(data);
            DataType::StrData(result)
        }

        /// Given a byte array which contains an encoded str, pull the string out
        /// and return it. The protocol from strings are:
        ///
        /// bytes 0 - 3: "str"
        /// bytes 3 - 7: signed integer, size of string
        /// bytes 7 - ?: The actual string message
        ///
        /// Note: An empty string is valid, in this cass length will be 0. A NULL
        ///       string is also valid, it has length of -1.
        fn extract_string(data: &[u8]) -> String {
            // Sanity checks
            assert!(data.len() >= 7, "Not enough bytes in array to extract string");
            let obj_type = from_utf8(&data[0..3]).unwrap();
            assert!(obj_type == "str");

            // Get the start and end limits for this string
            let str_size = bytes_to_int(&data[3..7]);
            let start_pos = 7 as usize;
            let end_pos = start_pos + str_size as usize;

            // Pull out and return the string
            let data_str = match str_size {
                0  => "",  // Empty string
                -1 => "",  // Null string TODO how to encode the idea of this?
                _  => from_utf8(&data[start_pos..end_pos]).unwrap(),
            };
            String::from(data_str)
        }
    }
}



fn main() {
    // TODO move these into a conf file somewhere
    let host = String::from("weechat.vimalloc.com");
    let port = 8001;
    let password = String::from("porter22pears");

    // Needs to be mutable cause the underlying TcpStream must be mutable
    let mut relay = weechat::Relay::new(host, port, password).unwrap();
    relay.run();
}
