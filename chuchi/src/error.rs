use crate::header::StatusCode;

use std::error::Error as StdError;
use std::{fmt, io};

pub type Result<T> = std::result::Result<T, Error>;

/// A universal error type which contains a source and a kind.
///
/// An error is either associated with the client or the server.
#[derive(Debug)]
pub struct Error {
	kind: ErrorKind,
	source: Option<Box<dyn StdError + Send + Sync>>,
}

impl Error {
	/// Creates a new error.
	pub fn new<K, E>(kind: K, error: E) -> Self
	where
		K: Into<ErrorKind>,
		E: Into<Box<dyn StdError + Send + Sync>>,
	{
		Self {
			kind: kind.into(),
			source: Some(error.into()),
		}
	}

	/// Creates a new error without a source.
	pub fn empty<K>(kind: K) -> Self
	where
		K: Into<ErrorKind>,
	{
		Self {
			kind: kind.into(),
			source: None,
		}
	}

	/// Returns the `StatusCode` corresponding to the `ErrorKind`.
	pub fn status_code(&self) -> StatusCode {
		match self.kind {
			ErrorKind::Client(c) => c.into(),
			ErrorKind::Server(s) => s.into(),
		}
	}

	/// Returns a new error from an io::Error originating from the client.
	pub fn from_client_io(error: io::Error) -> Self {
		// try to detect if source is known to us
		Self::new(ClientErrorKind::from_io(&error), error)
	}

	/// Returns a new error originating from the server.
	pub fn from_server_error<E>(error: E) -> Self
	where
		E: Into<Box<dyn StdError + Send + Sync>>,
	{
		Self::new(ServerErrorKind::InternalServerError, error)
	}
}

impl<T> From<T> for Error
where
	T: Into<ErrorKind>,
{
	fn from(e: T) -> Self {
		Self::empty(e)
	}
}

#[cfg(feature = "json")]
mod deserialize_error {
	use super::*;

	use crate::request::DeserializeError;

	impl From<DeserializeError> for Error {
		fn from(e: DeserializeError) -> Self {
			Self::new(ClientErrorKind::BadRequest, e)
		}
	}
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

impl StdError for Error {
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		self.source.as_ref().and_then(|e| e.source())
	}
}

/// An error can either come from the client or the server.
#[derive(Debug)]
pub enum ErrorKind {
	Client(ClientErrorKind),
	Server(ServerErrorKind),
}

impl From<ClientErrorKind> for ErrorKind {
	fn from(k: ClientErrorKind) -> Self {
		Self::Client(k)
	}
}

impl From<ServerErrorKind> for ErrorKind {
	fn from(k: ServerErrorKind) -> Self {
		Self::Server(k)
	}
}

macro_rules! error_kind {
	($name:ident, $($kind:ident => $status:ident),*) => (
		#[derive(Debug, Clone, Copy, PartialEq, Eq)]
		pub enum $name {
			$($kind),*
		}

		impl From<$name> for StatusCode {
			fn from(k: $name) -> Self {
				match k {
					$($name::$kind => Self::$status),*
				}
			}
		}
	)
}

// impl ClientErrorKind
error_kind!( ClientErrorKind,
	BadRequest => BAD_REQUEST,
	Unauthorized => UNAUTHORIZED,
	PaymentRequired => PAYMENT_REQUIRED,
	Forbidden => FORBIDDEN,
	NotFound => NOT_FOUND,
	MethodNotAllowed => METHOD_NOT_ALLOWED,
	NotAcceptable => NOT_ACCEPTABLE,
	ProxyAuthenticationRequired => PROXY_AUTHENTICATION_REQUIRED,
	RequestTimeout => REQUEST_TIMEOUT,
	Conflict => CONFLICT,
	Gone => GONE,
	LengthRequired => LENGTH_REQUIRED,
	PreconditionFailed => PRECONDITION_FAILED,
	RequestEntityTooLarge => PAYLOAD_TOO_LARGE,
	RequestURITooLarge => URI_TOO_LONG,
	UnsupportedMediaType => UNSUPPORTED_MEDIA_TYPE,
	RequestedRangeNotSatisfiable => RANGE_NOT_SATISFIABLE,
	ExpectationFailed => EXPECTATION_FAILED
);

impl ClientErrorKind {
	/// Converts an io::Error into the appropriate kind.
	pub fn from_io(error: &io::Error) -> Self {
		use io::ErrorKind::*;
		match error.kind() {
			NotFound => Self::NotFound,
			PermissionDenied => Self::Unauthorized,
			// this should probably not happen?
			AlreadyExists => Self::Conflict,
			UnexpectedEof => Self::RequestEntityTooLarge,
			InvalidInput | InvalidData | Other => Self::BadRequest,
			TimedOut => Self::RequestTimeout,
			_ => Self::ExpectationFailed,
		}
	}
}

// Impl ServerErrorKind
error_kind!( ServerErrorKind,
	InternalServerError => INTERNAL_SERVER_ERROR,
	NotImplemented => NOT_IMPLEMENTED,
	BadGateway => BAD_GATEWAY,
	ServiceUnavailable => SERVICE_UNAVAILABLE,
	GatewayTimeout => GATEWAY_TIMEOUT
);
