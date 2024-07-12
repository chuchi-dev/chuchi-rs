use super::static_files::CachingBuilder;
use super::{file, partial_file, Caching, Range};
use crate::header::Method;
use crate::into::IntoResponse;
use crate::routes::{ParamsNames, PathParams, RoutePath};
use crate::util::PinnedFuture;
use crate::{Error, IntoRoute, Request, Resources, Response, Route};

use std::io;
use std::time::Duration;

pub fn serve_memory_file(
	path: &'static str,
	bytes: &'static [u8],
	req: &Request,
	caching: Option<Caching>,
) -> io::Result<Response> {
	// check caching and if the etag matches return NOT_MODIFIED
	if matches!(&caching, Some(c) if c.if_none_match(req.header())) {
		return Ok(caching.unwrap().into_response());
	}

	let range = Range::parse(req.header());

	let mut res = match range {
		Some(range) => {
			partial_file::serve_memory_partial_file(path, bytes, range)?
		}
		None => file::serve_memory_file(path, bytes)?,
	};

	// set etag
	if let Some(caching) = caching {
		caching.complete_header(&mut res.header);
	}

	Ok(res)
}

/// Static get handler which servers/returns a file.
///
/// ## Example
/// ```
/// use chuchi::fs::MemoryFile;
/// use chuchi::memory_file;
///
/// const INDEX: MemoryFile = memory_file!(
/// 	"/",
/// 	"../../examples/www/hello_world.html"
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryFile {
	uri: &'static str,
	path: &'static str,
	bytes: &'static [u8],
	caching: CachingBuilder,
}

impl MemoryFile {
	/// Creates a `MemoryFile` with Default caching settings
	pub const fn new(
		uri: &'static str,
		path: &'static str,
		bytes: &'static [u8],
	) -> Self {
		Self {
			uri,
			path,
			bytes,
			caching: CachingBuilder::Default,
		}
	}

	pub const fn no_cache(
		uri: &'static str,
		path: &'static str,
		bytes: &'static [u8],
	) -> Self {
		Self {
			uri,
			path,
			bytes,
			caching: CachingBuilder::None,
		}
	}

	pub const fn cache_with_age(
		uri: &'static str,
		path: &'static str,
		bytes: &'static [u8],
		max_age: Duration,
	) -> Self {
		Self {
			uri,
			path,
			bytes,
			caching: CachingBuilder::MaxAge(max_age),
		}
	}
}

impl IntoRoute for MemoryFile {
	type IntoRoute = MemoryFileRoute;

	fn into_route(self) -> MemoryFileRoute {
		MemoryFileRoute {
			uri: self.uri,
			path: self.path,
			bytes: self.bytes,
			caching: self.caching.into(),
		}
	}
}

#[doc(hidden)]
pub struct MemoryFileRoute {
	uri: &'static str,
	path: &'static str,
	bytes: &'static [u8],
	caching: Option<Caching>,
}

impl Route for MemoryFileRoute {
	fn validate_requirements(&self, _params: &ParamsNames, _data: &Resources) {}

	fn path(&self) -> RoutePath {
		RoutePath {
			method: Some(Method::GET),
			path: self.uri.into(),
		}
	}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		_params: &'a PathParams,
		_data: &'a Resources,
	) -> PinnedFuture<'a, crate::Result<Response>> {
		let caching = self.caching.clone();

		PinnedFuture::new(async move {
			serve_memory_file(self.path, self.bytes, &req, caching)
				.map_err(Error::from_client_io)
		})
	}
}
