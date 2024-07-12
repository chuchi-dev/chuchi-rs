use super::{with_file, with_partial_file};
use super::{Caching, IntoPathBuf, Range};

use crate::error::ClientErrorKind;
use crate::header::{Method, StatusCode};
use crate::into::{IntoResponse, IntoRoute};
use crate::routes::{ParamsNames, PathParams, Route, RoutePath};
use crate::util::PinnedFuture;
use crate::{Error, Request, Resources, Response};

use std::borrow::Cow;
use std::io;
use std::path::Path;
use std::time::Duration;

/// returns io::Error not found if the path is a directory
pub async fn serve_file(
	path: impl AsRef<Path>,
	req: &Request,
	caching: Option<Caching>,
) -> io::Result<Response> {
	// check caching and if the etag matches return NOT_MODIFIED
	if matches!(&caching, Some(c) if c.if_none_match(req.header())) {
		return Ok(caching.unwrap().into_response());
	}

	let range = Range::parse(req.header());

	let mut res = match range {
		Some(range) => with_partial_file(path, range).await?.into_response(),
		None => with_file(path).await?.into_response(),
	};

	// set etag
	if let Some(caching) = caching {
		if matches!(
			res.header.status_code,
			StatusCode::OK | StatusCode::NOT_FOUND
		) {
			caching.complete_header(&mut res.header);
		}
	}

	Ok(res)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CachingBuilder {
	None,
	Default,
	MaxAge(Duration),
}

impl From<CachingBuilder> for Option<Caching> {
	fn from(b: CachingBuilder) -> Self {
		match b {
			CachingBuilder::None => None,
			CachingBuilder::Default => Some(Caching::default()),
			CachingBuilder::MaxAge(age) => Some(Caching::new(age)),
		}
	}
}

/// Static get handler which servers files from a directory.
///
/// ## Example
/// ```
/// use chuchi::fs::StaticFiles;
///
/// const FILES: StaticFiles = StaticFiles::new("/files", "./www/");
///
/// #[tokio::main]
/// async fn main() {
/// 	let mut server = chuchi::build("127.0.0.1:0").await.unwrap();
/// 	server.add_route(FILES);
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticFiles {
	uri: &'static str,
	path: &'static str,
	caching: CachingBuilder,
}

impl StaticFiles {
	/// Creates a `StaticFiles` with Default caching settings
	pub const fn new(uri: &'static str, path: &'static str) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::Default,
		}
	}

	pub const fn no_cache(uri: &'static str, path: &'static str) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::None,
		}
	}

	pub const fn cache_with_age(
		uri: &'static str,
		path: &'static str,
		max_age: Duration,
	) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::MaxAge(max_age),
		}
	}
}

impl IntoRoute for StaticFiles {
	type IntoRoute = StaticFilesRoute;

	fn into_route(self) -> StaticFilesRoute {
		StaticFilesRoute {
			uri: self.uri.into(),
			path: self.path.into(),
			caching: self.caching.into(),
		}
	}
}

/// Static get handler which servers files from a directory.
///
/// ## Example
/// ```
/// use chuchi::fs::StaticFilesOwned;
///
/// #[tokio::main]
/// async fn main() {
/// 	let mut server = chuchi::build("127.0.0.1:0").await.unwrap();
/// 	server.add_route(
/// 		StaticFilesOwned::new("/files".into(), "./www/".into())
/// 	);
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticFilesOwned {
	uri: String,
	path: String,
	caching: CachingBuilder,
}

impl StaticFilesOwned {
	/// Creates a `StaticFiles` with Default caching settings
	pub fn new(uri: String, path: String) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::Default,
		}
	}

	pub fn no_cache(uri: String, path: String) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::None,
		}
	}

	pub fn cache_with_age(
		uri: String,
		path: String,
		max_age: Duration,
	) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::MaxAge(max_age),
		}
	}
}

impl IntoRoute for StaticFilesOwned {
	type IntoRoute = StaticFilesRoute;

	fn into_route(self) -> StaticFilesRoute {
		StaticFilesRoute {
			uri: self.uri.trim_end_matches('/').to_string().into(),
			path: self.path.into(),
			caching: self.caching.into(),
		}
	}
}

#[doc(hidden)]
pub struct StaticFilesRoute {
	// should not end with a trailing slash
	uri: Cow<'static, str>,
	path: Cow<'static, str>,
	caching: Option<Caching>,
}

impl Route for StaticFilesRoute {
	fn validate_requirements(&self, _params: &ParamsNames, _data: &Resources) {}

	fn path(&self) -> RoutePath {
		RoutePath {
			method: Some(Method::GET),
			path: format!("{}/{{*rem}}", self.uri).into(),
		}
	}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		_params: &'a PathParams,
		_: &'a Resources,
	) -> PinnedFuture<'a, crate::Result<Response>> {
		let uri = &self.uri;
		let caching = self.caching.clone();

		PinnedFuture::new(async move {
			let res_path_buf =
				req.header().uri().path()[uri.len()..].into_path_buf();

			// validate path buf
			// if path is a directory serve_file will return NotFound
			let path_buf = res_path_buf
				.map_err(|e| Error::new(ClientErrorKind::BadRequest, e))?;

			// build full pathbuf
			let path_buf = Path::new(&*self.path).join(path_buf);

			serve_file(path_buf, &req, caching)
				.await
				.map_err(Error::from_client_io)
		})
	}
}

/// Static get handler which servers/returns a file.
///
/// ## Example
/// ```
/// use chuchi::fs::StaticFile;
///
/// const INDEX: StaticFile = StaticFile::new("/", "./www/index.html");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticFile {
	uri: &'static str,
	path: &'static str,
	caching: CachingBuilder,
}

impl StaticFile {
	/// Creates a `StaticFile` with Default caching settings
	pub const fn new(uri: &'static str, path: &'static str) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::Default,
		}
	}

	pub const fn no_cache(uri: &'static str, path: &'static str) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::None,
		}
	}

	pub const fn cache_with_age(
		uri: &'static str,
		path: &'static str,
		max_age: Duration,
	) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::MaxAge(max_age),
		}
	}
}

impl IntoRoute for StaticFile {
	type IntoRoute = StaticFileRoute;

	fn into_route(self) -> StaticFileRoute {
		StaticFileRoute {
			uri: self.uri.trim_end_matches('/').into(),
			path: self.path.into(),
			caching: self.caching.into(),
		}
	}
}

/// Static get handler which servers/returns a file.
///
/// ## Example
/// ```
/// use chuchi::fs::StaticFileOwned;
///
/// #[tokio::main]
/// async fn main() {
/// 	let mut server = chuchi::build("127.0.0.1:0").await.unwrap();
/// 	server.add_route(
/// 		StaticFileOwned::new("/files/file".into(), "./www/file".into())
/// 	);
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaticFileOwned {
	uri: String,
	path: String,
	caching: CachingBuilder,
}

impl StaticFileOwned {
	/// Creates a `StaticFile` with Default caching settings
	pub const fn new(uri: String, path: String) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::Default,
		}
	}

	pub const fn no_cache(uri: String, path: String) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::None,
		}
	}

	pub const fn cache_with_age(
		uri: String,
		path: String,
		max_age: Duration,
	) -> Self {
		Self {
			uri,
			path,
			caching: CachingBuilder::MaxAge(max_age),
		}
	}
}

impl IntoRoute for StaticFileOwned {
	type IntoRoute = StaticFileRoute;

	fn into_route(self) -> StaticFileRoute {
		StaticFileRoute {
			uri: self.uri.into(),
			path: self.path.into(),
			caching: self.caching.into(),
		}
	}
}

#[doc(hidden)]
pub struct StaticFileRoute {
	uri: Cow<'static, str>,
	path: Cow<'static, str>,
	caching: Option<Caching>,
}

impl Route for StaticFileRoute {
	fn validate_requirements(&self, _params: &ParamsNames, _data: &Resources) {}

	fn path(&self) -> RoutePath {
		RoutePath {
			method: Some(Method::GET),
			path: self.uri.clone(),
		}
	}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		_params: &'a PathParams,
		_data: &'a Resources,
	) -> PinnedFuture<'a, crate::Result<Response>> {
		PinnedFuture::new(async move {
			serve_file(&*self.path, &req, self.caching.clone())
				.await
				.map_err(Error::from_client_io)
		})
	}
}
