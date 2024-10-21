use std::net;
use std::thread;

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
			let msg = websocket.read().unwrap();
			if msg.is_binary() || msg.is_text() {
				websocket.send(msg).unwrap();
			}
		});
	}
}
