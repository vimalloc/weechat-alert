mod conversions;
mod message_data;
mod errors;
mod hdata;
mod message_body;
mod message_header;
mod relay;

use relay::Relay;

fn main() {
    // TODO move these into a conf file somewhere
    let host = String::from("weechat.vimalloc.com");
    let port = 8001;
    let password = String::from("porter2pears");

    // Run our program
    let relay =  Relay::new(host, port, password);
    match relay.run() {
        Err(e) => println!("Error: {}", e),
        Ok(_) => ()
    }
}
