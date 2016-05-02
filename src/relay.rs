
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
        if byte_array.len() != 4 {
            panic!("byte array is not exactly 4 bytes, cannot cast to int");
        }

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

            // Subsequent bytes depend on what the command is
            println!("command_name is {}", command_name);
            // TODO
            // switch on command name
            // call parse method for given command
        }

        fn parse_pong(&mut self) -> String {
            // TODO will we always get a string back that is the command name?
            //      or is that only for pong
            // str_len (4 bytes)
            // _pong
            // str_len (4 bytes)
            // message
            return String::from("foobar");
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
