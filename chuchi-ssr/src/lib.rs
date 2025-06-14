mod pool;
mod runtime;

use pool::PoolHandle;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::{fmt, io};

use serde::{Deserialize, Serialize};

use chuchi::header::{Mime, StatusCode};
use chuchi::{ChuchiShared, Request, Resource, Response};

use aho_corasick::AhoCorasick;

#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
	Panicked,
	Io(io::Error),
	Other(String),
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

impl std::error::Error for Error {}

// /// Request to ssr
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SsrRequest {
	// the ip address from the requestor
	pub address: String,
	/// GET, POST
	pub method: String,
	pub uri: String,
	pub headers: HashMap<String, String>,
	pub body: String,
}

// // pub struct Cache {
// // 	pub
// // }

// /// Response from ssr
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SsrResponse {
	// pub cache: Option<Cache>
	pub status: u16,
	// those fields are replace in the index.html <!--ssr-field-->
	// where `field` is the key
	pub fields: HashMap<String, String>,
}

#[derive(Clone, Resource)]
pub struct JsServer {
	pool: PoolHandle,
	index: Arc<String>, // tx: mpsc::Sender<(SsrRequest, oneshot::Receiver<SsrResponse>)>
}

impl JsServer {
	/// this module should export { render } (which takes a SsrRequest)
	/// and should return a SsrResponse
	///
	/// threads: how many concurrent js instances can exist
	///
	/// ## Panics if opts cannot be serialized to serde_json::Value
	pub fn new<T: Serialize>(
		base_dir: impl Into<PathBuf>,
		index_html: impl Into<String>,
		opts: T,
		max_threads: usize,
	) -> Self {
		let pool = PoolHandle::new(
			base_dir.into(),
			max_threads,
			serde_json::to_value(opts).unwrap(),
		);

		// runtime ready
		Self {
			pool,
			index: Arc::new(index_html.into()),
		}
	}

	/// Call this if you wan't to route requests internally without going over
	/// the http stack
	///
	/// You need to pass a ChuchiShared
	pub async fn route_internally(&self, shared: ChuchiShared) {
		self.pool.send_pit(shared).await;
	}

	pub async fn request(&self, req: &mut Request) -> Result<Response, Error> {
		let body = req.take_body().into_string().await.map_err(Error::Io)?;

		let header = &req.header;
		let method = header.method.to_string().to_uppercase();

		let headers = header.values.clone().into_inner();
		let headers: HashMap<_, _> = headers
			.iter()
			.filter_map(|(key, val)| {
				val.to_str().ok().map(|s| (key.to_string(), s.to_string()))
			})
			.collect();

		let uri = if let Some(query) = header.uri.query() {
			format!("{}?{}", header.uri.path(), query)
		} else {
			header.uri.path().to_string()
		};

		let ssr_request = SsrRequest {
			address: header.address.to_string(),
			method,
			uri,
			headers,
			body,
		};

		let resp = self
			.pool
			.send_req(ssr_request)
			.await
			.ok_or(Error::Panicked)?;

		let ac = AhoCorasick::new(
			resp.fields.keys().map(|k| format!("<!--ssr-{k}-->")),
		)
		.expect("aho corasick limit exceeded");

		let values: Vec<_> = resp.fields.values().collect();

		let index = ac.replace_all(&self.index, &values);

		let resp = Response::builder()
			.status_code(
				StatusCode::from_u16(resp.status)
					.map_err(|e| Error::Other(e.to_string()))?,
			)
			.content_type(Mime::HTML)
			.body(index)
			.build();

		Ok(resp)
	}
}
