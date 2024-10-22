use crate::*;

pub fn handler(mut websocket: Websocket, sender: Sender) {
	log::info!("cliente connectado");
	let receiver = {
		let (s, receiver) = sync::mpsc::channel::<InternalMessage>();
		sender.send(InternalMessage::Register(s)).unwrap();
		receiver
	};

	// connection id.
	// TODO: this doesn't need to be an Option
	let mut conn_id = None;

	loop {
		let msg = match websocket.read() {
			Ok(tungstenite::Message::Text(msg)) => Some(msg),
			Err(tungstenite::Error::AlreadyClosed | tungstenite::Error::ConnectionClosed) => break,
			Err(tungstenite::Error::Io(err)) => {
				let err = err.kind();
				if !matches!(err, ErrorKind::WouldBlock) {
					log::error!("{err}");
				}
				None
			}
			_ => None,
		};
		if let Some(msg) = msg {
			match NetworkMessage::deserialize_json(&msg) {
				Ok(NetworkMessage::SendMessage { channel, content }) => {
					// log::info!("id: {id} ,channel: {channel} ,content: {content}");
					if let Some(author) = conn_id {
						sender.send(InternalMessage::SendMessage { author, channel, content }).unwrap();
					}
				}
				Err(e) => {
					log::info!("{e}");
					continue;
				}
				_ => {}
			};
		}
		match receiver.recv_timeout(TIMEOUT) {
			Ok(InternalMessage::SendMessage {
				author, channel, content, ..
			}) => {
				let msg = NetworkMessage::ReceiveMessage { author, channel, content }.serialize_json();
				let _ = websocket.send(tungstenite::Message::text(msg));
			}
			Ok(InternalMessage::Registered { id }) => {
				conn_id = Some(id);
				let msg = NetworkMessage::Registered { id }.serialize_json();
				let _ = websocket.send(tungstenite::Message::text(msg));
			}
			_ => {}
		}
	}
	if let Some(id) = conn_id {
		sender.send(InternalMessage::Unregister { id }).unwrap();
	}
}
