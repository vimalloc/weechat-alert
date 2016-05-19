use std::io::prelude::*;
use std::net::Shutdown;
use std::net::TcpStream;
use std::thread;
use std::io;

use ears::{Sound, AudioController};

use openssl::ssl::{Ssl, SslMethod, SslContext, SslStream, SSL_VERIFY_NONE, SSL_VERIFY_PEER};

use errors::WeechatError;
use hdata::HData;
use message;

// number of bytes that make up the message header
const HEADER_LENGTH: usize = 5;

/// Holds relay connection information
pub struct Relay {
    host: String,
    port: i32,
    password: String,
}

enum StreamType {
    Tcp(TcpStream),
    Ssl(SslStream<TcpStream>),
}

impl Relay {
    pub fn new(host: String, port: i32, password: String) -> Relay {
         Relay {
            host: host,
            port: port,
            password: password,
        }
    }

    fn connect_relay(&self) -> Result<TcpStream, WeechatError> {
        // The initial tpc connection to the server
        let addr = format!("{}:{}", self.host, self.port);
        match TcpStream::connect(&*addr) {
            Ok(stream) => Ok(stream),
            Err(e)     => Err(WeechatError::Io(e))
        }
    }

    fn send_cmd(&self, stream: &mut SslStream<TcpStream>, mut cmd_str: String) -> Result<(), WeechatError> {
        // Relay must end in \n per spec
        if !cmd_str.ends_with("\n") {
            cmd_str.push('\n');
        }
        try!(stream.write_all(cmd_str.as_bytes()));
        Ok(())
    }

    fn recv_msg(&self, stream: &mut SslStream<TcpStream>) -> Result<message::Message, WeechatError> {
        // header is first 5 bytes. The first 4 are the length, and the last
        // one is if compression is enabled or not
        let mut buffer = [0; HEADER_LENGTH];
        try!(stream.read_exact(&mut buffer));
        let header = try!(message::Header::new(&buffer));

        // Now that we have the header, get the rest of the message.
        let mut data = vec![0; header.length];
        try!(stream.read_exact(data.as_mut_slice()));
        message::Message::new(data.as_slice())
    }

    fn init_relay(&self, stream: &mut SslStream<TcpStream>) -> Result<(), WeechatError> {
        // If initing the relay failed (due to a bad password) the protocol
        // will not actually send us a message saying that, it will just
        // silently disconnect the socket. To check this, we will do a ping
        // pong right after initing, which if the password is bad should
        // result in no bytes being read from the socket (UnexpectedEof)
        let cmd_str = format!("init password={},compression=off", self.password);
        try!(self.send_cmd(stream, cmd_str));
        try!(self.send_cmd(stream, "ping".to_string()));

        // UnexpectedEof means that a bad password was sent in. Any other
        // error is something unexpected.
        match self.recv_msg(stream) {
            Err(e) => match e {
                WeechatError::Io(err) => match err.kind() {
                    io::ErrorKind::UnexpectedEof => Err(WeechatError::BadPassword),
                    _                            => Err(WeechatError::Io(err)),
                },
                _                     => Err(e)
            },
            Ok(_) =>  Ok(())
        }
    }

    /// Tell weechat we are done, and close our socket. TcpStream can no
    /// longer be used after a call to close_relay. Any errors here are ignored
    fn close_relay(&self, stream: &mut SslStream<TcpStream>) {
        let cmd_str = "quit".to_string();
        let _ = self.send_cmd(stream, cmd_str);
        let _ = stream.flush();
        let _ = stream.get_mut().shutdown(Shutdown::Both);
    }

    fn buffer_line_added(&self, hdata: &HData) {
        // Check if this line has a highlight or a private message that we
        // should notify on
        let mut play_sound = false;
        for data in &hdata.data {
            let highlight = data["highlight"].as_character().unwrap();
            if highlight == (1 as char) {
                play_sound = true;
                break;
            }

            let tags_array = data["tags_array"].as_array().unwrap();
            for element in tags_array {
                let tag_str = element.as_not_null_str().unwrap();
                if tag_str == "notify_private" {
                    play_sound = true;
                    break
                }
            }
        }

        // The play is a blocking call, and if we don't loop for is_playing it
        // seems to go out of scope and get destroyed before it can actually play
        // the sound. So we will spawn it in a new thread, so that we don't have
        // to wait x seconds for the sound to play before processing another
        // message.
        if play_sound {
            thread::spawn(move || {
                let mut snd = Sound::new("/home/lgbland/.weechat/noises/test.wav").unwrap();
                snd.play();
                while snd.is_playing() {}
            });
        }
    }

    fn run_loop(&self, stream: &mut SslStream<TcpStream>) -> Result<(), WeechatError> {
        try!(self.init_relay(stream));

        // We only need to sync buffers to get highlights. We don't need
        // nicklist or anything like that
        let cmd_str = "sync * buffer".to_string();
        try!(self.send_cmd(stream, cmd_str));

        loop {
            let msg = try!(self.recv_msg(stream));
            match msg.identifier.as_ref() {
                "_buffer_line_added" => self.buffer_line_added(try!(msg.as_hdata())),
                _                    => (),
            };
        }
    }

    pub fn run(&self) -> Result<(), WeechatError> {
        let mut ctx = SslContext::new(SslMethod::Sslv23).unwrap();
        ctx.set_verify(SSL_VERIFY_NONE, None);
        let ssl = Ssl::new(&ctx).unwrap();

        let stream = try!(self.connect_relay());
        let mut ssl_stream = SslStream::connect(ssl, stream).unwrap();

        let result = self.run_loop(&mut ssl_stream);
        self.close_relay(&mut ssl_stream);
        result
    }
}
