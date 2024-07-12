use super::Request;
use crate::body::Body;
use crate::header::{
	values::IntoHeaderName, ContentType, HeaderValue, HeaderValues, Method,
	RequestHeader, Uri, CONTENT_LENGTH, CONTENT_TYPE,
};

use std::fmt;
use std::net::SocketAddr;

/// A builder to create a `Request`.
///
/// This is only useful for direct request calling on a FirePit.
#[derive(Debug)]
pub struct RequestBuilder {
	header: RequestHeader,
	body: Body,
}

impl RequestBuilder {
	/// Creates a new `RequestBuilder`.
	pub fn new(uri: Uri) -> Self {
		Self {
			header: RequestHeader {
				address: ([127, 0, 0, 1], 0).into(),
				method: Method::GET,
				uri,
				values: HeaderValues::new(),
			},
			body: Body::new(),
		}
	}

	/// Sets the address.
	pub fn address(mut self, addr: impl Into<SocketAddr>) -> Self {
		self.header.address = addr.into();
		self
	}

	/// Set the method.
	pub fn method(mut self, method: Method) -> Self {
		self.header.method = method;
		self
	}

	/// Sets the content type.
	pub fn content_type(self, content_type: impl Into<ContentType>) -> Self {
		self.header(CONTENT_TYPE, content_type.into())
	}

	/// Sets a header value.
	///
	/// ## Note
	/// Only ASCII characters are allowed, use
	/// `self.values_mut().insert_encoded()` to allow any character.
	///
	/// ## Panics
	/// If the value is not a valid `HeaderValue`.
	pub fn header<K, V>(mut self, key: K, val: V) -> Self
	where
		K: IntoHeaderName,
		V: TryInto<HeaderValue>,
		V::Error: fmt::Debug,
	{
		self.values_mut().insert(key, val);
		self
	}

	/// Returns `HeaderValues` mutably.
	pub fn values_mut(&mut self) -> &mut HeaderValues {
		&mut self.header.values
	}

	/// Sets the body dropping the previous one.
	pub fn body(mut self, body: impl Into<Body>) -> Self {
		self.body = body.into();
		self
	}

	/// Sets the body from a serialized value
	#[cfg(feature = "json")]
	#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
	pub fn serialize<S>(self, value: &S) -> Result<Self, super::SerializeError>
	where
		S: serde::Serialize + ?Sized,
	{
		Ok(self
			.body(Body::serialize(value).map_err(super::SerializeError::Json)?))
	}

	/// Serializes the value to a query string and appends it to the path.
	///
	/// ## Note
	/// This replaces the previous query.
	#[cfg(feature = "query")]
	#[cfg_attr(docsrs, doc(cfg(feature = "query")))]
	pub fn serialize_query<S>(
		mut self,
		value: &S,
	) -> Result<Self, super::SerializeError>
	where
		S: serde::Serialize + ?Sized,
	{
		let mut parts = self.header.uri.into_parts();

		let query = serde_urlencoded::to_string(value)
			.map_err(super::SerializeError::UrlEncoded)?;

		parts.path_and_query = Some(
			format!(
				"{}?{}",
				parts
					.path_and_query
					.as_ref()
					.map(|p| p.path())
					.unwrap_or("/"),
				query
			)
			.parse()
			.expect("serde_urlencoded should always return a valid query"),
		);

		self.header.uri = Uri::from_parts(parts).unwrap();

		Ok(self)
	}

	/// Builds a `Request`. Adding the `content-length` header
	/// if the len of the body is known.
	pub fn build(mut self) -> Request {
		// lets calculate content-length
		// if the body size is already known
		if let Some(len) = self.body.len() {
			self.values_mut().insert(CONTENT_LENGTH, len);
		}

		Request::new(self.header, self.body)
	}
}
