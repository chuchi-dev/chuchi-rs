mod sync_reader;
use sync_reader::sync_reader_into_bytes;
pub use sync_reader::BodySyncReader;

mod async_reader;
use async_reader::async_reader_into_bytes;
pub use async_reader::BodyAsyncReader;

mod async_bytes_streamer;
use async_bytes_streamer::async_bytes_streamer_into_bytes;
pub use async_bytes_streamer::BodyAsyncBytesStreamer;

mod body_http;
pub use body_http::BodyHttp;
use body_http::HyperBodyAsAsyncBytesStream;

use std::io::Read as SyncRead;
use std::pin::Pin;
use std::time::Duration;
use std::{fmt, io, mem};

use tokio::io::AsyncRead;
use tokio::task;

use futures_core::Stream as AsyncStream;

use bytes::Bytes;
use hyper::body::Incoming;

type PinnedAsyncRead = Pin<Box<dyn AsyncRead + Send + Sync>>;
type BoxedSyncRead = Box<dyn SyncRead + Send + Sync>;
type PinnedAsyncBytesStream =
	Pin<Box<dyn AsyncStream<Item = io::Result<Bytes>> + Send + Sync>>;

enum Inner {
	Empty,
	// Bytes will never be empty
	Bytes(Bytes),
	Hyper(Incoming),
	SyncReader(BoxedSyncRead),
	AsyncReader(PinnedAsyncRead),
	AsyncBytesStreamer(PinnedAsyncBytesStream),
}

impl fmt::Debug for Inner {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::Empty => f.write_str("Empty"),
			Self::Bytes(b) => f.debug_tuple("Bytes").field(&b.len()).finish(),
			Self::Hyper(_) => f.write_str("Hyper"),
			Self::SyncReader(_) => f.write_str("SyncReader"),
			Self::AsyncReader(_) => f.write_str("AsyncReader"),
			Self::AsyncBytesStreamer(_) => f.write_str("AsyncBytesStreamer"),
		}
	}
}

impl Default for Inner {
	fn default() -> Self {
		Self::Empty
	}
}

#[derive(Debug, Clone, Default)]
struct Constraints {
	timeout: Option<Duration>,
	size: Option<usize>,
}

#[derive(Debug, Default)]
pub struct Body {
	inner: Inner,
	constraints: Constraints,
}

impl Body {
	fn new_inner(inner: Inner) -> Self {
		Self {
			inner,
			constraints: Constraints::default(),
		}
	}

	/// Creates a new empty `Body`.
	pub fn new() -> Self {
		Self::new_inner(Inner::Empty)
	}

	/// Creates a new `Body` from the given bytes.
	pub fn from_bytes(bytes: impl Into<Bytes>) -> Self {
		let bytes = bytes.into();
		if !bytes.is_empty() {
			Self::new_inner(Inner::Bytes(bytes))
		} else {
			Self::new()
		}
	}

	/// Creates a new `Body` from the given bytes.
	pub fn copy_from_slice(slice: impl AsRef<[u8]>) -> Self {
		let slice = slice.as_ref();
		if !slice.is_empty() {
			Self::new_inner(Inner::Bytes(Bytes::copy_from_slice(slice)))
		} else {
			Self::new()
		}
	}

	/// Creates a new Body from a `hyper::body::Incoming`.
	pub fn from_hyper(body: Incoming) -> Self {
		Self::new_inner(Inner::Hyper(body))
	}

	/// Creates a new Body from a `Read` implementation.
	pub fn from_sync_reader<R>(reader: R) -> Self
	where
		R: SyncRead + Send + Sync + 'static,
	{
		Self::new_inner(Inner::SyncReader(Box::new(reader)))
	}

	/// Creates a new Body from an `AsyncRead` implementation.
	pub fn from_async_reader<R>(reader: R) -> Self
	where
		R: AsyncRead + Send + Sync + 'static,
	{
		Self::new_inner(Inner::AsyncReader(Box::pin(reader)))
	}

	/// Creates a new Body from a `Stream<Item=io::Result<Bytes>>`
	/// implementation.
	pub fn from_async_bytes_streamer<S>(streamer: S) -> Self
	where
		S: AsyncStream<Item = io::Result<Bytes>> + Send + Sync + 'static,
	{
		Self::new_inner(Inner::AsyncBytesStreamer(Box::pin(streamer)))
	}

	/// Creates a new Body from a serializeable object.
	#[cfg(feature = "json")]
	#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
	pub fn serialize<S>(value: &S) -> Result<Self, serde_json::Error>
	where
		S: serde::Serialize + ?Sized,
	{
		serde_json::to_vec(value).map(|v| v.into())
	}

	/// Returns true if we know the body is empty, the body still might be empty
	/// but we just don't know it yet
	pub fn is_empty(&self) -> bool {
		// we don't need to check the Inner::Bytes(b) since it will never
		// be empty
		matches!(self.inner, Inner::Empty)
	}

	/// Returns a length if it is already known.
	pub fn len(&self) -> Option<usize> {
		match &self.inner {
			Inner::Empty => Some(0),
			Inner::Bytes(b) => Some(b.len()),
			_ => None,
		}
	}

	/// Sets a read size limit.
	pub fn set_size_limit(&mut self, size: Option<usize>) {
		self.constraints.size = size;
	}

	/// Sets a read timeout, the timer starts counting after you call into_*
	pub fn set_timeout(&mut self, timeout: Option<Duration>) {
		self.constraints.timeout = timeout;
	}

	/// Takes the body and replaces it with an empty one.
	pub fn take(&mut self) -> Self {
		mem::take(self)
	}

	/// Converts the Body into Bytes.
	pub async fn into_bytes(self) -> io::Result<Bytes> {
		match self.inner {
			Inner::Empty => Ok(Bytes::new()),
			Inner::Bytes(b) => {
				if let Some(size_limit) = self.constraints.size {
					if b.len() > size_limit {
						return Err(size_limit_reached("Bytes to big"));
					}
				}
				Ok(b)
			}
			Inner::Hyper(i) => {
				async_bytes_streamer_into_bytes(
					HyperBodyAsAsyncBytesStream::new(i),
					self.constraints,
				)
				.await
			}
			Inner::SyncReader(r) => task::spawn_blocking(|| {
				sync_reader_into_bytes(r, self.constraints)
			})
			.await
			.map_err(join_error)?,
			Inner::AsyncReader(r) => {
				async_reader_into_bytes(r, self.constraints).await
			}
			Inner::AsyncBytesStreamer(s) => {
				async_bytes_streamer_into_bytes(s, self.constraints).await
			}
		}
	}

	/// Converts the Body into a string.
	pub async fn into_string(self) -> io::Result<String> {
		let bytes = self.into_bytes().await?;
		String::from_utf8(bytes.into())
			.map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
	}

	/// Converts the Body into a type that implements `Read`.
	pub fn into_sync_reader(self) -> BodySyncReader {
		BodySyncReader::new(self.inner, self.constraints)
	}

	/// Converts the Body into a type that implements `AsyncRead`.
	pub fn into_async_reader(self) -> BodyAsyncReader {
		BodyAsyncReader::new(self.inner, self.constraints)
	}

	/// Converts the Body into a type that implements
	/// `Stream<Item=io::Result<Bytes>>`.
	pub fn into_async_bytes_streamer(self) -> BodyAsyncBytesStreamer {
		BodyAsyncBytesStreamer::new(self.inner, self.constraints)
	}

	/// Converts the Body into a type that implements `hyper::body::Body`.
	pub fn into_http_body(self) -> BodyHttp {
		BodyHttp::new(self.inner, self.constraints)
	}

	/// Converts the Body into a deserializeable type.
	#[cfg(feature = "json")]
	#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
	pub async fn deserialize<D>(self) -> Result<D, serde_json::Error>
	where
		D: serde::de::DeserializeOwned + Send + 'static,
	{
		let reader = self.into_sync_reader();
		if reader.needs_spawn_blocking() {
			task::spawn_blocking(|| serde_json::from_reader(reader))
				.await
				.expect("deserializing panicked")
		} else {
			serde_json::from_reader(reader)
		}
	}
}

impl From<Bytes> for Body {
	fn from(b: Bytes) -> Self {
		Self::from_bytes(b)
	}
}

impl From<Vec<u8>> for Body {
	fn from(b: Vec<u8>) -> Self {
		Self::from_bytes(b)
	}
}

impl From<String> for Body {
	fn from(s: String) -> Self {
		Self::from_bytes(s)
	}
}

impl From<&'static str> for Body {
	fn from(s: &'static str) -> Self {
		Self::from_bytes(Bytes::from_static(s.as_bytes()))
	}
}

impl From<Incoming> for Body {
	fn from(i: Incoming) -> Self {
		Self::from_hyper(i)
	}
}

fn size_limit_reached(msg: &'static str) -> io::Error {
	io::Error::new(io::ErrorKind::UnexpectedEof, msg)
}

fn timed_out(msg: &'static str) -> io::Error {
	io::Error::new(io::ErrorKind::TimedOut, msg)
}

fn join_error(error: task::JoinError) -> io::Error {
	io::Error::new(io::ErrorKind::Other, error)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn is_unpin<T: Unpin>() {}
	fn is_send<T: Send>() {}
	fn is_sync<T: Sync>() {}

	#[test]
	fn test_body() {
		is_unpin::<Body>();
		is_send::<Body>();
		is_sync::<Body>();
	}
}

#[cfg(all(test, feature = "json"))]
mod json_tests {
	use super::*;
	use serde::{Deserialize, Serialize};

	#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
	enum SomeEnum {
		Abc(String),
	}

	#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
	struct Struct1 {
		some_data: String,
		some_enum: SomeEnum,
	}

	#[tokio::test]
	async fn test_serde() {
		let s1 = Struct1 {
			some_data: "test".into(),
			some_enum: SomeEnum::Abc("test2".into()),
		};

		let body = Body::serialize(&s1).unwrap();
		let v: Struct1 = body.deserialize().await.unwrap();

		assert_eq!(s1, v);
	}
}
