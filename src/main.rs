use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

extern crate ears;
extern crate toml;

mod conversions;
mod message_data;
mod errors;
mod hdata;
mod message_body;
mod message_header;
mod relay;

use relay::Relay;

fn main() {
    // Get and parse the config file
    let homedir = match env::home_dir() {
        Some(path) => path,
        None       => {
            println!("Error: Could not find home directory");
            return;
        }
    };
    let mut config_path = PathBuf::from(homedir);
    config_path.push(".relay");
    config_path.set_extension("toml");
    let mut file = match File::open(&config_path) {
        Ok(file) => file,
        Err(why) => {
            println!("Error opening {}: {}", config_path.display(),
                                             Error::description(&why));
            return;
        },
    };
    let mut config_str = String::new();
    match file.read_to_string(&mut config_str) {
        Ok(_)    => (),
        Err(why) => {
            println!("Couldn't read {}: {}", config_path.display(),
                                             Error::description(&why));
            return;
        }
    }
    let config = match toml::Parser::new(config_str.as_ref()).parse() {
        Some(config) => config,
        None         => {
            println!("Failed to parse config file. Verify it's valid toml");
            return
        }
    };

    // holy shit clean this up. Fucking terrible
    let relay_config = config.get("relay").unwrap();
    let host = relay_config.as_table().unwrap().get("server").unwrap().as_str().unwrap().to_string();
    let port = relay_config.as_table().unwrap().get("port").unwrap().as_integer().unwrap() as i32;
    let password = relay_config.as_table().unwrap().get("password").unwrap().as_str().unwrap().to_string();

    // Run our program
    let relay =  Relay::new(host, port, password);
    match relay.run() {
        Err(e) => println!("Error: {}", e),
        Ok(_) => ()
    }
}
