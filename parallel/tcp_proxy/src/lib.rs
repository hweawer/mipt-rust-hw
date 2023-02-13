#![forbid(unsafe_code)]

use std::io::{BufReader, BufWriter};
use std::net::{Shutdown, TcpListener, TcpStream};

use log::info;
use std::io::prelude::*;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

//todo: it is passing tests, but is not working with Firefox for some reason,
// also when should shutdown be called?
pub fn run_proxy(port: u32, destination: String) {
    let client_host_port = format!("{}", destination);
    let server_host_port = format!("127.0.0.1:{}", port);
    info!("Server destination : {}", server_host_port);
    let server = TcpListener::bind(server_host_port).expect("Can't start proxy.");
    for stream in server.incoming() {
        info!("Connection came");
        let stream = stream.expect("Connection failed");
        let client_stream = Arc::new(stream);
        let proxy_stream = Arc::new(
            TcpStream::connect(&client_host_port).expect("Can't connect to the destination."),
        );
        let _ = handle_duplex_channel(Arc::clone(&proxy_stream), Arc::clone(&client_stream));
        let _ = handle_duplex_channel(Arc::clone(&client_stream), Arc::clone(&proxy_stream));
    }
}

fn handle_duplex_channel(from_stream: Arc<TcpStream>, to_stream: Arc<TcpStream>) -> JoinHandle<()> {
    thread::spawn(move || {
        let mut reader = BufReader::new(from_stream.as_ref());
        let mut writer = BufWriter::new(to_stream.as_ref());
        loop {
            let mut data = reader.fill_buf().unwrap();
            while !data.is_empty() {
                let bytes_to_consume = data.len();
                writer.write(&data).unwrap();
                reader.consume(bytes_to_consume);
                writer.flush().unwrap();
                data = reader.fill_buf().unwrap();
            }
        }
    })
}
