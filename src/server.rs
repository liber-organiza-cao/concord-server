use std::collections;
use std::io;
use std::marker;
use std::net;
use std::sync;
use std::thread;
use std::time;

enum InternalMessage<T: nanoserde::DeJson + nanoserde::SerJson> {
	SendMessage { receiver_id: u64, message: T },
}

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	Io,
	SendError,
}

pub struct Server<T: nanoserde::DeJson + nanoserde::SerJson, const TIMEOUT_IN_MILLIS: u64> {
	listener: net::TcpListener,
	message: marker::PhantomData<T>,
}

pub struct ConnectionHandler<T: nanoserde::DeJson + nanoserde::SerJson> {
	sender: crossbeam_channel::Sender<InternalMessage<T>>,
	clients: sync::Arc<sync::RwLock<collections::HashSet<u64>>>,
	messages_queue: Vec<(u64, T)>,
}

impl<T: nanoserde::DeJson + nanoserde::SerJson> ConnectionHandler<T> {
	pub fn send(&mut self, id: u64, message: T) -> Result<()> {
		Ok(self.sender.send(InternalMessage::SendMessage { receiver_id: id, message })?)
	}
	pub fn read(&mut self) -> Option<(u64, T)> {
		self.messages_queue.pop()
	}
	pub fn get_clients(&self) -> Vec<u64> {
		self.clients.read().unwrap().iter().map(|id| *id).collect()
	}
}

impl<T: nanoserde::DeJson + nanoserde::SerJson + std::marker::Send + 'static, const TIMEOUT_IN_MILLIS: u64> Server<T, TIMEOUT_IN_MILLIS> {
	pub fn bind<A: net::ToSocketAddrs>(addr: A) -> Result<Self> {
		let listener = net::TcpListener::bind(addr)?;
		Ok(Self {
			listener,
			message: marker::PhantomData,
		})
	}
	pub fn run<F: Fn(&mut ConnectionHandler<T>) + std::marker::Sync>(self, handshake: &'static F) {
		let timeout = time::Duration::from_millis(TIMEOUT_IN_MILLIS);
		let (sender, receiver) = crossbeam_channel::bounded::<InternalMessage<T>>(0);
		let clients = sync::Arc::new(sync::RwLock::new(collections::HashSet::<u64>::new()));
		let mut id_counter = 0;

		for stream in self.listener.incoming() {
			let Ok(stream) = stream else {
				continue;
			};
			stream.set_read_timeout(Some(timeout)).unwrap();
			let Ok(mut websocket) = tungstenite::accept(stream) else {
				continue;
			};

			{
				let mut clients = clients.write().unwrap();
				clients.insert(id_counter);
			}

			let sender = sender.clone();
			let receiver = receiver.clone();
			let clients = clients.clone();
			let id = id_counter;

			let mut handler = ConnectionHandler {
				sender,
				clients,
				messages_queue: Vec::new(),
			};
			id_counter += 1;

			thread::spawn(move || {
				loop {
					match websocket.read() {
						Ok(tungstenite::Message::Text(str)) => {
							if let Ok(msg) = T::deserialize_json(&str) {
								handler.messages_queue.push((id, msg));
							}
						}
						Err(tungstenite::Error::AlreadyClosed | tungstenite::Error::ConnectionClosed) => break,
						Err(tungstenite::Error::Io(err)) => {
							let err = err.kind();
							if !matches!(err, io::ErrorKind::WouldBlock) {
								log::error!("{err}");
							}
						}
						_ => {}
					};

					match receiver.recv_timeout(timeout) {
						Ok(InternalMessage::SendMessage { receiver_id, message }) => {
							if receiver_id == id {
								let message = message.serialize_json();
								if let Err(e) = websocket.send(tungstenite::Message::text(message)) {
									log::error!("{e}");
								}
							}
						}
						Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
						Err(e) => log::error!("{e}"),
					}

					handshake(&mut handler);
				}
				{
					let mut clients = handler.clients.write().unwrap();
					clients.remove(&id);
				}
			});
		}
	}
}

impl From<io::Error> for Error {
	#[inline(always)]
	fn from(_: io::Error) -> Self {
		Self::Io
	}
}

impl<T> From<crossbeam_channel::SendError<T>> for Error {
	fn from(_: crossbeam_channel::SendError<T>) -> Self {
		Self::SendError
	}
}
