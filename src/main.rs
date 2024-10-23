use nanoserde::{DeJson, SerJson};

mod server;

#[derive(Debug, Clone, PartialEq, Eq, DeJson, SerJson)]
pub enum Message {
	SendMessage { channel: String, content: String },
	ReceiveMessage { author: u64, channel: String, content: String },
}

fn main() {
	#[cfg(not(debug_assertions))]
	simple_logger::init_with_level(log::Level::Info).unwrap();
	#[cfg(debug_assertions)]
	simple_logger::init_with_level(log::Level::Debug).unwrap();

	let server = server::Server::<Message, 5>::bind("0.0.0.0:6464").unwrap();
	server.run(&|handler| {
		if let Some((id, msg)) = handler.read() {
			match msg {
				Message::SendMessage { channel, content } => {
					let author = id;
					for id in handler.get_clients() {
						let channel = channel.clone();
						let content = content.clone();
						handler.send(id, Message::ReceiveMessage { author, channel, content }).unwrap();
					}
				}
				_ => {}
			}
		}
	});
}
