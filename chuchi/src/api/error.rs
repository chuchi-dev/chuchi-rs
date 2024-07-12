use std::fmt::{Debug, Display};

pub use crate::header::StatusCode;

use crate::request::{DeserializeError, SerializeError};
use serde::Serialize;

/*
Errors we encounter at the moment

headers_missing

deserialize error

serialize error


*/

/// The error that is sent if something goes wrong while responding
/// to a request.
/// ## Panics
/// If deserialization or serialization failes
/// this will result in a panic
pub trait ApiError: Debug + Display + Serialize {
	fn from_error(e: Error) -> Self;

	fn status_code(&self) -> StatusCode;
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
	/// Some headers are missing
	///
	/// Not all headers from this list might be missing
	#[error("Headers missing: {0:?}")]
	HeadersMissing(&'static [&'static str]),

	/// Deserialization failed
	#[error("Deserialize error: {0}")]
	Deserialize(DeserializeError),

	/// Serialization failed
	#[error("Serialize error: {0}")]
	Serialize(SerializeError),

	#[error("Extraction error: {0}")]
	ExtractionError(Box<dyn std::error::Error + Send + Sync>),

	/// Some internal Error
	#[error("Internal error: {0}")]
	Chuchi(crate::Error),
}
