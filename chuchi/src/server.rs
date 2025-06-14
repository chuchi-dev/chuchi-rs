use crate::routing::{self, ServerShared};
use crate::util::PinnedFuture;
use crate::{ChuchiShared, Error, Result};

use std::convert::Infallible;
use std::net::SocketAddr;
use std::pin::Pin;
use std::result::Result as StdResult;
use std::sync::Arc;
use std::task::{Context, Poll};

use crate::body::BodyHttp;

use hyper::body::{Body, Frame, Incoming, SizeHint};
use hyper::service::Service;
use hyper::{Request, Response};

pub type HyperRequest = hyper::Request<HyperBody>;

use tokio::net::TcpListener;

// todo replace this function once hyper-util is ready
pub(crate) struct Server {
	listener: TcpListener,
	#[allow(dead_code)]
	shared: Arc<ServerShared>,
}

impl Server {
	pub(crate) async fn bind(
		addr: SocketAddr,
		shared: Arc<ServerShared>,
	) -> Result<Self> {
		let listener = TcpListener::bind(&addr)
			.await
			.map_err(Error::from_server_error)?;

		Ok(Self { listener, shared })
	}

	pub fn local_addr(&self) -> Result<SocketAddr> {
		self.listener.local_addr().map_err(Error::from_server_error)
	}

	#[cfg(any(feature = "http1", feature = "http2"))]
	pub async fn serve(self) -> Result<()> {
		use hyper_util::rt::{TokioExecutor, TokioIo};
		use hyper_util::server::conn::auto::Builder;

		loop {
			let (stream, address) = self
				.listener
				.accept()
				.await
				.map_err(Error::from_server_error)?;

			let io = TokioIo::new(stream);

			let service = ChuchiService {
				shared: self.shared.clone(),
				address,
			};

			tokio::task::spawn(async move {
				if let Err(err) = Builder::new(TokioExecutor::new())
					.serve_connection_with_upgrades(io, service)
					.await
				{
					tracing::error!(error = ?err, "Error serving connection: {err}");
				}
			});
		}
	}
}

pub struct ChuchiService {
	shared: Arc<ServerShared>,
	address: SocketAddr,
}

impl ChuchiService {
	/// Creates a new ChuchiService which can be passed to a hyper server.
	pub fn new(pit: ChuchiShared, address: SocketAddr) -> Self {
		Self {
			shared: pit.inner,
			address,
		}
	}
}

impl Service<Request<Incoming>> for ChuchiService {
	type Response = Response<BodyHttp>;
	type Error = Infallible;
	type Future = PinnedFuture<'static, StdResult<Self::Response, Self::Error>>;

	fn call(&self, req: Request<Incoming>) -> Self::Future {
		let wood = self.shared.clone();
		let address = self.address;
		PinnedFuture::new(async move {
			routing::route_hyper(&wood, req, address).await
		})
	}
}

#[derive(Debug)]
pub struct HyperBody {
	inner: InnerBody,
}

impl HyperBody {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn take(&mut self) -> Self {
		std::mem::take(self)
	}
}

#[derive(Debug)]
enum InnerBody {
	Empty,
	Incoming(Incoming),
}

impl Body for HyperBody {
	type Data = hyper::body::Bytes;
	type Error = hyper::Error;

	fn poll_frame(
		self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<StdResult<Frame<Self::Data>, Self::Error>>> {
		match &mut self.get_mut().inner {
			InnerBody::Empty => Poll::Ready(None),
			InnerBody::Incoming(inc) => Pin::new(inc).poll_frame(cx),
		}
	}

	fn is_end_stream(&self) -> bool {
		match &self.inner {
			InnerBody::Empty => true,
			InnerBody::Incoming(inc) => inc.is_end_stream(),
		}
	}

	fn size_hint(&self) -> SizeHint {
		match &self.inner {
			InnerBody::Empty => SizeHint::default(),
			InnerBody::Incoming(inc) => inc.size_hint(),
		}
	}
}

impl Default for HyperBody {
	fn default() -> Self {
		Self {
			inner: InnerBody::Empty,
		}
	}
}

impl From<Incoming> for HyperBody {
	fn from(inc: Incoming) -> Self {
		Self {
			inner: InnerBody::Incoming(inc),
		}
	}
}

impl From<HyperBody> for crate::Body {
	fn from(hyper_body: HyperBody) -> Self {
		match hyper_body.inner {
			InnerBody::Empty => Self::new(),
			InnerBody::Incoming(inc) => Self::from(inc),
		}
	}
}
