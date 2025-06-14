// Todo check if we should take a snapshot
use crate::{SsrRequest, SsrResponse};

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use deno_core::{
	ascii_str_include, extension, op2, ModuleLoadResponse, ModuleSourceCode,
	RequestedModuleType,
};
use serde::{Deserialize, Serialize};

use deno_core::{
	anyhow, error::generic_error, resolve_import, JsRuntime, ModuleLoader,
	ModuleSource, ModuleSpecifier, ModuleType, OpState, ResolutionKind,
	RuntimeOptions,
};

use tokio::fs;
use tokio::sync::oneshot;

use chuchi::header::values::HeaderName;
use chuchi::header::{HeaderValues, Method, RequestHeader, StatusCode};
use chuchi::{ChuchiShared, Request};

use serde_json::Value;

use tracing::{trace, warn};

struct StaticModuleLoader {
	root: PathBuf,
}

impl ModuleLoader for StaticModuleLoader {
	fn resolve(
		&self,
		specifier: &str,
		referrer: &str,
		_kind: ResolutionKind,
	) -> Result<ModuleSpecifier, anyhow::Error> {
		trace!("resolve specifier {specifier:?} with referrer {referrer:?}");

		Ok(resolve_import(specifier, referrer)?)
	}

	fn load(
		&self,
		specifier: &ModuleSpecifier,
		_maybe_referrer: Option<&ModuleSpecifier>,
		_is_dyn_import: bool,
		_requested_module_type: RequestedModuleType,
	) -> ModuleLoadResponse {
		let specifier = specifier.clone();

		let file_path = specifier.to_file_path().map_err(|_| {
			generic_error(format!(
				"Provided module specifier \"{specifier}\" is not a file \
					URL."
			))
		});

		let path = file_path.map(|p| {
			// because this was parsed from a &str it will always be valid
			// utf8
			// let's remove the trailing slash
			let path = p.to_str().and_then(|s| s.get(1..)).unwrap_or("");
			self.root.join(path)
		});

		ModuleLoadResponse::Async(Box::pin(async move {
			let path = path?;

			trace!("reading file {path:?}");

			let is_json = path
				.extension()
				.and_then(|p| p.to_str())
				.map(|ext| ext == "json")
				.unwrap_or(false);

			let module_type = if is_json {
				ModuleType::Json
			} else {
				ModuleType::JavaScript
			};

			let code = fs::read_to_string(path).await?;

			Ok(ModuleSource::new(
				module_type,
				ModuleSourceCode::String(code.into()),
				&specifier,
				None,
			))
		}))
	}
}

pub struct Runtime {
	runtime: JsRuntime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Options(Value);

struct TimersAllowed;

impl deno_web::TimersPermission for TimersAllowed {
	fn allow_hrtime(&mut self) -> bool {
		true
	}
}

extension!(
	chuchi_ssr,
	deps = [deno_webidl, deno_url, deno_web, deno_crypto],
	ops = [
		op_tracing_trace, op_tracing_debug, op_tracing_info, op_tracing_warn, op_tracing_error,
		op_get_options, op_next_request, op_send_response, op_fetch
	],
	esm_entry_point = "ext:chuchi_ssr/ext_entry.js",
	esm = [dir "js", "ext_entry.js"]
);

impl Runtime {
	/// needs to be run in a current_thread tokio runtime
	pub(crate) async fn new(
		base_dir: PathBuf,
		shared: Option<ChuchiShared>,
		rx: RequestReceiver,
		opts: Value,
	) -> Self {
		let mut runtime = JsRuntime::new(RuntimeOptions {
			extensions: vec![
				deno_webidl::deno_webidl::init_ops_and_esm(),
				deno_url::deno_url::init_ops_and_esm(),
				deno_console::deno_console::init_ops_and_esm(),
				deno_web::deno_web::init_ops_and_esm::<TimersAllowed>(
					std::sync::Arc::new(deno_web::BlobStore::default()),
					None,
				),
				deno_crypto::deno_crypto::init_ops_and_esm(None),
				chuchi_ssr::init_ops_and_esm(),
			],
			module_loader: Some(Rc::new(StaticModuleLoader { root: base_dir })),
			..Default::default()
		});

		{
			let state = runtime.op_state();
			let mut state = state.borrow_mut();
			state.put(rx);
			if let Some(pit) = shared {
				state.put(pit);
			}
			state.put(IdCounter::new());
			state.put(ResponseSenders::new());
			state.put(Options(opts));
		}

		let mod_id = runtime
			.load_main_es_module_from_code(
				&"file:///__chuchi_ssr_entry.js".parse().unwrap(),
				ascii_str_include!("../js/main.js"),
			)
			.await
			.expect("failed to load main module");

		let res = runtime.mod_evaluate(mod_id);
		runtime
			.run_event_loop(Default::default())
			.await
			.expect("failed to run event loop");
		trace!("event loop ran");
		res.await.expect("main.js failed");

		// runtime ready
		Self { runtime }
	}

	/// ## Panics
	/// if the runtime failes.
	pub async fn run(&mut self) {
		self.runtime
			.run_event_loop(Default::default())
			.await
			.unwrap();
	}

	pub fn remove_request_receiver(&mut self) {
		self.runtime
			.op_state()
			.borrow_mut()
			.take::<RequestReceiver>();
	}
}

// registering ops
#[derive(Debug, Clone)]
pub(crate) struct RequestReceiver(
	pub flume::Receiver<(SsrRequest, oneshot::Sender<SsrResponse>)>,
);

struct ResponseSenders {
	inner: HashMap<usize, oneshot::Sender<SsrResponse>>,
}

impl ResponseSenders {
	pub fn new() -> Self {
		Self {
			inner: HashMap::new(),
		}
	}

	pub fn insert(&mut self, id: usize, sender: oneshot::Sender<SsrResponse>) {
		self.inner.insert(id, sender);
	}

	pub fn take(&mut self, id: usize) -> Option<oneshot::Sender<SsrResponse>> {
		self.inner.remove(&id)
	}
}

struct IdCounter {
	inner: usize,
}

impl IdCounter {
	pub fn new() -> Self {
		Self { inner: 0 }
	}

	pub fn next(&mut self) -> usize {
		let id = self.inner;
		self.inner = self.inner.wrapping_add(1);
		id
	}
}

#[op2(fast)]
fn op_tracing_trace(#[string] msg: Cow<str>) {
	tracing::trace!("js: {msg}");
}

#[op2(fast)]
fn op_tracing_debug(#[string] msg: Cow<str>) {
	tracing::debug!("js: {msg}");
}

#[op2(fast)]
fn op_tracing_info(#[string] msg: Cow<str>) {
	tracing::info!("js: {msg}");
}

#[op2(fast)]
fn op_tracing_warn(#[string] msg: Cow<str>) {
	tracing::warn!("js: {msg}");
}

#[op2(fast)]
fn op_tracing_error(#[string] msg: Cow<str>) {
	tracing::error!("js: {msg}");
}

#[op2]
#[serde]
fn op_get_options(state: Rc<RefCell<OpState>>) -> serde_json::Value {
	state.borrow().borrow::<Options>().clone().0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpSsrRequest {
	pub id: usize,
	pub req: SsrRequest,
}

/// if None get's returned this means you should stop the request loop
#[op2(async)]
#[serde]
async fn op_next_request(state: Rc<RefCell<OpState>>) -> Option<OpSsrRequest> {
	let (id, rx) = {
		let mut state = state.borrow_mut();
		// try_borrow since if we should close the connection we remove the
		// RequestReceiver
		let recv = state.try_borrow::<RequestReceiver>()?.clone();
		let id = state.borrow_mut::<IdCounter>().next();

		(id, recv)
	};
	let (req, tx) = rx.0.recv_async().await.ok()?;

	trace!("op next request {id} {req:?}");

	state
		.borrow_mut()
		.borrow_mut::<ResponseSenders>()
		.insert(id, tx);

	Some(OpSsrRequest { id, req })
}

#[op2]
fn op_send_response(
	state: Rc<RefCell<OpState>>,
	#[bigint] id: usize,
	#[serde] resp: SsrResponse,
) -> bool {
	trace!("op send response {id}");
	let mut state = state.borrow_mut();

	let Some(tx) = state.borrow_mut::<ResponseSenders>().take(id) else {
		return false;
	};

	tx.send(resp).is_ok()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FetchRequest {
	url: String,
	method: String,
	headers: HashMap<String, String>,
	body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FetchResponse {
	status: u16,
	headers: HashMap<String, String>,
	body: String,
}

/// This will not check for a length and might use abitrary amounts of memory
/// todo fix it
#[op2(async)]
#[serde]
async fn op_fetch(
	state: Rc<RefCell<OpState>>,
	#[serde] req: FetchRequest,
) -> Result<FetchResponse, anyhow::Error> {
	trace!("op fetch {req:?}");

	let mut values = HeaderValues::new();
	for (key, val) in req.headers {
		let key: HeaderName = key.parse().unwrap();
		let _ = values.try_insert(key, val);
	}

	let method: Method =
		req.method.to_uppercase().parse().unwrap_or(Method::GET);

	if !req.url.starts_with('/') {
		let resp = reqwest::Client::new()
			.request(method, reqwest::Url::parse(&req.url)?)
			.headers(values.into_inner())
			.body(req.body)
			.send()
			.await?;

		let status = resp.status().as_u16();
		let headers: HashMap<_, _> = resp
			.headers()
			.iter()
			.filter_map(|(key, val)| {
				val.to_str().ok().map(|s| (key.to_string(), s.to_string()))
			})
			.collect();

		return Ok(FetchResponse {
			status,
			headers,
			body: resp.text().await?,
		});
	}

	let header = RequestHeader {
		address: ([0, 0, 0, 0], 0).into(),
		method,
		uri: req.url.parse()?,
		values,
	};

	let mut req = Request::new(header, req.body.into());

	trace!("op internal fetch {req:?}");

	let pit = {
		let state = state.borrow();
		state.try_borrow::<ChuchiShared>().cloned()
	};

	if pit.is_none() {
		warn!("no pit");
	}

	let res = match pit {
		Some(pit) => match pit.route(&mut req).await {
			Some(res) => res?,
			None => StatusCode::NOT_FOUND.into(),
		},
		None => StatusCode::NOT_FOUND.into(),
	};

	trace!("op fetch resp {res:?}");

	let mut headers: HashMap<_, _> = res
		.header
		.values
		.into_inner()
		.iter()
		.filter_map(|(key, val)| {
			val.to_str().ok().map(|s| (key.to_string(), s.to_string()))
		})
		.collect();

	headers.insert("content-type".into(), res.header.content_type.to_string());

	Ok(FetchResponse {
		status: res.header.status_code.as_u16(),
		headers,
		body: res.body.into_string().await.unwrap_or_else(|_| "".into()),
	})
}
