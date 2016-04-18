use std::net::TcpStream;

struct WeechatRelay{
    host: String,
    port: i32,
    password: String,
    reconnect_on_error: bool,
    stream: TcpStream,
}

impl WeechatRelay {
    fn new(host: String, port: i32, password: String) -> WeechatRelay {
        let addr = format!("{}:{}", host, port);
        let stream = TcpStream::connect(&*addr).unwrap();
        WeechatRelay {
            host: host,
            port: port,
            password: password,
            reconnect_on_error: false,
            stream: stream
        }
    }
}

fn main() {
    // TODO move these into a conf file somewhere
    let host = String::from("weechat.vimalloc.com");
    let port = 8001;
    let password = String::from("porter2pears");
    let relay = WeechatRelay::new(host, port, password);
    println!("Connecting to {}:{}", relay.host, relay.port);
}
