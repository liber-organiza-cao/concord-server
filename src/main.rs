use std::collections;

mod error;
mod server;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Message {
	SendMessage { channel: String, content: String },
	ReceiveMessage { author: u64, channel: String, content: String },
	Connected { id: u64 },
	Disconnected { id: u64 },
	ChangeStatus { author: u64, afk: bool },
	Offer { id: u64, data: Offer },
	Answer { id: u64, data: Answer },
	Candidate { id: u64, data: Candidate },
	JoinVoiceChannel { channel: String },
	JoinedVoiceChannel { channel: String, id: u64 },
	LeaveVoiceChannel { channel: String },
	LeftVoiceChannel { channel: String, id: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct Answer {
	r#type: String,
	sdp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct Offer {
	r#type: String,
	sdp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct Candidate {
	candidate: Option<String>,
	sdpMLineIndex: Option<i64>,
	sdpMid: Option<String>,
	usernameFragment: Option<String>,
}

fn main() {
	#[cfg(not(debug_assertions))]
	simple_logger::init_with_level(log::Level::Info).unwrap();
	#[cfg(debug_assertions)]
	simple_logger::init_with_level(log::Level::Debug).unwrap();

	let mut server = server::Server::<Message, 5>::bind("0.0.0.0:6464").unwrap();
	let mut voice_channel_peers = collections::HashMap::<String, Vec<u64>>::new();

	while server.pull() {
		if let Some(id) = server.on_connected() {
			let _ = server.send(id, Message::Connected { id });
			for (channel, peers) in &voice_channel_peers {
				for peer_id in peers {
					let channel = channel.clone();
					let _ = server.send(id, Message::JoinedVoiceChannel { channel, id: *peer_id });
				}
			}
			log::info!("{id} connected");
		}
		if let Some(disconnected_id) = server.on_disconnected() {
			log::info!("{disconnected_id} disconnected");

			for (channel, peers) in &mut voice_channel_peers {
				peers.retain(|p| {
					let state = *p != disconnected_id;
					if !state {
						for client_id in server.get_clients() {
							let channel = channel.clone();
							let _ = server.send(
								client_id,
								Message::LeftVoiceChannel {
									channel,
									id: disconnected_id,
								},
							);
						}
					}
					state
				});
			}

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
				Message::Offer { id: receiver_id, data } => {
					let _ = server.send(receiver_id, Message::Offer { id, data });
				}
				Message::Candidate { id: receiver_id, data } => {
					let _ = server.send(receiver_id, Message::Candidate { id, data });
				}
				Message::Answer { id: receiver_id, data } => {
					let _ = server.send(receiver_id, Message::Answer { id, data });
				}
				Message::JoinVoiceChannel { channel } => {
					if let Some(peers) = voice_channel_peers.get_mut(&channel) {
						if peers.contains(&id) {
							continue;
						}
						peers.push(id);
						for client_id in server.get_clients() {
							let channel = channel.clone();
							let _ = server.send(client_id, Message::JoinedVoiceChannel { channel, id });
						}
					} else {
						let peers = vec![id];
						voice_channel_peers.insert(channel, peers);
					}
				}
				Message::LeaveVoiceChannel { channel } => {
					if let Some(peers) = voice_channel_peers.get_mut(&channel) {
						peers.retain(|peer_id| *peer_id != id);
					}
					for peer_id in server.get_clients() {
						let channel = channel.clone();
						let _ = server.send(peer_id, Message::LeftVoiceChannel { channel, id });
					}
				}
				_ => {}
			}
		}
	}
}
