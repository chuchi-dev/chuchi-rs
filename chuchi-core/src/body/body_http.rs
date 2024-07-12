use super::{BodyAsyncBytesStreamer, Constraints};

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::body::{Body as HyperBody, Frame, Incoming};

use futures_core::Stream;

use pin_project_lite::pin_project;

use bytes::Bytes;

pin_project! {
	pub struct BodyHttp {
		#[pin]
		inner: BodyAsyncBytesStreamer
	}
}

impl BodyHttp {
	pub(super) fn new(inner: super::Inner, constraints: Constraints) -> Self {
		Self {
			inner: BodyAsyncBytesStreamer::new(inner, constraints),
		}
	}
}

impl HyperBody for BodyHttp {
	type Data = Bytes;
	type Error = io::Error;

	fn poll_frame(
		self: Pin<&mut Self>,
		cx: &mut Context,
	) -> Poll<Option<io::Result<Frame<Bytes>>>> {
		let me = self.project();
		match me.inner.poll_next(cx) {
			Poll::Ready(Some(Ok(b))) => Poll::Ready(Some(Ok(Frame::data(b)))),
			Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
			Poll::Ready(None) => Poll::Ready(None),
			Poll::Pending => Poll::Pending,
		}
	}
}

pub(super) struct HyperBodyAsAsyncBytesStream {
	inner: Incoming,
}

impl HyperBodyAsAsyncBytesStream {
	pub fn new(inner: Incoming) -> Self {
		Self { inner }
	}
}

impl Stream for HyperBodyAsAsyncBytesStream {
	type Item = io::Result<Bytes>;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context,
	) -> Poll<Option<io::Result<Bytes>>> {
		let me = self.get_mut();
		// loop to retry to get data
		loop {
			let r = match Pin::new(&mut me.inner).poll_frame(cx) {
				Poll::Ready(Some(Ok(frame))) => {
					let Ok(data) = frame.into_data() else {
						continue;
					};

					Poll::Ready(Some(Ok(data)))
				}
				Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(
					io::Error::new(io::ErrorKind::Other, e),
				))),
				Poll::Ready(None) => Poll::Ready(None),
				Poll::Pending => Poll::Pending,
			};

			break r;
		}
	}
}
