use nanoserde::{DeJson, SerJson};
use std::collections;
use std::io::ErrorKind;
use std::net;
use std::sync;
use std::thread;
use std::time;

mod client;
mod orchestrator;

#[derive(DeJson, SerJson, Clone, PartialEq, Eq)]
pub enum NetworkMessage {
	SendMessage { id: u32, channel: String, content: String },
	ReceiveMessage { id: u32, channel: String, content: String },
	Registered { id: u32 },
}

#[derive(Debug, Clone)]
pub enum InternalMessage {
	Register(Sender),
	Registered { id: u32 },
	SendMessage { id: u32, channel: String, content: String },
}

pub type Websocket = tungstenite::WebSocket<net::TcpStream>;
pub type Receiver = sync::mpsc::Receiver<InternalMessage>;
pub type Sender = sync::mpsc::Sender<InternalMessage>;

fn main() {
	#[cfg(not(debug_assertions))]
	simple_logger::init_with_level(log::Level::Info).unwrap();
	#[cfg(debug_assertions)]
	simple_logger::init_with_level(log::Level::Debug).unwrap();
	let server = net::TcpListener::bind("0.0.0.0:6464").unwrap();
	let (sender, receiver) = sync::mpsc::channel::<InternalMessage>();
	thread::spawn(move || orchestrator::handler(receiver));
	for stream in server.incoming() {
		let Ok(stream) = stream else {
			continue;
		};
		stream.set_read_timeout(Some(time::Duration::from_nanos(100))).unwrap();
		let Ok(websocket) = tungstenite::accept(stream) else {
			continue;
		};
		let sender = sender.clone();
		thread::spawn(move || client::handler(websocket, sender));
	}
}
