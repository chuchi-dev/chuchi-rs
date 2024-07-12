use super::Response;
use crate::body::Body;
use crate::header::{
	values::IntoHeaderName, ContentType, HeaderValue, HeaderValues,
	ResponseHeader, StatusCode, CONTENT_LENGTH,
};

use std::fmt;

/// A builder to create a `Response`.
#[derive(Debug)]
pub struct ResponseBuilder {
	header: ResponseHeader,
	body: Body,
}

impl ResponseBuilder {
	/// Creates a new `ResponseBuilder`.
	pub fn new() -> Self {
		Self {
			header: ResponseHeader::default(),
			body: Body::new(),
		}
	}

	/// Sets the status code.
	pub fn status_code(mut self, status_code: StatusCode) -> Self {
		self.header.status_code = status_code;
		self
	}

	/// Sets the content type.
	pub fn content_type(
		mut self,
		content_type: impl Into<ContentType>,
	) -> Self {
		self.header.content_type = content_type.into();
		self
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

	/// Builds a `Response`. Adding the `content-length` header
	/// if the len of the body is known.
	pub fn build(mut self) -> Response {
		// lets calculate content-length
		// if the body size is already known
		if let Some(len) = self.body.len() {
			self.values_mut().insert(CONTENT_LENGTH, len);
		}

		Response::new(self.header, self.body)
	}
}

impl Default for ResponseBuilder {
	fn default() -> Self {
		Self::new()
	}
}
