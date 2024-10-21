use nanoserde::{DeJson, SerJson};
use std::collections;
use std::net;
use std::sync;
use std::thread;

#[derive(DeJson, SerJson, Clone, PartialEq, Eq)]
enum NetworkMessage {
	SendMessage { id: u32, channel: String, content: String },
	ReceiveMessage { id: u32, channel: String, content: String },
	Registered { id: u32 },
}

#[derive(Debug, Clone)]
enum InternalMessage {
	Register(Sender),
	Registered { id: u32 },
	SendMessage { id: u32, channel: String, content: String },
}

type Websocket = tungstenite::WebSocket<net::TcpStream>;
type Receiver = sync::mpsc::Receiver<InternalMessage>;
type Sender = sync::mpsc::Sender<InternalMessage>;

fn main() {
	let server = net::TcpListener::bind("0.0.0.0:6464").unwrap();
	let (sender, receiver) = sync::mpsc::channel::<InternalMessage>();
	thread::spawn(move || orchestrator(receiver));
	for stream in server.incoming() {
		let Ok(stream) = stream else {
			continue;
		};
		let Ok(websocket) = tungstenite::accept(stream) else {
			continue;
		};
		let sender = sender.clone();
		thread::spawn(move || client_handler(websocket, sender));
	}
}

fn orchestrator(receiver: Receiver) {
	let mut next_id = 0;
	let mut clients = collections::HashMap::<u32, Sender>::new();

	while let Ok(msg) = receiver.recv() {
		match msg {
			InternalMessage::Register(sender) => {
				sender.send(InternalMessage::Registered { id: next_id }).unwrap();
				clients.insert(next_id, sender);
				next_id += 1;
			}
			InternalMessage::SendMessage { id, channel, content } => {
				let Some(sender) = clients.get(&id) else {
					log::info!("Client: {id} not exists");
					continue;
				};
				sender.send(InternalMessage::SendMessage { id, channel, content }).unwrap();
			}
			_ => {}
		}
	}
}

fn client_handler(mut websocket: Websocket, sender: Sender) {
	let receiver = {
		let (s, receiver) = sync::mpsc::channel::<InternalMessage>();
		let _ = sender.send(InternalMessage::Register(s));
		receiver
	};

	loop {
		let msg = match websocket.read() {
			Ok(tungstenite::Message::Text(msg)) => msg,
			Err(tungstenite::Error::AlreadyClosed | tungstenite::Error::ConnectionClosed) => break,
			_ => continue,
		};

		match NetworkMessage::deserialize_json(&msg) {
			Ok(NetworkMessage::SendMessage { id, channel, content }) => {
				println!("id: {id} ,channel: {channel} ,content: {content}");
				sender.send(InternalMessage::SendMessage { id, channel, content }).unwrap()
			}
			Err(e) => {
				log::info!("{e}");
				continue;
			}
			_ => {}
		};
		match receiver.recv() {
			Ok(InternalMessage::SendMessage { id, channel, content }) => {
				let msg = NetworkMessage::SendMessage { id, channel, content }.serialize_json();
				let _ = websocket.send(tungstenite::Message::text(msg));
			}
			Ok(InternalMessage::Registered { id }) => {
				let msg = NetworkMessage::Registered { id }.serialize_json();
				let _ = websocket.send(tungstenite::Message::text(msg));
			}
			_ => {}
		}
	}
}
