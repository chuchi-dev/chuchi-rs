mod builder;
pub use builder::RequestBuilder;

use crate::body::Body;
#[cfg(feature = "json")]
use crate::header::CONTENT_TYPE;
use crate::header::{RequestHeader, Uri};

use std::time::Duration;

/// The request that is received from a client.
#[derive(Debug)]
pub struct Request {
	pub header: RequestHeader,
	pub body: Body,
}

impl Request {
	/// Creates a new `Request`.
	pub fn new(header: RequestHeader, body: Body) -> Self {
		Self { header, body }
	}

	/// Creates a new `Request` with a builder.
	pub fn builder(uri: Uri) -> RequestBuilder {
		RequestBuilder::new(uri)
	}

	/// Takes the body replacing it with an empty one.
	pub fn take_body(&mut self) -> Body {
		self.body.take()
	}

	/// Get the request header by reference.
	pub fn header(&self) -> &RequestHeader {
		&self.header
	}

	/// Sets a read size limit.
	pub fn set_size_limit(&mut self, size: Option<usize>) {
		self.body.set_size_limit(size)
	}

	/// Sets a read timeout, the timer starts counting after you call into_*
	pub fn set_timeout(&mut self, timeout: Option<Duration>) {
		self.body.set_timeout(timeout)
	}

	/// Tries to deserialize the request body.
	///
	/// ## Errors
	/// - If the header `content-type` does not contain `application/json`.
	/// - If the body does not contain a valid json or some data is missing.
	#[cfg(feature = "json")]
	#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
	pub async fn deserialize<D>(&mut self) -> Result<D, DeserializeError>
	where
		D: serde::de::DeserializeOwned + Send + 'static,
	{
		use crate::header::Mime;

		// try to read mime
		// this will not work if content-type has charset
		// TODO allow charset (probably implement Parse for ContentType)
		let raw_content_type = self
			.header()
			.value(CONTENT_TYPE)
			.ok_or(DeserializeError::NoContentType)?;
		let mime: Mime = raw_content_type.trim().parse().map_err(|_| {
			DeserializeError::UnknownContentType(raw_content_type.to_string())
		})?;

		if mime != Mime::JSON {
			return Err(DeserializeError::WrongMimeType(mime));
		}

		// now parse body
		self.body
			.take()
			.deserialize()
			.await
			.map_err(DeserializeError::Json)
	}

	#[cfg(feature = "query")]
	#[cfg_attr(docsrs, doc(cfg(feature = "query")))]
	pub fn deserialize_query<D>(&self) -> Result<D, DeserializeError>
	where
		D: serde::de::DeserializeOwned + Send + 'static,
	{
		let query = self.header().uri().query().unwrap_or("");

		serde_urlencoded::from_str(query)
			.map_err(DeserializeError::UrlEncoded)
	}
}

#[cfg(any(feature = "json", feature = "query"))]
mod serde_error {
	use crate::header::Mime;

	use std::fmt;

	#[derive(Debug)]
	#[non_exhaustive]
	pub enum DeserializeError {
		NoContentType,
		UnknownContentType(String),
		WrongMimeType(Mime),
		#[cfg(feature = "json")]
		Json(serde_json::Error),
		#[cfg(feature = "query")]
		UrlEncoded(serde::de::value::Error),
	}

	impl fmt::Display for DeserializeError {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			write!(f, "Failed to deserialize request {:?}", self)
		}
	}

	impl std::error::Error for DeserializeError {}

	#[derive(Debug)]
	#[non_exhaustive]
	pub enum SerializeError {
		#[cfg(feature = "json")]
		Json(serde_json::Error),
		#[cfg(feature = "query")]
		UrlEncoded(serde_urlencoded::ser::Error),
	}

	impl fmt::Display for SerializeError {
		fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
			write!(f, "Failed to serialize request {:?}", self)
		}
	}

	impl std::error::Error for SerializeError {}
}

#[cfg(any(feature = "json", feature = "query"))]
pub use serde_error::*;

#[cfg(test)]
mod tests {
	#[allow(unused_imports)]
	use super::*;

	#[cfg(feature = "query")]
	#[tokio::test]
	async fn deserialize_query() {
		let uri = "http://localhost:8080/?a=1&b=2";
		let req = Request::builder(uri.parse().unwrap()).build();

		#[derive(serde::Deserialize)]
		struct Query {
			a: String,
			b: String,
			c: Option<String>,
		}

		let query: Query = req.deserialize_query().unwrap();
		assert_eq!(query.a, "1");
		assert_eq!(query.b, "2");
		assert_eq!(query.c, None);
	}
}
