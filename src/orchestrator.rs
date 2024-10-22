use crate::*;

pub fn handler(receiver: Receiver) {
	let mut next_id = 0;
	let mut clients = collections::HashMap::<u32, Sender>::new();

	while let Ok(msg) = receiver.recv() {
		match msg {
			InternalMessage::Register(sender) => {
				sender.send(InternalMessage::Registered { id: next_id }).unwrap();
				clients.insert(next_id, sender);
				next_id += 1;
			}
			InternalMessage::SendMessage { author, channel, content } => {
				// TODO: only send to clients on the same channel
				for (_, client) in &clients {
					client
						.send(InternalMessage::SendMessage {
							author,
							channel: channel.clone(),
							content: content.clone(),
						})
						.unwrap();
				}
			}
			InternalMessage::Unregister { id } => {
				clients.remove(&id);
			}
			_ => {}
		}
	}
}
