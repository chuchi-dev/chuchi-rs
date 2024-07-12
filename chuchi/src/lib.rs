#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

#[macro_use]
mod macros;

pub mod resources;
use resources::Resources;

pub mod state;

pub mod routes;
use routes::{Catcher, ParamsNames, RawRoute, Route, Routes};

#[macro_use]
pub mod util;

pub mod into;
use into::IntoRoute;

pub mod error;
pub use error::{Error, Result};

pub mod extractor;
pub use extractor::Res;

mod server;
use server::Server;

mod routing;
use routing::{RequestConfigs, ServerShared};
use tracing::info;

#[cfg(feature = "fs")]
#[cfg_attr(docsrs, doc(cfg(feature = "fs")))]
pub mod fs;

#[cfg(feature = "json")]
#[cfg_attr(docsrs, doc(cfg(feature = "json")))]
pub mod json;

#[cfg(feature = "ws")]
#[cfg_attr(docsrs, doc(cfg(feature = "ws")))]
pub mod ws;

#[cfg(feature = "graphql")]
#[cfg_attr(docsrs, doc(cfg(feature = "graphql")))]
pub mod graphql;

#[cfg(feature = "api")]
#[cfg_attr(docsrs, doc(cfg(feature = "api")))]
pub mod api;

#[cfg(feature = "json")]
#[doc(hidden)]
pub use serde_json;

pub mod service {
	pub use crate::server::ChuchiService;
}

use std::any::Any;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::ToSocketAddrs;
use tokio::task::JoinHandle;

pub use chuchi_core::{
	body, header, request, response, Body, Request, Response,
};

pub use chuchi_codegen::*;

/// Prepares a server.
pub async fn build(addr: impl ToSocketAddrs) -> Result<Chuchi> {
	Chuchi::new(addr).await
}

/// `FireBuilder` gathers all materials needed to light a fire (start a server).
pub struct Chuchi {
	addr: SocketAddr,
	resources: Resources,
	routes: Routes,
	configs: RequestConfigs,
}

impl Chuchi {
	pub(crate) async fn new<A>(addr: A) -> Result<Self>
	where
		A: ToSocketAddrs,
	{
		let addr = tokio::net::lookup_host(addr)
			.await
			.map_err(Error::from_server_error)?
			.next()
			.unwrap();
		Ok(Self {
			addr,
			resources: Resources::new(),
			routes: Routes::new(),
			configs: RequestConfigs::new(),
		})
	}

	/// Returns a reference to the current data.
	pub fn resources(&self) -> &Resources {
		&self.resources
	}

	pub fn add_resource<R>(&mut self, resource: R)
	where
		R: Any + Send + Sync,
	{
		self.resources.insert(resource);
	}

	/// Adds a `RawRoute` to the fire.
	pub fn add_raw_route<R>(&mut self, route: R)
	where
		R: RawRoute + 'static,
	{
		let path = route.path();
		let names = ParamsNames::parse(&path.path);
		route.validate_requirements(&names, &self.resources);
		self.routes.push_raw(path, route)
	}

	/// Adds a `Route` to the fire.
	pub fn add_route<R>(&mut self, route: R)
	where
		R: IntoRoute + 'static,
	{
		let route = route.into_route();
		let path = route.path();
		let names = ParamsNames::parse(&path.path);
		route.validate_requirements(&names, &self.resources);
		self.routes.push(path, route)
	}

	/// Adds a `Catcher` to the fire.
	pub fn add_catcher<C>(&mut self, catcher: C)
	where
		C: Catcher + 'static,
	{
		catcher.validate_data(&self.resources);
		self.routes.push_catcher(catcher)
	}

	/// Sets the request size limit. The default is 4 kilobytes.
	///
	/// This can be changed in every Route.
	///
	/// ## Panics
	/// If the size is zero.
	pub fn request_size_limit(&mut self, size_limit: usize) {
		self.configs.size_limit(size_limit)
	}

	/// Sets the request timeout. The default is 60 seconds.
	///
	/// This can be changed in every Route.
	pub fn request_timeout(&mut self, timeout: Duration) {
		self.configs.timeout(timeout)
	}

	/// Binds to the address and prepares to serve requests.
	///
	/// You need to call ignite on the `Fire` so that it starts handling
	/// requests.
	pub async fn build(self) -> Result<ChuchiServer> {
		let wood = Arc::new(ServerShared::new(
			self.resources,
			self.routes,
			self.configs,
		));

		let server = Server::bind(self.addr, wood.clone()).await?;

		Ok(ChuchiServer {
			shared: wood,
			server,
		})
	}

	/// Ignites the fire, which starts the server.
	///
	/// ## Note
	/// Under normal conditions this function should run forever.
	pub async fn run(self) -> Result<()> {
		let server = self.build().await?;
		server.run().await
	}

	/// Ignites the fire, and spawns it on a new tokio task.
	///
	/// ## Note
	/// Under normal conditions this task should run forever.
	pub fn run_task(self) -> JoinHandle<()> {
		tokio::spawn(async move { self.run().await.unwrap() })
	}

	/// Creates a FirePit without starting the server.
	///
	/// In most cases you should use `build` and then call `pit` on the `Fire`.
	///
	/// Creating a `FirePit` might be useful for testing or if you want to
	/// manually create a server.
	pub fn into_shared(self) -> ChuchiShared {
		let wood = Arc::new(ServerShared::new(
			self.resources,
			self.routes,
			self.configs,
		));

		ChuchiShared { inner: wood }
	}
}

/// A Fire that is ready to be ignited.
pub struct ChuchiServer {
	shared: Arc<ServerShared>,
	server: Server,
}

impl ChuchiServer {
	pub fn local_addr(&self) -> Option<SocketAddr> {
		self.server.local_addr().ok()
	}

	pub fn shared(&self) -> ChuchiShared {
		ChuchiShared {
			inner: self.shared.clone(),
		}
	}

	pub async fn run(self) -> Result<()> {
		info!("Running server on addr: {}", self.local_addr().unwrap());

		self.server.serve().await
	}
}

#[derive(Clone)]
pub struct ChuchiShared {
	inner: Arc<ServerShared>,
}

impl ChuchiShared {
	pub fn data(&self) -> &Resources {
		self.inner.data()
	}

	/// Routes the request to normal routes and returns their result.
	///
	/// Useful for tests and niche applications.
	///
	/// Returns None if no route was found matching the request.
	pub async fn route(&self, req: &mut Request) -> Option<Result<Response>> {
		routing::route(&self.inner, req).await
	}
}
