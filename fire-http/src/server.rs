use crate::{Result, Error, FirePit};
use crate::util::PinnedFuture;
use crate::fire::{self, Wood};

use std::future;
use std::sync::Arc;
use std::net::SocketAddr;
use std::task::{Poll, Context};
use std::convert::Infallible;
use std::result::Result as StdResult;

use types::body::BodyHttp;

use hyper::Response;
use hyper::service::Service;
use hyper::server::conn::{AddrIncoming, AddrStream};


pub type HyperRequest = hyper::Request<hyper::Body>;

// todo replace this function once hyper-util is ready
pub(crate) struct Server {
	listener: hyper::Server<AddrIncoming, MakeFireService>
}

impl Server {
	pub(crate) async fn bind(
		addr: SocketAddr,
		wood: Arc<Wood>
	) -> Result<Self> {
		let listener = hyper::Server::try_bind(&addr)
			.map_err(Error::from_server_error)?
			.serve(MakeFireService { wood });
		Ok(Self { listener })
	}

	pub fn local_addr(&self) -> Result<SocketAddr> {
		Ok(self.listener.local_addr())
	}

	pub async fn serve(self) -> Result<()> {
		self.listener.await.map_err(Error::from_server_error)
	}
}

/// A `tower::Service` to be used with the `hyper::Server`
pub struct MakeFireService {
	pub(crate) wood: Arc<Wood>
}

impl MakeFireService {
	pub fn pit(&self) -> FirePit {
		FirePit { wood: self.wood.clone() }
	}

	pub fn make(&self, address: SocketAddr) -> FireService {
		FireService { wood: self.wood.clone(), address }
	}
}

impl<'a> Service<&'a AddrStream> for MakeFireService {
	type Response = FireService;
	type Error = Infallible;
	type Future = future::Ready<StdResult<FireService, Infallible>>;

	fn poll_ready(
		&mut self,
		_: &mut Context
	) -> Poll<StdResult<(), Infallible>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, addr_stream: &'a AddrStream) -> Self::Future {
		let address = addr_stream.remote_addr();
		future::ready(Ok(FireService {
			wood: self.wood.clone(),
			address
		}))
	}
}

pub struct FireService {
	wood: Arc<Wood>,
	address: SocketAddr
}

impl Service<HyperRequest> for FireService {
	type Response = Response<BodyHttp>;
	type Error = Infallible;
	type Future = PinnedFuture<'static, StdResult<Self::Response, Self::Error>>;

	fn poll_ready(
		&mut self,
		_: &mut Context
	) -> Poll<StdResult<(), Infallible>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: HyperRequest) -> Self::Future {
		let wood = self.wood.clone();
		let address = self.address;
		PinnedFuture::new(async move {
			fire::route_hyper(&wood, req, address).await
		})
	}
}