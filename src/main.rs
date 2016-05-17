use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::exit;

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
    // Get the path of the config file
    let homedir = match env::home_dir() {
        Some(path) => path,
        None       => {
            println!("Error: Could not find home directory");
            exit(1);
        }
    };
    let mut config_path = PathBuf::from(homedir);
    config_path.push(".relay");
    config_path.set_extension("toml");

    // Open the config file
    let mut file = match File::open(&config_path) {
        Ok(file) => file,
        Err(why) => {
            println!("Error opening {}: {}", config_path.display(),
                                             Error::description(&why));
            exit(1);
        },
    };

    // Read the contents of the config file into memory
    let mut config_str = String::new();
    match file.read_to_string(&mut config_str) {
        Ok(_)    => (),
        Err(why) => {
            println!("Couldn't read {}: {}", config_path.display(),
                                             Error::description(&why));
            exit(1);
        }
    }

    // Parse the toml
    let config: toml::Value = match config_str.parse() {
        Ok(config) => config,
        Err(errs)   => {
            println!("Error Parsing config file:");
            for err in &errs {
                println!("  {}", Error::description(err));
            }
            exit(1);
        }
    };

    let host = match config.lookup("relay.server") {
        Some(host) => match host.as_str() {
            Some(s) => s.to_string(),
            None    => {
                println!("Error: 'server' option is not a valid string");
                exit(1);
            }
        },
        None       => {
            println!("Error: 'server' option not found under [relay] block");
            exit(1);
        }
    };

    let password = match config.lookup("relay.password") {
        Some(pw) => match pw.as_str() {
            Some(s) => s.to_string(),
            None    => {
                println!("Error: 'password' option is not a valid string");
                exit(1);
            }
        },
        None       => {
            println!("Error: 'password' option not found under [relay] block");
            exit(1);
        }
    };

    let port = match config.lookup("relay.port") {
        Some(port) => match port.as_integer() {
            Some(i) => i as i32,
            None    => {
                println!("Error: 'port' option is not an integer");
                exit(1);
            }
        },
        None       => {
            println!("Error: 'port' option not found under [relay] block");
            exit(1);
        }
    };

    // Run our program
    let relay =  Relay::new(host, port, password);
    match relay.run() {
        Err(e) => println!("Error: {}", e),
        Ok(_) => ()
    }
}
