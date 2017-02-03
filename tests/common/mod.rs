// Copyright 2016 James Bendig. See the COPYRIGHT file at the top-level
// directory of this distribution.
//
// Licensed under:
//   the MIT license
//     <LICENSE-MIT or https://opensource.org/licenses/MIT>
//   or the Apache License, Version 2.0
//     <LICENSE-APACHE or https://www.apache.org/licenses/LICENSE-2.0>,
// at your option. This file may not be copied, modified, or distributed
// except according to those terms.

extern crate chrono;
extern crate fix_rs;
extern crate mio;

use mio::{Events,Poll,PollOpt,Ready,Token};
use mio::tcp::{TcpListener,TcpStream};
use std::any::Any;
use std::collections::HashMap;
use std::net::{Ipv4Addr,SocketAddr,SocketAddrV4};
use std::io::{Read,Write};
use std::sync::atomic::{AtomicUsize,Ordering};
use std::thread;
use std::time::{Duration,Instant};

use fix_rs::dictionary::CloneDictionary;
use fix_rs::dictionary::field_types::other::EncryptMethod;
use fix_rs::dictionary::messages::Logon;
use fix_rs::fix::Parser;
use fix_rs::fix_version::FIXVersion;
use fix_rs::fixt::client::{Client,ClientEvent};
use fix_rs::fixt::message::{BuildFIXTMessage,FIXTMessage};
use fix_rs::message_version::MessageVersion;

const SOCKET_BASE_PORT: usize = 7000;
static SOCKET_PORT: AtomicUsize = AtomicUsize::new(SOCKET_BASE_PORT);

const CLIENT_TARGET_COMP_ID: &'static [u8] = b"TX"; //Test Exchange
const CLIENT_SENDER_COMP_ID: &'static [u8] = b"TEST";
pub const SERVER_TARGET_COMP_ID: &'static [u8] = CLIENT_SENDER_COMP_ID;
pub const SERVER_SENDER_COMP_ID: &'static [u8] = CLIENT_TARGET_COMP_ID;

const MAX_MESSAGE_SIZE: u64 = 4096;

#[macro_export]
macro_rules! client_poll_event {
    ( $client:ident,$pat:pat => $body:expr ) => {{
        let result = $client.poll(Some(Duration::from_secs(5))).expect("Client does not have any events");
        if let $pat = result {
            $body
        }
        else {
            panic!("Client has wrong event: {:?}",result)
        }
    }};
}

#[macro_export]
macro_rules! client_poll_message {
    ( $client:ident, $connection_id:ident, $message_type:ty ) => {
        client_poll_event!($client,ClientEvent::MessageReceived(msg_connection_id,response_message) => {
            assert_eq!(msg_connection_id,$connection_id);

            response_message.as_any().downcast_ref::<$message_type>().expect("Not expected message type").clone()
        });
    };
}

#[macro_export]
macro_rules! new_fixt_message {
    ( $message_type:ident ) => {{
        let mut message = $message_type::new();
        message.setup_fixt_session_header(
            Some(1),
            //Set to from-server by default because if the message is sent by the client, it will
            //overwrite these.
            $crate::common::SERVER_SENDER_COMP_ID.to_vec(),
            $crate::common::SERVER_TARGET_COMP_ID.to_vec()
        );

        message
    }};
}

pub fn new_logon_message() -> Logon {
    let mut message = new_fixt_message!(Logon);
    message.encrypt_method = EncryptMethod::None;
    message.heart_bt_int = 5;
    message.default_appl_ver_id = MessageVersion::FIX50SP2;

    message
}

pub fn accept_with_timeout(listener: &TcpListener,timeout: Duration) -> Option<TcpStream> {
    let now = Instant::now();

    while now.elapsed() <= timeout {
        if let Ok((stream,_)) = listener.accept() {
            return Some(stream);
        }

        thread::yield_now();
    }

    None
}

pub fn recv_bytes_with_timeout(stream: &mut TcpStream,timeout: Duration) -> Option<Vec<u8>> {
    let now = Instant::now();

    let mut buffer = Vec::new();
    buffer.resize(1024,0);

    while now.elapsed() <= timeout {
        if let Ok(bytes_read) = stream.read(&mut buffer[..]) {
            if bytes_read > 0 {
                buffer.resize(bytes_read,0u8);
                return Some(buffer);
            }
        }

        thread::yield_now();
    }

    None
}

pub fn send_message_with_timeout(stream: &mut TcpStream,fix_version: FIXVersion,message_version: MessageVersion,message: Box<FIXTMessage + Send>,timeout: Option<Duration>) -> Result<(),usize> {
    let mut bytes = Vec::new();
    message.read(fix_version,message_version,&mut bytes);

    let now = Instant::now();
    let mut bytes_written_total = 0;
    while bytes_written_total < bytes.len() {
        if let Some(timeout) = timeout {
            if now.elapsed() > timeout {
                return Err(bytes.len() - bytes_written_total);
            }
        }

        match stream.write(&bytes[bytes_written_total..bytes.len()]) {
            Ok(bytes_written) => bytes_written_total += bytes_written,
            Err(e) => {
                if e.kind() == ::std::io::ErrorKind::WouldBlock {
                    continue;
                }
                panic!("Could not write bytes: {}",e);
            },
        }
    }

    Ok(())
}

pub fn send_message(stream: &mut TcpStream,fix_version: FIXVersion,message_version: MessageVersion,message: Box<FIXTMessage + Send>) {
    let _ = send_message_with_timeout(stream,fix_version,message_version,message,None);
}

pub struct TestServer {
    _listener: TcpListener,
    fix_version: FIXVersion,
    message_version: MessageVersion,
    pub stream: TcpStream,
    poll: Poll,
    parser: Parser,
}

impl TestServer {
    pub fn setup_with_ver(fix_version: FIXVersion,message_version: MessageVersion,message_dictionary: HashMap<&'static [u8],Box<BuildFIXTMessage + Send>>) -> (TestServer,Client,usize) {
        //Setup server listener socket.
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127,0,0,1),SOCKET_PORT.fetch_add(1,Ordering::SeqCst) as u16));
        let listener = TcpListener::bind(&addr).unwrap();

        //Setup client and connect to socket.
        let mut client = Client::new(message_dictionary.clone(),CLIENT_SENDER_COMP_ID,CLIENT_TARGET_COMP_ID,MAX_MESSAGE_SIZE).unwrap();
        let connection_id = client.add_connection(fix_version,message_version,addr).unwrap();

        //Try to accept connection from client. Fails on timeout or socket error.
        let stream = accept_with_timeout(&listener,Duration::from_secs(5)).expect("Could not accept connection");

        //Confirm client was able to connect.
        let event = client.poll(Duration::from_secs(5)).expect("Could not connect");
        assert!(if let ClientEvent::ConnectionSucceeded(id) = event { id == connection_id } else { false });

        //Setup a single Poll to watch the TCPStream. This way we can check for disconnects in
        //is_stream_closed(). Unfortunately, as of mio 0.6.1, Linux implementation emulates OS X
        //and Windows where a stream can only be registered with one Poll for the life of the
        //socket. See: https://github.com/carllerche/mio/issues/327
        let poll = Poll::new().unwrap();
        poll.register(&stream,Token(0),Ready::all(),PollOpt::edge()).unwrap();

        (TestServer {
            _listener: listener,
            fix_version: fix_version,
            message_version: message_version,
            stream: stream,
            poll: poll,
            parser: Parser::new(message_dictionary,MAX_MESSAGE_SIZE),
        },client,connection_id)
    }

    pub fn setup(message_dictionary: HashMap<&'static [u8],Box<BuildFIXTMessage + Send>>) -> (TestServer,Client,usize) {
        Self::setup_with_ver(FIXVersion::FIXT_1_1,MessageVersion::FIX50SP2,message_dictionary)
    }

    pub fn setup_and_logon_with_ver(fix_version: FIXVersion,message_version: MessageVersion,message_dictionary: HashMap<&'static [u8],Box<BuildFIXTMessage + Send>>) -> (TestServer,Client,usize) {
        //Connect.
        let (mut test_server,mut client,connection_id) = TestServer::setup_with_ver(fix_version,message_version,message_dictionary);

        //Logon.
        let mut logon_message = new_logon_message();
        logon_message.default_appl_ver_id = message_version;
        client.send_message_box_with_message_version(connection_id,fix_version.max_message_version(),Box::new(logon_message));
        let message = test_server.recv_message::<Logon>();
        assert_eq!(message.msg_seq_num,1);

        let mut response_message = new_fixt_message!(Logon);
        response_message.encrypt_method = message.encrypt_method;
        response_message.heart_bt_int = message.heart_bt_int;
        response_message.default_appl_ver_id = message.default_appl_ver_id;
        test_server.send_message_with_ver(fix_version,fix_version.max_message_version(),response_message);
        client_poll_event!(client,ClientEvent::SessionEstablished(_) => {});
        let message = client_poll_message!(client,connection_id,Logon);
        assert_eq!(message.msg_seq_num,1);

        //After logon, just like the Client, setup the default message version that future messages
        //should adhere to.
        test_server.parser.set_default_message_version(message_version);

        (test_server,client,connection_id)
    }

    pub fn setup_and_logon(message_dictionary: HashMap<&'static [u8],Box<BuildFIXTMessage + Send>>) -> (TestServer,Client,usize) {
        Self::setup_and_logon_with_ver(FIXVersion::FIXT_1_1,MessageVersion::FIX50SP2,message_dictionary)
    }

    pub fn is_stream_closed(&self,timeout: Duration) -> bool {
        let now = Instant::now();

        while now.elapsed() <= timeout {
            let mut events = Events::with_capacity(1);
            self.poll.poll(&mut events,Some(Duration::from_millis(0))).unwrap();

            for event in events.iter() {
                if event.kind().is_hup() || event.kind().is_error() {
                    return true;
                }
            }

            thread::yield_now();
        }

        false
    }

    pub fn try_recv_fixt_message(&mut self,timeout: Duration) -> Option<Box<FIXTMessage + Send>> {
        if !self.parser.messages.is_empty() {
            return Some(self.parser.messages.remove(0));
        }

        let now = Instant::now();

        let mut buffer = Vec::new();
        buffer.resize(1024,0);

        while now.elapsed() <= timeout {
            let bytes_read = if let Ok(bytes_read) = self.stream.read(&mut buffer[..]) {
                bytes_read
            }
            else {
                thread::yield_now();
                continue;
            };

            let (bytes_parsed,result) = self.parser.parse(&buffer[0..bytes_read]);
            if result.is_err() {
                println!("try_recv_fixt_message: Parse error"); //TODO: Use Result instead of Option.
                println!("\t{}",result.err().unwrap());
                return None;
            }
            assert_eq!(bytes_parsed,bytes_read);

            if !self.parser.messages.is_empty() {
                return Some(self.parser.messages.remove(0));
            }
        }

        println!("try_recv_fixt_message: Timed out");
        None
    }

    pub fn recv_fixt_message(&mut self) -> Box<FIXTMessage + Send> {
        self.try_recv_fixt_message(Duration::from_secs(5)).expect("Did not receive FIXT message")
    }

    pub fn recv_message<T: FIXTMessage + Any + Clone>(&mut self) -> T {
        let fixt_message = self.recv_fixt_message();
        if !fixt_message.as_any().is::<T>() {
            println!("{:?}",fixt_message);
        }
        fixt_message.as_any().downcast_ref::<T>().expect("^^^ Not expected message type").clone()
    }

    pub fn send_message_with_ver<T: FIXTMessage + Any + Send>(&mut self,fix_version: FIXVersion,message_version: MessageVersion,message: T) {
        send_message(&mut self.stream,fix_version,message_version,Box::new(message));
    }

    pub fn send_message<T: FIXTMessage + Any + Send>(&mut self,message: T) {
        let fix_version = self.fix_version;
        let message_version = self.message_version;
        self.send_message_with_ver::<T>(fix_version,message_version,message);
    }

    pub fn send_message_with_timeout<T: FIXTMessage + Any + Send>(&mut self,message: T,timeout: Duration) -> Result<(),usize> {
        let fix_version = self.fix_version;
        let message_version = self.message_version;
        send_message_with_timeout(&mut self.stream,fix_version,message_version,Box::new(message),Some(timeout))
    }
}
