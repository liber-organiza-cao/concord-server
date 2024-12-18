use std::io;

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
	Io,
	ClientNotFound,
	Tungstenite,
}

impl From<io::Error> for Error {
	#[inline(always)]
	fn from(_: io::Error) -> Self {
		Self::Io
	}
}

impl From<tungstenite::Error> for Error {
	#[inline(always)]
	fn from(_: tungstenite::Error) -> Self {
		Self::Tungstenite
	}
}
