#[doc(hidden)]
pub mod util;

use crate::extractor::Extractor;

use std::convert::Infallible;
use std::fmt;
use std::str::Utf8Error;

use hyper_util::rt::TokioIo;
use tracing::warn;

#[doc(hidden)]
pub use hyper::upgrade;

use tokio_tungstenite::tungstenite;
use tokio_tungstenite::WebSocketStream;
use tungstenite::protocol::Role;

// rexport
use tungstenite::protocol::Message as ProtMessage;
pub use tungstenite::{
	error::Error, protocol::frame::coding::CloseCode, protocol::CloseFrame,
};

use futures_util::sink::SinkExt;
use futures_util::stream::StreamExt;

pub trait LogWebSocketReturn: fmt::Debug {
	fn should_log_error(&self) -> bool;
}

impl<T, E> LogWebSocketReturn for Result<T, E>
where
	T: fmt::Debug,
	E: fmt::Debug,
{
	fn should_log_error(&self) -> bool {
		self.is_err()
	}
}

impl LogWebSocketReturn for () {
	fn should_log_error(&self) -> bool {
		false
	}
}

#[cfg(feature = "json")]
macro_rules! try2 {
	($e:expr) => {
		match $e {
			Some(v) => v,
			None => return Ok(None),
		}
	};
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Message {
	Text(String),
	Binary(Vec<u8>),
}

impl Message {
	pub fn into_data(self) -> Vec<u8> {
		match self {
			Self::Text(t) => t.into(),
			Self::Binary(b) => b,
		}
	}

	pub fn to_text(&self) -> Result<&str, Utf8Error> {
		match self {
			Self::Text(t) => Ok(&t),
			Self::Binary(b) => std::str::from_utf8(b),
		}
	}
}

impl From<String> for Message {
	fn from(s: String) -> Self {
		Self::Text(s)
	}
}

impl From<&str> for Message {
	fn from(s: &str) -> Self {
		Self::Text(s.into())
	}
}

impl From<Vec<u8>> for Message {
	fn from(v: Vec<u8>) -> Self {
		Self::Binary(v)
	}
}

impl From<&[u8]> for Message {
	fn from(v: &[u8]) -> Self {
		Self::Binary(v.into())
	}
}

impl From<Message> for ProtMessage {
	fn from(m: Message) -> Self {
		match m {
			Message::Text(t) => Self::Text(t),
			Message::Binary(b) => Self::Binary(b),
		}
	}
}

#[derive(Debug)]
pub struct WebSocket {
	inner: WebSocketStream<TokioIo<upgrade::Upgraded>>,
}

impl WebSocket {
	pub async fn new(upgraded: upgrade::Upgraded) -> Self {
		Self {
			inner: WebSocketStream::from_raw_socket(
				TokioIo::new(upgraded),
				Role::Server,
				None,
			)
			.await,
		}
	}

	// used for tests
	#[doc(hidden)]
	pub fn from_raw(
		inner: WebSocketStream<TokioIo<upgrade::Upgraded>>,
	) -> Self {
		Self { inner }
	}

	/// Handles Ping and Pong messages
	///
	/// never returns Error::ConnectionClose | Error::AlreadyClosed
	pub async fn receive(&mut self) -> Result<Option<Message>, Error> {
		// loop used to handle Message::Pong | Message::Ping
		loop {
			let res = self.inner.next().await.transpose();
			return match res {
				Ok(None) => Ok(None),
				Ok(Some(ProtMessage::Text(t))) => Ok(Some(Message::Text(t))),
				Ok(Some(ProtMessage::Binary(b))) => {
					Ok(Some(Message::Binary(b)))
				}
				Ok(Some(ProtMessage::Ping(d))) => {
					// respond with a pong
					self.inner.send(ProtMessage::Pong(d)).await?;
					// then listen for a new message
					continue;
				}
				Ok(Some(ProtMessage::Pong(_))) => continue,
				Ok(Some(ProtMessage::Close(_))) => Ok(None),
				Ok(Some(ProtMessage::Frame(f))) => {
					warn!("we received a websocket frame {:?}", f);
					// Todod should we do something about this frame??
					continue;
				}
				Err(Error::ConnectionClosed) | Err(Error::AlreadyClosed) => {
					Ok(None)
				}
				Err(e) => Err(e),
			};
		}
	}

	pub async fn send<M>(&mut self, msg: M) -> Result<(), Error>
	where
		M: Into<Message>,
	{
		self.inner.send(msg.into().into()).await
	}

	pub async fn close(&mut self, code: CloseCode, reason: String) {
		let _ = self
			.inner
			.send(ProtMessage::Close(Some(CloseFrame {
				code,
				reason: reason.into(),
			})))
			.await;
		let _ = self.inner.close(None).await;
		// close is close
		// don't mind if you could send close or not
	}

	pub async fn ping(&mut self) -> Result<(), Error> {
		self.inner.send(ProtMessage::Ping(vec![])).await
	}

	/// calls receive and then deserialize
	#[cfg(feature = "json")]
	#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
	pub async fn deserialize<D>(&mut self) -> Result<Option<D>, JsonError>
	where
		D: serde::de::DeserializeOwned,
	{
		let msg = try2!(self.receive().await?).into_data();
		serde_json::from_slice(&msg)
			.map(|d| Some(d))
			.map_err(|e| e.into())
	}

	/// calls serialize then send
	#[cfg(feature = "json")]
	#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
	pub async fn serialize<S: ?Sized>(
		&mut self,
		value: &S,
	) -> Result<(), JsonError>
	where
		S: serde::Serialize,
	{
		let v = serde_json::to_string(value)?;
		self.send(v).await.map_err(|e| e.into())
	}
}

#[cfg(feature = "json")]
mod json_error {

	use super::Error;
	use std::fmt;

	#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
	#[derive(Debug)]
	pub enum JsonError {
		ConnectionError(Error),
		SerdeError(serde_json::Error),
	}

	impl From<Error> for JsonError {
		fn from(e: Error) -> Self {
			Self::ConnectionError(e)
		}
	}

	impl From<serde_json::Error> for JsonError {
		fn from(e: serde_json::Error) -> Self {
			Self::SerdeError(e)
		}
	}

	impl fmt::Display for JsonError {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			fmt::Debug::fmt(self, f)
		}
	}

	impl std::error::Error for JsonError {
		fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
			match self {
				Self::ConnectionError(e) => Some(e),
				Self::SerdeError(e) => Some(e),
			}
		}
	}
}

#[cfg(feature = "json")]
pub use json_error::JsonError;

impl<'a> Extractor<'a, WebSocket> for WebSocket {
	type Error = Infallible;
	type Prepared = ();

	extractor_validate!();

	extractor_prepare!();

	extractor_extract!(<WebSocket> |extract| {
		Ok(extract.request.take().unwrap())
	});
}
