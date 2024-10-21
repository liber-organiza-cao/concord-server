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
			InternalMessage::SendMessage {
				sender_id,
				receiver_id,
				channel,
				content,
			} => {
				let Some(sender) = clients.get(&receiver_id) else {
					log::info!("Client: {receiver_id} not exists");
					continue;
				};
				sender
					.send(InternalMessage::SendMessage {
						sender_id,
						receiver_id,
						channel,
						content,
					})
					.unwrap();
			}
			InternalMessage::Unregister { id } => {
				clients.remove(&id);
			}
			_ => {}
		}
	}
}
