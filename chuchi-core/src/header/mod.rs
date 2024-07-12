use std::net::SocketAddr;

pub use http::{Method, StatusCode, Uri};

pub mod url;
pub use url::Url;

mod contenttype;
pub use contenttype::{ContentType, Mime};

pub mod values;
pub use values::{HeaderValue, HeaderValues};

pub use constants::*;

/// RequestHeader received from a client.
#[derive(Debug, Clone)]
pub struct RequestHeader {
	pub address: SocketAddr,
	pub method: Method,
	pub uri: Uri,
	pub values: HeaderValues,
}

impl RequestHeader {
	/// Returns the ip address of the requesting client.
	pub fn address(&self) -> &SocketAddr {
		&self.address
	}

	/// Returns the requesting method.
	pub fn method(&self) -> &Method {
		&self.method
	}

	/// Returns the requesting uri.
	pub fn uri(&self) -> &Uri {
		&self.uri
	}

	pub fn to_url(&self) -> Option<Url> {
		Url::from_inner(self.uri.clone())
	}

	/// Returns all header values.
	pub fn values(&self) -> &HeaderValues {
		&self.values
	}

	/// Returns a header value from it's key if it exists and is valid ascii.
	///
	/// ## Note
	/// If you wan't a decoded value use `self.values().get_decoded(key)`.
	pub fn value<K>(&self, key: K) -> Option<&str>
	where
		K: values::AsHeaderName,
	{
		self.values.get_str(key)
	}
}

/// ResponseHeader created from a server.
///
/// To create a ResponseHeader you should probably
/// use ResponseHeaderBuilder.
#[derive(Debug, Clone)]
pub struct ResponseHeader {
	pub status_code: StatusCode,
	pub content_type: ContentType,
	pub values: HeaderValues,
}

impl ResponseHeader {
	/// Returns the used status code.
	pub fn status_code(&self) -> &StatusCode {
		&self.status_code
	}

	/// Returns the used content type.
	pub fn content_type(&self) -> &ContentType {
		&self.content_type
	}

	/// Returns all header values.
	pub fn values(&self) -> &HeaderValues {
		&self.values
	}

	/// Returns a header value from it's key if it exists and is valid ascii.
	///
	/// ## Note
	/// If you wan't a decoded value use `self.values().get_decoded(key)`.
	pub fn value<K>(&self, key: K) -> Option<&str>
	where
		K: values::AsHeaderName,
	{
		self.values.get_str(key)
	}
}

impl Default for ResponseHeader {
	fn default() -> Self {
		Self {
			status_code: StatusCode::OK,
			content_type: ContentType::None,
			values: HeaderValues::new(),
		}
	}
}

mod constants {
	pub use hyper::header::{
		ACCEPT, ACCEPT_CHARSET, ACCEPT_ENCODING, ACCEPT_LANGUAGE,
		ACCEPT_RANGES, ACCESS_CONTROL_ALLOW_CREDENTIALS,
		ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
		ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS,
		ACCESS_CONTROL_MAX_AGE, ACCESS_CONTROL_REQUEST_HEADERS,
		ACCESS_CONTROL_REQUEST_METHOD, AGE, ALLOW, ALT_SVC, AUTHORIZATION,
		CACHE_CONTROL, CONNECTION, CONTENT_DISPOSITION, CONTENT_ENCODING,
		CONTENT_LANGUAGE, CONTENT_LENGTH, CONTENT_LOCATION, CONTENT_RANGE,
		CONTENT_SECURITY_POLICY, CONTENT_SECURITY_POLICY_REPORT_ONLY,
		CONTENT_TYPE, COOKIE, DATE, DNT, ETAG, EXPECT, EXPIRES, FORWARDED,
		FROM, HOST, IF_MATCH, IF_MODIFIED_SINCE, IF_NONE_MATCH, IF_RANGE,
		IF_UNMODIFIED_SINCE, LAST_MODIFIED, LINK, LOCATION, MAX_FORWARDS,
		ORIGIN, PRAGMA, PROXY_AUTHENTICATE, PROXY_AUTHORIZATION,
		PUBLIC_KEY_PINS, PUBLIC_KEY_PINS_REPORT_ONLY, RANGE, REFERER,
		REFERRER_POLICY, REFRESH, RETRY_AFTER, SEC_WEBSOCKET_ACCEPT,
		SEC_WEBSOCKET_EXTENSIONS, SEC_WEBSOCKET_KEY, SEC_WEBSOCKET_PROTOCOL,
		SEC_WEBSOCKET_VERSION, SERVER, SET_COOKIE, STRICT_TRANSPORT_SECURITY,
		TE, TRAILER, TRANSFER_ENCODING, UPGRADE, UPGRADE_INSECURE_REQUESTS,
		USER_AGENT, VARY, VIA, WARNING, WWW_AUTHENTICATE,
		X_CONTENT_TYPE_OPTIONS, X_DNS_PREFETCH_CONTROL, X_FRAME_OPTIONS,
		X_XSS_PROTECTION,
	};
}
