use crate::header::{
	RequestHeader, ResponseHeader, StatusCode, CACHE_CONTROL, ETAG,
	IF_NONE_MATCH,
};
use crate::into::IntoResponse;
use crate::Response;

use std::fmt;
use std::time::Duration;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

// == 1day
const DEFAULT_MAX_AGE: Duration = Duration::from_secs(60 * 60 * 24);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Etag(String);

impl Etag {
	pub fn new() -> Self {
		let rand_str: String = thread_rng()
			.sample_iter(&Alphanumeric)
			.map(char::from)
			.take(30)
			.collect();

		Self(rand_str)
	}

	pub fn as_str(&self) -> &str {
		self.0.as_str()
	}
}

impl fmt::Display for Etag {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl From<Etag> for String {
	fn from(e: Etag) -> Self {
		e.0
	}
}

impl PartialEq<&str> for Etag {
	fn eq(&self, other: &&str) -> bool {
		self.as_str() == *other
	}
}

/// Controls if caching information should be sent.
///
/// The Caching struct contains an Etag which stores a random tag which
/// indentifies a specific file. The Etag gets generated when the struct gets
/// created.
///
/// ## Example
/// ```ignore
/// # use fire_http as fire;
/// use fire::{get, Request};
/// use fire::fs::Caching;
/// use fire::into::IntoResponse;
/// use std::cell::LazyCell;
///
///
/// const INDEX_CACHE: LazyCell<Caching> = LazyCell::new(|| {
/// 	Caching::default()
/// });
///
/// #[get("/")]
/// fn index(req: &mut Request) -> Response {
/// 	let cache = INDEX_CACHE.clone();
/// 	if cache.if_none_match(req.header()) {
/// 		return cache.into_response()
/// 	}
///
/// 	let mut resp = Response::html("<h1>Hello, World!</h1>");
/// 	cache.complete_header(&mut resp.header);
///
/// 	resp
/// }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caching {
	max_age: Duration,
	etag: Etag,
}

impl Caching {
	pub fn new(max_age: Duration) -> Self {
		Self {
			max_age,
			etag: Etag::new(),
		}
	}

	// defaults to 1 day
	pub fn default() -> Self {
		Self::new(DEFAULT_MAX_AGE)
	}

	pub fn if_none_match(&self, header: &RequestHeader) -> bool {
		header
			.value(IF_NONE_MATCH)
			.map(|none_match| none_match.len() == 30 && self.etag == none_match)
			.unwrap_or(false)
	}

	fn cache_control_string(&self) -> String {
		format!("max-age={}, public", self.max_age.as_secs())
	}

	pub fn complete_header(self, header: &mut ResponseHeader) {
		header
			.values
			.insert(CACHE_CONTROL, self.cache_control_string());

		// etag makes only sense with files not 404
		if header.status_code == StatusCode::OK {
			header.values.insert(ETAG, String::from(self.etag));
		}
	}
}

impl IntoResponse for Caching {
	fn into_response(self) -> Response {
		Response::builder()
			.status_code(StatusCode::NOT_MODIFIED)
			.header(CACHE_CONTROL, self.cache_control_string())
			.header(ETAG, String::from(self.etag))
			.build()
	}
}
