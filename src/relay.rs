
// TODO handle if it receives an invalid password

mod weechat {
    use std::io::prelude::*;
    use std::time::Duration;
    use std::net::TcpStream;
    use std::str::from_utf8;
    use std::thread;
    use std::mem;
    use std::str;

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

    /// Converts a 4 byte array slice into a 32 bit signed integer. The bytes
    /// are assumed to be encoded in a big-endian format
    fn bytes_to_int(byte_array: &[u8]) -> i32 {
        assert!(byte_array.len() == 4, "Array isn't 4 bytes, cannot cast to int");

        // Re-arrange bytes from little to big-endian (so we can transmute them)
        let mut bytes: [u8; 4] = [0, 0, 0, 0];
        bytes[0] = byte_array[3];
        bytes[1] = byte_array[2];
        bytes[2] = byte_array[1];
        bytes[3] = byte_array[0];

        // Do the casting
        let i: i32;
        unsafe {
            i = mem::transmute::<[u8; 4], i32>(bytes);
        }
        return i;
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
    fn extract_string(data: &[u8]) -> &str {
        assert!(data.len() >= 7, "Not enough bytes in array to extract string");
        let obj_type = from_utf8(&data[0..3]).unwrap();
        assert!(obj_type == "str");
        let str_size = bytes_to_int(&data[3..7]);

        if str_size == 0 {
            return "";
        } else if str_size == -1 {
            return "";  // TODO how would we want to encode the idea of a null string?
        } else {
            let end_pos = 7 + str_size as usize;
            return from_utf8(&data[7..end_pos]).unwrap();
        }
    }

    impl Relay {
        pub fn new(host: String, port: i32, password: String) -> Relay {
            let addr = format!("{}:{}", host, port);
            let stream = TcpStream::connect(&*addr).unwrap();
            Relay {
                host: host,
                port: port,
                password: password,
                stream: stream
            }
        }

        fn send_cmd(&mut self, mut cmd_str: String) {
            // Relay must end in \n per spec
            if !cmd_str.ends_with("\n") {
                cmd_str.push('\n');
            }
            let _ = self.stream.write_all(cmd_str.as_bytes());
        }

        fn recv_msg(&mut self) {
            // header is first 5 bytes. The first 4 are the length, and the last
            // one is if compression is enabled or not
            let mut buffer = [0; HEADER_LENGTH];
            let _ = self.stream.read_exact(&mut buffer);
            let header = MessageHeader::new(&buffer);
            println!("Length is {} and compression is {}", header.length, header.compression);

            // Now that we have the header, get the rest of the message.
            let mut data = vec![0; header.length];
            let _ = self.stream.read_exact(data.as_mut_slice());
            self.parse_cmd(data.as_slice());
            /*
            println!("data length is {}", data.len());
            for byte in data {
                println!("received: {}", byte);
            }
            */
        }

        fn parse_cmd(&mut self, data: &[u8]) {
            // First 4 bytes are the integer length of the command name
            let command_name_length = bytes_to_int(&data[0..4]);
            let start_pos = 4;
            let end_pos = 4 + command_name_length as usize;
            let command_name = from_utf8(&data[start_pos..end_pos]).unwrap();

            // The rest of the received data that needs to be sent to the handler
            let cmd_data = &data[end_pos..];

            // Subsequent bytes depend on what the command is
            match command_name {
                "_pong" => { self.handle_pong(&cmd_data); }
                _       => { panic!(format!("unsupported command: {}", command_name)); }
            }

            // TODO build some generic or wrapper struct where we can return
            //      anything that we need to from this method (string, struct,
            //      int, whatever)
        }

        fn handle_pong(&mut self, data: &[u8]) {
            // TODO actually return this (see note above in parse_cmd)
            let result = extract_string(data);
            println!("received pong: {}", result);
        }

        fn init_relay(&mut self) {
            // If init failed, the protocol wont say anyting. Try doing a
            // ping->pong right now, and if that disconnects the socket then
            // the password failed
            let cmd_str = format!("init password={},compression=off", self.password);
            self.send_cmd(cmd_str);
            self.ping();
            self.recv_msg();
        }

        fn close_relay(&mut self) {
            let cmd_str = String::from("quit");
            self.send_cmd(cmd_str);
        }

        fn ping(&mut self) {
            let cmd_str = String::from("ping foobar");
            self.send_cmd(cmd_str);
        }

        pub fn run(&mut self) {
            self.init_relay();
            //thread::sleep(Duration::from_millis(5000));
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
            let compression;
            let compression_byte = data[4];
            if compression_byte == 0 {
                compression = false;
            } else if compression_byte == 1 {
                compression = true;
            } else {
                panic!("Compression byte is not 0 or 1");
            }

            // Create the struct
            MessageHeader {
                length: length,
                compression: compression,
            }
        }
    }
}



fn main() {
    // TODO move these into a conf file somewhere
    let host = String::from("weechat.vimalloc.com");
    let port = 8001;
    let password = String::from("porter2pears");

    // Needs to be mutable cause the underlying TcpStream must be mutable
    let mut relay = weechat::Relay::new(host, port, password);
    relay.run()
}
