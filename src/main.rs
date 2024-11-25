use nanoserde::{DeJson, SerJson};

mod error;
mod server;

#[derive(Debug, Clone, PartialEq, Eq, DeJson, SerJson)]
pub enum Message {
	SendMessage { channel: String, content: String },
	ReceiveMessage { author: u64, channel: String, content: String },
	Connected { id: u64 },
	Disconnected { id: u64 },
	ChangeStatus { author: u64, afk: bool },
}

fn main() {
	#[cfg(not(debug_assertions))]
	simple_logger::init_with_level(log::Level::Info).unwrap();
	#[cfg(debug_assertions)]
	simple_logger::init_with_level(log::Level::Debug).unwrap();

	let mut server = server::Server::<Message, 5>::bind("0.0.0.0:6464").unwrap();
	while server.pull() {
		if let Some(id) = server.on_connected() {
			let _ = server.send(id, Message::Connected { id });
			log::info!("{id} connected");
		}
		if let Some(disconnected_id) = server.on_disconnected() {
			log::info!("{disconnected_id} disconnected");

			for id in server.get_clients() {
				let _ = server.send(id, Message::Disconnected { id: disconnected_id });
			}
		}
		if let Some((id, msg)) = server.on_message() {
			match msg {
				Message::SendMessage { channel, content } => {
					let author = id;
					for id in server.get_clients() {
						let channel = channel.clone();
						let content = content.clone();
						let _ = server.send(id, Message::ReceiveMessage { author, channel, content });
					}
				}
				Message::ChangeStatus { author, afk } => {
					for id in server.get_clients() {
						let _ = server.send(id, Message::ChangeStatus { author, afk });
					}
				}
				_ => {}
			}
		}
	}
}
