use std::net::SocketAddr;

use crate::util::PinnedFuture;
use crate::{Resources, Response};

pub use crate::server::{HyperBody, HyperRequest};

use super::{ParamsNames, PathParams, RoutePath};

/// A `RawRoute` is the more powerfull brother/sister to `Route`. It get's
/// executed before `Route`.
/// The `RawRoute` should only be needed if you implement something lower level
/// like websockets and need access to the underlying hyper types.
pub trait RawRoute: Send + Sync {
	// check if every data you expect is in Data
	fn validate_requirements(&self, _params: &ParamsNames, _data: &Resources) {}

	// get's only called once
	fn path(&self) -> RoutePath;

	fn call<'a>(
		&'a self,
		req: &'a mut HyperRequest,
		address: SocketAddr,
		params: &'a PathParams,
		resources: &'a Resources,
	) -> PinnedFuture<'a, Option<crate::Result<Response>>>;
}
