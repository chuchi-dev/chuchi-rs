use std::borrow::Cow;
use std::fmt;

pub use serde_json::Error as JsonError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecoverableError(Cow<'static, str>);

impl From<&'static str> for UnrecoverableError {
	fn from(s: &'static str) -> Self {
		Self(s.into())
	}
}

impl From<String> for UnrecoverableError {
	fn from(s: String) -> Self {
		Self(s.into())
	}
}

impl fmt::Display for UnrecoverableError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

#[derive(Debug)]
pub enum StreamError {
	Closed,
	Json(JsonError),
}

impl std::error::Error for StreamError {}

impl fmt::Display for StreamError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}
