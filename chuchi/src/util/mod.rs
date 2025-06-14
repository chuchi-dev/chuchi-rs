use crate::header::{ContentType, CONTENT_TYPE};
use crate::routing::RequestConfigs;
use crate::server::HyperRequest;
use crate::{Body, Request, Response};

use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use tracing::error;

mod header;
use header::convert_hyper_parts_to_chuchi_header;
pub use header::convert_hyper_req_to_chuchi_header;
pub(crate) use header::HeaderError;

use crate::header::RequestHeader;

/// Deprecated: Use `convert_hyper_req_to_chuchi_header` instead
#[deprecated = "Use `convert_hyper_req_to_chuchi_header` instead"]
pub fn convert_hyper_req_to_fire_header<B>(
	req: &hyper::Request<B>,
	address: SocketAddr,
) -> Result<RequestHeader, HeaderError> {
	convert_hyper_req_to_chuchi_header(req, address)
}

use crate::body::BodyHttp;

pub struct PinnedFuture<'a, O> {
	inner: Pin<Box<dyn Future<Output = O> + Send + 'a>>,
}

impl<'a, O> PinnedFuture<'a, O> {
	pub fn new<F>(future: F) -> Self
	where
		F: Future<Output = O> + Send + 'a,
	{
		Self {
			inner: Box::pin(future),
		}
	}
}

impl<O> Future for PinnedFuture<'_, O> {
	type Output = O;
	fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
		self.get_mut().inner.as_mut().poll(cx)
	}
}

// private stuff

pub(crate) fn convert_hyper_req_to_chuchi_req(
	hyper_req: HyperRequest,
	address: SocketAddr,
	configs: &RequestConfigs,
) -> Result<Request, HeaderError> {
	let (parts, body) = hyper_req.into_parts();

	let mut body = Body::from(body);
	body.set_size_limit(Some(configs.size_limit));
	body.set_timeout(Some(configs.timeout));

	let header = convert_hyper_parts_to_chuchi_header(parts, address)?;

	Ok(Request::new(header, body))
}

// // Response
pub(crate) fn convert_chuchi_resp_to_hyper_resp(
	response: Response,
) -> hyper::Response<BodyHttp> {
	// debug_checks
	#[cfg(debug_assertions)]
	let _ = validate_content_length(&response);

	let mut header = response.header;

	if !matches!(header.content_type, ContentType::None) {
		let e = header.values.try_insert(CONTENT_TYPE, header.content_type);
		if let Err(e) = e {
			error!("could not insert content type: {e}");
		}
	}

	let mut builder = hyper::Response::builder().status(header.status_code);

	*builder.headers_mut().unwrap() = header.values.into_inner();

	// builder failes if any argument failed
	// but no argument can fail that we pass here
	builder.body(response.body.into_http_body()).unwrap()
}

#[cfg(debug_assertions)]
fn validate_content_length(response: &Response) -> Option<()> {
	let len = response.header().value(crate::header::CONTENT_LENGTH)?;

	let len: usize = len.parse().expect("content-length not a number");

	let body_len = response.body.len()?;

	assert_eq!(len, body_len);

	Some(())
}
