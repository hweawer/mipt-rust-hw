use crate::data::{PeerMessage, VerifiedPeerMessage};

use anyhow::{bail, Context, Result};
use crossbeam::channel::{self, Receiver, Sender};
use log::*;
use serde::{Deserialize, Serialize};

use byteorder::ReadBytesExt;
use rand::Rng;
use std::io::{BufRead, BufWriter, Lines};
use std::net::Shutdown;
use std::{
    collections::HashMap,
    fmt::{self, Display},
    io::{self, BufReader, ErrorKind, Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

////////////////////////////////////////////////////////////////////////////////

const BUF_SIZE: usize = 65536;
const RECONNECT_LIMIT: usize = 3;

pub type SessionId = u64;

////////////////////////////////////////////////////////////////////////////////

#[derive(Default, Serialize, Deserialize)]
pub struct PeerServiceConfig {
    #[serde(with = "humantime_serde")]
    pub dial_cooldown: Duration,
    pub dial_addresses: Vec<String>,
    pub listen_address: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PeerEvent {
    pub session_id: SessionId,
    pub event_kind: PeerEventKind,
}

#[derive(Debug, Clone)]
pub enum PeerEventKind {
    Connected,
    Disconnected,
    NewMessage(VerifiedPeerMessage),
}

#[derive(Debug, Clone)]
pub struct PeerCommand {
    pub session_id: SessionId,
    pub command_kind: PeerCommandKind,
}

#[derive(Debug, Clone)]
pub enum PeerCommandKind {
    SendMessage(VerifiedPeerMessage),
    Drop,
}

////////////////////////////////////////////////////////////////////////////////

pub struct PeerService {
    config: PeerServiceConfig,
    peer_event_sender: Sender<PeerEvent>,
    command_receiver: Receiver<PeerCommand>,
    peers: Arc<Mutex<HashMap<SessionId, Arc<TcpStream>>>>,
}

impl PeerService {
    pub fn new(
        config: PeerServiceConfig,
        peer_event_sender: Sender<PeerEvent>,
        command_receiver: Receiver<PeerCommand>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            peer_event_sender,
            command_receiver,
            peers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn run(&mut self) {
        let nodes = &self.config.dial_addresses;
        let mut current_attempt = 0;
        for node in nodes {
            loop {
                let connection = TcpStream::connect(node);
                match connection {
                    Ok(conn) => {
                        let session_id = rand::thread_rng().gen::<SessionId>();
                        let connection = Arc::new(conn);
                        let mut peers = self.peers.lock().unwrap();
                        peers.insert(session_id, Arc::clone(&connection));

                        self.handle_reader(Arc::clone(&connection), session_id);
                        self.handle_writer();
                        current_attempt = 0;
                        break;
                    }
                    Err(_) => {
                        if current_attempt == RECONNECT_LIMIT {
                            current_attempt = 0;
                            break;
                        } else {
                            thread::sleep(self.config.dial_cooldown);
                            current_attempt += 1;
                        }
                    }
                }
            }
        }

        let address = self.config.listen_address.take().unwrap();
        let listener = TcpListener::bind(address.clone()).unwrap();
        self.config.listen_address = Some(address);
        for stream in listener.incoming() {
            debug!("New client has come");
            let stream = stream.expect("Failed to retrieve TcpStream from connection!");
            let session_id = rand::thread_rng().gen::<SessionId>();
            let connection = Arc::new(stream);
            let mut peers = self.peers.lock().unwrap();
            peers.insert(session_id, Arc::clone(&connection));

            self.handle_reader(Arc::clone(&connection), session_id);
            self.handle_writer();
        }
    }

    fn handle_reader(&self, connection: Arc<TcpStream>, session_id: SessionId) {
        let sender = self.peer_event_sender.clone();
        thread::spawn(move || {
            let event = PeerEvent {
                session_id,
                event_kind: PeerEventKind::Connected,
            };
            sender
                .send(event)
                .expect("Connection event was failed to send");
            let stream = connection.as_ref();
            let mut socket = BufReader::with_capacity(BUF_SIZE, stream);
            loop {
                let mut line = vec![];
                // todo: handling failure with reconnect
                let mut size = 0;
                let mut byte = socket.read_u8().unwrap();
                while byte != b'\0' && line.len() < BUF_SIZE {
                    line.push(byte);
                    byte = socket.read_u8().unwrap();
                    size += 1;
                }
                println!("Incoming size: {}", size);
                if size == 0 {
                    continue;
                }
                println!("Line: {:?}", line.to_vec());
                if line.len() == BUF_SIZE {
                    //todo: beautify
                    error!("Too large message");
                    let event = PeerEvent {
                        session_id,
                        event_kind: PeerEventKind::Disconnected,
                    };
                    sender
                        .send(event)
                        .expect("Disconnected Event can't be sent!");
                    break;
                }
                if line[size - 1] == b'\0' {
                    line.pop();
                }
                let str_json =
                    String::from_utf8(line).expect("Failed to convert bytes to valid utf_8");
                println!("New json has come: {}", str_json);
                let verified_peer_message =
                    match serde_json::from_str::<PeerMessage>(str_json.as_str()) {
                        Ok(mes) => mes.verified().expect("Failed to verify PeerMessage"),
                        Err(_) => {
                            error!("Failed to decode the message: {:?}", str_json);
                            let event = PeerEvent {
                                session_id,
                                event_kind: PeerEventKind::Disconnected,
                            };
                            sender
                                .send(event)
                                .expect("Disconnected Event can't be sent!");
                            break;
                        }
                    };
                let event = PeerEvent {
                    session_id,
                    event_kind: PeerEventKind::NewMessage(verified_peer_message),
                };
                debug!("Event to be sent: {:?}", event);
                let _ = sender
                    .send(event)
                    .expect("Failed to send PeerEvent to the channel");
            }
        });
    }

    fn handle_writer(&self) {
        let receiver = self.command_receiver.clone();
        let peers = Arc::clone(&self.peers);
        thread::spawn(move || loop {
            let PeerCommand {
                session_id,
                command_kind,
            } = receiver.recv().expect("Error while receiving command");
            let mut peers = peers.lock().expect("Failed to take lock on peers map");
            // todo: optimize buffer creation?
            let mut writer = BufWriter::with_capacity(
                BUF_SIZE,
                peers
                    .get(&session_id)
                    .expect("Failed to find writer which session should be present!")
                    .as_ref(),
            );
            match command_kind {
                PeerCommandKind::SendMessage(mes) => {
                    let peer_mes: PeerMessage = mes.into();
                    debug!("Peer serializer: {:?}", peer_mes);
                    serde_json::to_writer(&mut writer, &peer_mes)
                        .expect("Failed to serialize object ot json");
                    writer.write(&[b'\0']).unwrap();
                    writer.flush().expect("Failed to flush the buffer!");
                }
                PeerCommandKind::Drop => {
                    debug!("Finish session command has received {:?}", session_id);
                    //writer.get_mut().shutdown(Shutdown::Both).unwrap();
                    writer.write(b"Error has happened".as_slice()).unwrap();
                    writer.flush().unwrap();
                    writer.get_mut().shutdown(Shutdown::Both).unwrap();
                    break;
                }
            }
        });
    }
}
