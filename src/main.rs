use std::env;
use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::process::exit;

extern crate ears;
extern crate openssl;
extern crate toml;

mod message;
mod errors;
mod hdata;
mod parse;
mod relay;
mod strdata;

use relay::{Relay, SslConfig};


struct Config {
    host: String,
    port: i32,
    password: String,
    ssl: bool,
    ssl_verify: bool,
    ca_certs_path: Option<String>
}

fn parse_config() -> Result<Config, String> {
    // Get config filepath
    let homedir = try!(env::home_dir().ok_or("Cannot find home directory"));
    let mut path = PathBuf::from(homedir);
    path.push(".relay");
    path.set_extension("toml");

    // Open the file and read the data
    let mut file = try!(File::open(&path).map_err(|e| format!("{}: {}", path.display(), e)));
    let mut file_data = String::new();
    try!(file.read_to_string(&mut file_data).map_err(|e| format!("{}: {}", path.display(), e)));

    // Parse the config
    let config: toml::Value = try!(file_data.parse().map_err(|errs| {
        let mut err = "Error parsing config file:".to_string();
        for e in errs {
            err.push_str("\n  ");
            err.push_str(Error::description(&e));
        }
        err
    }));

    // Get data and return
    let host = try!(config.lookup("server").ok_or("'server' not found in the config file"));
    let host = try!(host.as_str().map(|s| s.to_string()).ok_or("'server' is not a valid string"));

    let pw = try!(config.lookup("password").ok_or("'password' not found in the config file"));
    let pw = try!(pw.as_str().map(|s| s.to_string()).ok_or("'password' is not a valid string"));

    let port = try!(config.lookup("port").ok_or("'port' not found in the config file"));
    let port = try!(port.as_integer().map(|s| s as i32).ok_or("'port' is not an integer"));

    let default_ssl = toml::Value::Boolean(false);
    let ssl = config.lookup("ssl").unwrap_or(&default_ssl);
    let ssl = try!(ssl.as_bool().ok_or("'ssl' is not true or false"));

    let default_ssl_verify = toml::Value::Boolean(false);
    let ssl_verify = config.lookup("ssl_verify").unwrap_or(&default_ssl_verify);
    let ssl_verify = try!(ssl_verify.as_bool().ok_or("'ssl_verify' is not a true or false"));

    let ca_certs = match config.lookup("ca_certs_path") {
        Some(ca) => Some(try!(ca.as_str().map(|s| s.to_string())
                         .ok_or("'ca_certs_path' is not a valid string"))),
        None     => None
    };

    Ok(Config {
        host: host,
        port: port,
        password: pw,
        ssl: ssl,
        ssl_verify: ssl_verify,
        ca_certs_path: ca_certs,
    })
}

fn main() {
    // Parse config
    let config = match parse_config() {
        Ok(config) => config,
        Err(e)     => {
            println!("Error: {}", e);
            exit(1);
        }
    };

    // Handle ssl if its configured
    let ssl = if config.ssl == true {
        SslConfig::new(config.ssl_verify, config.ca_certs_path)
    } else {
        None
    };

    // Run our program
    let relay =  Relay::new(config.host, config.port, config.password, ssl);
    match relay.run() {
        Err(e) => println!("Error: {}", e),
        Ok(_) => ()
    }
}
