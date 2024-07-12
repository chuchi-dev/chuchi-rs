use super::{
	size_limit_reached, timed_out, BoxedSyncRead, Constraints,
	HyperBodyAsAsyncBytesStream, PinnedAsyncBytesStream, PinnedAsyncRead,
};

use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};
use tokio::time::Sleep;
use tokio_util::io::StreamReader;

use bytes::Bytes;
use pin_project_lite::pin_project;

pin_project! {
	pub struct BodyAsyncReader {
		#[pin]
		reader: ConstrainedAsyncReader<Inner>
	}
}

impl BodyAsyncReader {
	pub(super) fn new(inner: super::Inner, constraints: Constraints) -> Self {
		let inner = match inner {
			super::Inner::Empty => Inner::Bytes(Bytes::new()),
			super::Inner::Bytes(b) => Inner::Bytes(b),
			super::Inner::Hyper(i) => Inner::Hyper(StreamReader::new(
				HyperBodyAsAsyncBytesStream::new(i),
			)),
			super::Inner::SyncReader(r) => Inner::SyncReader(r),
			super::Inner::AsyncReader(r) => Inner::AsyncReader(r),
			super::Inner::AsyncBytesStreamer(s) => {
				Inner::AsyncBytesStreamer(StreamReader::new(s))
			}
		};

		Self {
			reader: ConstrainedAsyncReader::new(inner, constraints),
		}
	}
}

impl AsyncRead for BodyAsyncReader {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context,
		buf: &mut ReadBuf,
	) -> Poll<io::Result<()>> {
		let me = self.project();
		me.reader.poll_read(cx, buf)
	}
}

enum Inner {
	Bytes(Bytes),
	Hyper(StreamReader<HyperBodyAsAsyncBytesStream, Bytes>),
	SyncReader(BoxedSyncRead),
	AsyncReader(PinnedAsyncRead),
	AsyncBytesStreamer(StreamReader<PinnedAsyncBytesStream, Bytes>),
}

impl AsyncRead for Inner {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context,
		buf: &mut ReadBuf,
	) -> Poll<io::Result<()>> {
		let me = self.get_mut();

		match me {
			Self::Bytes(b) => {
				if b.is_empty() {
					return Poll::Ready(Ok(()));
				}

				let read = buf.remaining().min(b.len());
				buf.put_slice(&b.split_to(read));
				Poll::Ready(Ok(()))
			}
			Self::Hyper(i) => Pin::new(i).poll_read(cx, buf),
			Self::SyncReader(r) => {
				// todo implement this without blocking the current thread
				let filled = match r.read(buf.initialize_unfilled()) {
					Ok(o) => o,
					Err(e) => return Poll::Ready(Err(e)),
				};

				buf.advance(filled);

				Poll::Ready(Ok(()))
			}
			Self::AsyncReader(r) => Pin::new(r).poll_read(cx, buf),
			Self::AsyncBytesStreamer(s) => Pin::new(s).poll_read(cx, buf),
		}
	}
}

pin_project! {
	pub(super) struct ConstrainedAsyncReader<R> {
		#[pin]
		inner: R,
		#[pin]
		timeout: Option<Sleep>,
		size_limit: Option<usize>
	}
}

impl<R> ConstrainedAsyncReader<R> {
	pub fn new(reader: R, constraints: Constraints) -> Self {
		Self {
			inner: reader,
			timeout: constraints.timeout.map(tokio::time::sleep),
			size_limit: constraints.size,
		}
	}
}

impl<R: AsyncRead> AsyncRead for ConstrainedAsyncReader<R> {
	fn poll_read(
		self: Pin<&mut Self>,
		cx: &mut Context,
		buf: &mut ReadBuf,
	) -> Poll<io::Result<()>> {
		let mut me = self.project();

		let prev_filled = buf.filled().len();

		if let Poll::Ready(r) = me.inner.poll_read(cx, buf) {
			if let Err(e) = r {
				return Poll::Ready(Err(e));
			}

			// validate size_limit
			if let Some(size_limit) = &mut me.size_limit {
				let read = buf.filled().len() - prev_filled;
				match size_limit.checked_sub(read) {
					Some(ns) => *size_limit = ns,
					None => {
						return Poll::Ready(Err(size_limit_reached(
							"async reader to big",
						)))
					}
				}
			}

			return Poll::Ready(Ok(()));
		}

		// pending
		if let Some(timeout) = Option::as_pin_mut(me.timeout) {
			if timeout.poll(cx).is_ready() {
				return Poll::Ready(Err(timed_out("async reader took to long")));
			}
		}

		Poll::Pending
	}
}

pub(super) async fn async_reader_into_bytes(
	r: PinnedAsyncRead,
	constraints: Constraints,
) -> io::Result<Bytes> {
	let reader = ConstrainedAsyncReader::new(r, constraints);
	tokio::pin!(reader);

	let mut v = vec![];
	reader.read_to_end(&mut v).await?;

	Ok(v.into())
}
