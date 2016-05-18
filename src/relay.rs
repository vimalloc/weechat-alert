use std::io::prelude::*;
use std::net::Shutdown;
use std::net::TcpStream;
use std::thread;
use std::io;

use ears::{Sound, AudioController};

use errors::WeechatError;
use message_header::MessageHeader;
use message_body::{MessageData, MessageType};
use message_data::DataType;

// TODO put this in just one place, or hand off actually reading data from the
//      socket to message header
const HEADER_LENGTH: usize = 5;

/// Holds relay connection information
pub struct Relay {
    host: String,
    port: i32,
    password: String,
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

    fn send_cmd(&self, mut stream: &TcpStream, mut cmd_str: String) -> Result<(), WeechatError> {
        // Relay must end in \n per spec
        if !cmd_str.ends_with("\n") {
            cmd_str.push('\n');
        }
        try!(stream.write_all(cmd_str.as_bytes()));
        Ok(())
    }

    fn recv_msg(&self, mut stream: &TcpStream) -> Result<MessageData, WeechatError> {
        // header is first 5 bytes. The first 4 are the length, and the last
        // one is if compression is enabled or not
        let mut buffer = [0; HEADER_LENGTH];
        try!(stream.read_exact(&mut buffer));
        let header = try!(MessageHeader::new(&buffer));

        // Now that we have the header, get the rest of the message.
        let mut data = vec![0; header.length];
        try!(stream.read_exact(data.as_mut_slice()));
        MessageData::new(data.as_slice())
    }

    fn init_relay(&self, stream: &TcpStream) -> Result<(), WeechatError> {
        // If initing the relay failed (due to a bad password) the protocol
        // will not actually send us a message saying that, it will just
        // silently disconnect the socket. To check this, we will do a ping
        // pong right after initing, which if the password is bad should
        // result in no bytes being read from the socket (UnexpectedEof)
        let cmd_str = format!("init password={},compression=off", self.password);
        try!(self.send_cmd(stream, cmd_str));
        let _ = self.send_cmd(stream, String::from("ping"));

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
    fn close_relay(&self, mut stream: &TcpStream) {
        let cmd_str = String::from("quit");
        let _ = self.send_cmd(stream, cmd_str);
        let _ = stream.flush();
        let _ = stream.shutdown(Shutdown::Both);
    }

    fn handle_buffer_line_added(&self, msg_type: MessageType) {
        let hdata = match msg_type {
            MessageType::HData(h)   => h,
            MessageType::StrData(_) => panic!("recvd strdata, expecting hdata"),
        };

        // Check if this line has a highlight or a private message that we
        // should notify on
        let mut play_sound = false;
        for data in hdata.data {
            let highlight = match data["highlight"] {
                DataType::Chr(c) => c,
                _                => panic!("Highlight should be a chr"),
            };
            if highlight == (1 as char) {
                play_sound = true;
                break;
            }

            let tags_array = match data["tags_array"] {
                DataType::Arr(ref array) => array,
                _                        => panic!("tags_array should be type array"),
            };
            for element in tags_array {
                let tag_str = match element {
                    &DataType::Str(Some(ref s)) => s.as_ref(),
                    &DataType::Str(None)        => "",
                    _                 => panic!("array should be type str"),
                };
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

    fn run_loop(&self, stream: &TcpStream) -> Result<(), WeechatError> {
        try!(self.init_relay(stream));

        // We only need to sync buffers to get highlights. We don't need
        // nicklist or anything like that
        let cmd_str = String::from("sync * buffer");
        try!(self.send_cmd(stream, cmd_str));

        loop {
            let msg = try!(self.recv_msg(stream));
            match msg.identifier.as_ref() {
                "_buffer_line_added" => self.handle_buffer_line_added(msg.data),
                _                    => (),
            };
        }
    }

    pub fn run(&self) -> Result<(), WeechatError> {
        let stream = &try!(self.connect_relay());
        let result = self.run_loop(stream);
        self.close_relay(stream);
        result
    }
}
