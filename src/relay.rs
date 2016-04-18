
// TODO handle if it receives an invalid password

mod weechat {
    use std::io::prelude::*;
    use std::time::Duration;
    use std::net::TcpStream;
    use std::thread;
    use std::mem;

    const HEADER_LENGTH: usize = 5;

    pub struct Relay {
        host: String,
        port: i32,
        password: String,
        stream: TcpStream,
    }

    struct MessageHeader {
        length: i32,
        compression: bool,
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
        }

        fn init_relay(&mut self) {
            let cmd_str = format!("init password={},compression=off", self.password);
            self.send_cmd(cmd_str);
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
            // If init failed, the protocol wont say anyting. Try doing a
            // ping->pong right now, and if that disconnects the socket then
            // the password failed
            self.init_relay();
            self.ping();
            self.recv_msg();

            thread::sleep(Duration::from_millis(5000));
            self.close_relay();
        }
    }

    impl MessageHeader {
        pub fn new(data: &[u8]) -> MessageHeader {
            // Pull length out of bytes and cast it to an int.
            // Reverse the endianness of the bits to get this working
            let length;
            let mut length_bytes: [u8; 4] = [0, 0, 0, 0];
            length_bytes[0] = data[3];
            length_bytes[1] = data[2];
            length_bytes[2] = data[1];
            length_bytes[3] = data[0];
            unsafe {
                length = mem::transmute::<[u8; 4], i32>(length_bytes);
            }


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
