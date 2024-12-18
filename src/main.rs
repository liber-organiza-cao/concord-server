use std::collections;

mod error;
mod server;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
enum Message {
	SetPubkey { pubkey: String },
	PubkeySet { id: u64, pubkey: String },
	SendMessage { channel: String, content: String },
	MessageSent { author: String, channel: String, content: String },
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
	#[serde(rename = "sdpMLineIndex")]
	sdp_mline_index: Option<i64>,
	#[serde(rename = "sdpMid")]
	sdp_mid: Option<String>,
	#[serde(rename = "usernameFragment")]
	username_fragment: Option<String>,
}

#[tokio::main]
async fn main() {
	#[cfg(not(debug_assertions))]
	simple_logger::init_with_level(log::Level::Info).unwrap();
	#[cfg(debug_assertions)]
	simple_logger::init_with_level(log::Level::Debug).unwrap();

	let mut server = server::Server::<Message, 5>::bind("0.0.0.0:6464").unwrap();
	let mut voice_channel_peers = collections::HashMap::<String, Vec<u64>>::new();
	let mut conn_id_to_pubkey = collections::HashMap::<u64, String>::new();

	while server.pull() {
		if let Some(id) = server.on_connected() {
			let _ = server.send(id, Message::Connected { id });
			for (channel, peers) in &voice_channel_peers {
				for peer_id in peers {
					let channel = channel.clone();
					let _ = server.send(id, Message::JoinedVoiceChannel { channel, id: *peer_id });
				}
			}

			for (conn_id, pubkey) in &conn_id_to_pubkey {
				let _ = server.send(
					id,
					Message::PubkeySet {
						id: *conn_id,
						pubkey: pubkey.clone(),
					},
				);
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
					let Some(author) = conn_id_to_pubkey.get(&id) else {
						continue;
					};

					for id in server.get_clients() {
						let _ = server.send(
							id,
							Message::MessageSent {
								author: author.clone(),
								channel: channel.clone(),
								content: content.clone(),
							},
						);
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
					} else {
						let peers = vec![id];
						let channel = channel.clone();
						voice_channel_peers.insert(channel, peers);
					}
					for client_id in server.get_clients() {
						let channel = channel.clone();
						let _ = server.send(client_id, Message::JoinedVoiceChannel { channel, id });
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
				Message::SetPubkey { pubkey } => {
					conn_id_to_pubkey.insert(id, pubkey.clone());

					for client in server.get_clients() {
						let _ = server.send(client, Message::PubkeySet { id, pubkey: pubkey.clone() });
					}
				}
				_ => {}
			}
		}
	}
}
