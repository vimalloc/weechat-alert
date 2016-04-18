

mod weechat {
    use std::time::Duration;
    use std::net::TcpStream;
    use std::io::Write;
    use std::thread;

    pub struct Relay {
        host: String,
        port: i32,
        password: String,
        reconnect_on_error: bool,
        stream: TcpStream,
    }

    impl Relay {
        pub fn new(host: String, port: i32, password: String) -> Relay {
            let addr = format!("{}:{}", host, port);
            let stream = TcpStream::connect(&*addr).unwrap();
            Relay {
                host: host,
                port: port,
                password: password,
                reconnect_on_error: false,
                stream: stream
            }
        }

        fn send_cmd(&mut self, cmd_str: String) {
            // Relay must end in \n per spec
            let mut cmd = cmd_str.clone();
            if !cmd.ends_with("\n") {
                cmd.push('\n');
            }
            let _ = self.stream.write_all(cmd.as_bytes());
        }

        fn init_relay(&mut self) {
            self.send_cmd(String::from("init password=mypass,compression=off"));
        }

        fn close_relay(&mut self) {
            self.send_cmd(String::from("quit"));
        }

        pub fn run(&mut self) {
            self.init_relay();
            thread::sleep(Duration::from_millis(5000));
            self.close_relay();
        }
    }
}



fn main() {
    // TODO move these into a conf file somewhere
    let host = String::from("weechat.vimalloc.com");
    let port = 8001;
    let password = String::from("porter2pears");
    let mut relay = weechat::Relay::new(host, port, password);
    relay.run()
}
