use crate::routes::Routes;
use crate::server::HyperBody;
use crate::util::{
	convert_fire_resp_to_hyper_resp, convert_hyper_req_to_fire_req,
};
use crate::{Error, Request, Resources};

use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use hyper::body::Incoming;
use tracing::{error, info, info_span, warn, Instrument};

use crate::body::BodyHttp;
use crate::header::StatusCode;
use crate::Response;

const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
// same as page size
const DEFAULT_REQUEST_SIZE_LIMIT: usize = 4096; // 4kb

#[derive(Debug)]
pub(crate) struct RequestConfigs {
	pub timeout: Duration,
	// in bytes
	pub size_limit: usize,
}

impl RequestConfigs {
	pub fn new() -> Self {
		Self {
			timeout: DEFAULT_REQUEST_TIMEOUT,
			size_limit: DEFAULT_REQUEST_SIZE_LIMIT,
		}
	}

	pub fn timeout(&mut self, timeout: Duration) {
		self.timeout = timeout;
	}

	/// ## Panics
	/// if is 0
	pub fn size_limit(&mut self, size_limit: usize) {
		assert!(size_limit > 0, "size limit needs to be bigger than zero");
		self.size_limit = size_limit;
	}
}

// IncredientsForAFire
pub(crate) struct ServerShared {
	data: Resources,
	routes: Routes,
	configs: RequestConfigs,
}

impl ServerShared {
	pub fn new(
		data: Resources,
		routes: Routes,
		configs: RequestConfigs,
	) -> Self {
		Self {
			data,
			routes,
			configs,
		}
	}

	pub fn routes(&self) -> &Routes {
		&self.routes
	}

	pub fn data(&self) -> &Resources {
		&self.data
	}

	pub fn configs(&self) -> &RequestConfigs {
		&self.configs
	}
}

#[cfg(feature = "sentry")]
pub(crate) async fn route_hyper(
	wood: &ServerShared,
	hyper_req: hyper::Request<Incoming>,
	address: SocketAddr,
) -> Result<hyper::Response<BodyHttp>, Infallible> {
	use std::sync::Arc;

	use sentry_core::{Hub, Scope, SentryFutureExt};

	let hub = Hub::new(Hub::current().client(), Arc::new(Scope::default()));

	route_hyper_with_span(wood, hyper_req, address)
		.bind_hub(hub)
		.await
}

#[cfg(not(feature = "sentry"))]
pub(crate) async fn route_hyper(
	wood: &ServerShared,
	hyper_req: hyper::Request<Incoming>,
	address: SocketAddr,
) -> Result<hyper::Response<BodyHttp>, Infallible> {
	route_hyper_with_span(wood, hyper_req, address).await
}

async fn route_hyper_with_span(
	wood: &ServerShared,
	hyper_req: hyper::Request<Incoming>,
	address: SocketAddr,
) -> Result<hyper::Response<BodyHttp>, Infallible> {
	let span = info_span!(
		"req",
		method = ?hyper_req.method(),
		uri = ?hyper_req.uri(),
	);

	route_hyper_inner(wood, hyper_req, address)
		.instrument(span)
		.await
}

async fn route_hyper_inner(
	wood: &ServerShared,
	hyper_req: hyper::Request<Incoming>,
	address: SocketAddr,
) -> Result<hyper::Response<BodyHttp>, Infallible> {
	let method = hyper_req.method().clone();
	let uri = hyper_req.uri().clone();
	info!(?method, ?uri, "req");
	let resp = route_hyper_req(wood, hyper_req, address).await;
	let status_code = resp.header().status_code;

	if status_code.is_server_error() {
		error!(?status_code, "{method} {uri} | {status_code}");
	} else if status_code.is_client_error() {
		warn!(?status_code, "{method} {uri} | {status_code}");
	} else {
		info!(?status_code, "{method} {uri} | {status_code}");
	}

	let hyper_resp = convert_fire_resp_to_hyper_resp(resp);

	Ok(hyper_resp)
}

async fn route_hyper_req(
	wood: &ServerShared,
	hyper_req: hyper::Request<Incoming>,
	address: SocketAddr,
) -> Response {
	let mut hyper_req = hyper_req.map(HyperBody::from);

	// route raw_routes
	// response is Option<Response>
	let resp = if let Some((route, params)) = wood
		.routes()
		.route_raw(hyper_req.method(), hyper_req.uri().path())
	{
		let res = route
			.call(&mut hyper_req, address, &params, wood.data())
			.await;
		match res {
			Some(Ok(res)) => Some(res),
			Some(Err(e)) => {
				error!("raw_route error: {}", e);
				Some(e.status_code().into())
			}
			None => None,
		}
	} else {
		None
	};

	let req = convert_hyper_req_to_fire_req(hyper_req, address, wood.configs());
	let mut req = match req {
		Ok(r) => r,
		Err(e) => {
			if let Some(resp) = resp {
				return resp;
			}
			error!("Could not parse the hyper request: {e}");
			return StatusCode::BAD_REQUEST.into();
		}
	};

	// normal route
	let mut resp = if let Some(r) = resp {
		r
	} else {
		match route(wood, &mut req).await {
			Some(Ok(resp)) => resp,
			Some(Err(error)) => {
				error!(?error, "route error");
				error.status_code().into()
			}
			None => StatusCode::NOT_FOUND.into(),
		}
	};

	// APPLY OVERRIDES

	// check with catcher
	for catcher in wood.routes().catchers() {
		if !catcher.check(req.header(), resp.header()) {
			continue;
		}

		if let Err(e) = catcher.call(&mut req, &mut resp, wood.data()).await {
			resp = e.status_code().into();
		}
	}

	resp
}

pub(crate) async fn route(
	wood: &ServerShared,
	req: &mut Request,
) -> Option<Result<Response, Error>> {
	// first response
	let (route, params) = wood
		.routes()
		.route(&req.header().method, req.header().uri().path())?;

	let r = route.call(req, &params, wood.data()).await;

	Some(r)
}
