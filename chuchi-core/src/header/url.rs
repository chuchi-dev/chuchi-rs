use super::Uri;

use http::uri::{Authority, PathAndQuery, Scheme};

pub use form_urlencoded::Parse as QueryIter;

/// Contains a request url.
///
/// This is a wrapper around `Uri` with the caveat that a scheme
/// and an authority is set, which makes it a Url.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Url {
	scheme: Scheme,
	authority: Authority,
	path_and_query: PathAndQuery,
}

impl Url {
	/// Creates a new `Uri` from an `http::Uri`
	///
	/// Returns None if the `http::Uri` does not contain a scheme or authority.
	pub fn from_inner(inner: Uri) -> Option<Self> {
		let parts = inner.into_parts();
		Some(Self {
			scheme: parts.scheme?,
			authority: parts.authority?,
			path_and_query: parts
				.path_and_query
				.unwrap_or_else(|| PathAndQuery::from_static("/")),
		})
	}

	/// Returns the used scheme.
	pub fn scheme(&self) -> &str {
		self.scheme.as_str()
	}

	/// Returns true if the used scheme is https.
	pub fn is_https(&self) -> bool {
		self.scheme == Scheme::HTTPS
	}

	/// Returns true if the used scheme is http.
	pub fn is_http(&self) -> bool {
		self.scheme == Scheme::HTTP
	}

	/// Returns the host.
	pub fn host(&self) -> &str {
		self.authority.host()
	}

	/// Returns the used port if any.
	pub fn port(&self) -> Option<u16> {
		self.authority.port_u16()
	}

	/// Returns the path.
	pub fn path(&self) -> &str {
		self.path_and_query.path()
	}

	/// Returns the path as segments divided by a slash, first starting and
	/// ending slash removed.
	pub fn path_segments(&self) -> std::str::Split<'_, char> {
		let path = self.path();
		let path = path.strip_prefix('/').unwrap_or(path);
		let path = path.strip_suffix('/').unwrap_or(path);
		path.split('/')
	}

	/// Returns the query string.
	pub fn query(&self) -> Option<&str> {
		self.path_and_query.query()
	}

	// named as parse_query_pairs since maybe it would make sense
	// to make a separate type which allows to lookup pairs
	// and deserialize values in it which would be in `query_pairs`
	//
	/// Returns an iterator with the Item `(Cow<'_, str>, Cow<'_, str>)`
	///
	/// Key and values are percent decoded.
	pub fn parse_query_pairs(&self) -> QueryIter {
		form_urlencoded::parse(self.query().unwrap_or("").as_bytes())
	}
}
