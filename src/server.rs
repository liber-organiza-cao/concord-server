use std::collections;
use std::io;
use std::net;

use crate::*;

type WebSocket = tungstenite::WebSocket<net::TcpStream>;

pub struct Server<T: nanoserde::DeJson + nanoserde::SerJson, const TIMEOUT_IN_MILLIS: u64> {
	listener: net::TcpListener,
	clients: collections::HashMap<u64, WebSocket>,
	message_queue: Vec<(u64, T)>,
	connected_queue: Vec<u64>,
	disconnected_queue: Vec<u64>,
	id_counter: u64,
}

impl<T: nanoserde::DeJson + nanoserde::SerJson, const TIMEOUT_IN_MILLIS: u64> Server<T, TIMEOUT_IN_MILLIS> {
	pub fn bind<A: net::ToSocketAddrs>(addr: A) -> error::Result<Self> {
		let listener = net::TcpListener::bind(addr)?;
		listener.set_nonblocking(true)?;
		let clients = collections::HashMap::new();
		let connected_queue = Vec::new();
		let disconnected_queue = Vec::new();
		let message_queue = Vec::new();
		let id_counter = 0;
		Ok(Self {
			listener,
			clients,
			connected_queue,
			disconnected_queue,
			id_counter,
			message_queue,
		})
	}
	pub fn pull(&mut self) -> bool {
		if let Ok((stream, _)) = self.listener.accept() {
			if stream.set_nonblocking(true).is_ok() {
				match tungstenite::accept(stream) {
					Ok(socket) => {
						let id = self.id_counter;
						self.id_counter += 1;
						self.connected_queue.push(id);
						self.clients.insert(id, socket);
					}
					Err(tungstenite::HandshakeError::Interrupted(mut m)) => {
						let websocket = loop {
							match m.handshake() {
								Ok(t) => break Ok(t),
								Err(tungstenite::HandshakeError::Interrupted(md)) => m = md,
								Err(e) => break Err(e),
							}
						};
						if let Ok(socket) = websocket {
							let id = self.id_counter;
							self.id_counter += 1;
							self.connected_queue.push(id);
							self.clients.insert(id, socket);
						}
					}
					Err(e) => log::info!("{e}"),
				}
			}
		}

		for (id, socket) in &mut self.clients {
			match socket.read() {
				Ok(tungstenite::Message::Text(msg)) => match T::deserialize_json(&msg) {
					Ok(t) => self.message_queue.push((*id, t)),
					Err(e) => log::error!("{e}"),
				},
				Err(tungstenite::Error::Io(io)) => {
					let kind = io.kind();
					if !matches!(kind, io::ErrorKind::WouldBlock) {
						log::error!("{kind}");
					}
				}
				Err(tungstenite::Error::AlreadyClosed | tungstenite::Error::ConnectionClosed) => {
					self.disconnected_queue.push(*id);
				}
				Err(e) => log::error!("{e}"),
				_ => {}
			}
		}
		for id in &self.disconnected_queue {
			self.clients.remove(id);
		}
		true
	}
	pub fn on_connected(&mut self) -> Option<u64> {
		self.connected_queue.pop()
	}
	pub fn on_disconnected(&mut self) -> Option<u64> {
		self.disconnected_queue.pop()
	}
	pub fn on_message(&mut self) -> Option<(u64, T)> {
		self.message_queue.pop()
	}
	pub fn send(&mut self, id: u64, msg: T) -> error::Result<()> {
		if let Some(socket) = self.clients.get_mut(&id) {
			let message = tungstenite::Message::Text(msg.serialize_json());
			socket.send(message)?;
		}
		return Err(error::Error::ClientNotFound);
	}
	pub fn get_clients(&mut self) -> Vec<u64> {
		self.clients.iter().map(|(id, _)| *id).collect()
	}
}
