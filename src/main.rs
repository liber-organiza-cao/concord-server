use nanoserde::{DeJson, SerJson};
use std::net;
use std::thread;

#[derive(DeJson, SerJson, Clone, PartialEq, Eq)]
enum Message {
	SendMessage { channel: String, text: String },
}

fn main() {
	let server = net::TcpListener::bind("0.0.0.0:6464").unwrap();
	for stream in server.incoming() {
		let Ok(stream) = stream else {
			continue;
		};
		let Ok(mut websocket) = tungstenite::accept(stream) else {
			continue;
		};
		thread::spawn(move || loop {
			let msg = match websocket.read() {
				Ok(tungstenite::Message::Text(msg)) => msg,
				Err(tungstenite::Error::AlreadyClosed | tungstenite::Error::ConnectionClosed) => break,
				_ => continue,
			};

			match Message::deserialize_json(&msg) {
				Ok(Message::SendMessage { channel, text }) => {
					println!("channel: {channel}, text: {text}");
				}
				Err(e) => {
					log::info!("{e}");
					continue;
				}
			};

			let _ = websocket.send(tungstenite::Message::Text(msg));
		});
	}
}
