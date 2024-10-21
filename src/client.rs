use crate::*;

pub fn handler(mut websocket: Websocket, sender: Sender) {
	log::info!("cliente connectado");
	let receiver = {
		let (s, receiver) = sync::mpsc::channel::<InternalMessage>();
		sender.send(InternalMessage::Register(s)).unwrap();
		receiver
	};

	let mut my_id = None;

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
				Ok(NetworkMessage::SendMessage { id, channel, content }) => {
					log::info!("id: {id} ,channel: {channel} ,content: {content}");
					sender.send(InternalMessage::SendMessage { id, channel, content }).unwrap();
				}
				Err(e) => {
					log::info!("{e}");
					continue;
				}
				_ => {}
			};
		}
		match receiver.recv_timeout(TIMEOUT) {
			Ok(InternalMessage::SendMessage { id, channel, content }) => {
				let msg = NetworkMessage::ReceiveMessage { id, channel, content }.serialize_json();
				let _ = websocket.send(tungstenite::Message::text(msg));
			}
			Ok(InternalMessage::Registered { id }) => {
				my_id = Some(id);
				let msg = NetworkMessage::Registered { id }.serialize_json();
				let _ = websocket.send(tungstenite::Message::text(msg));
			}
			_ => {}
		}
	}
	if let Some(id) = my_id {
		sender.send(InternalMessage::Unregister { id }).unwrap();
	}
}
