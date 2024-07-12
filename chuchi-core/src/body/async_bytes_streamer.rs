use super::{
	size_limit_reached, timed_out, BoxedSyncRead, Constraints,
	HyperBodyAsAsyncBytesStream, PinnedAsyncBytesStream, PinnedAsyncRead,
};

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{io, mem};

use tokio::time::Sleep;
use tokio_stream::StreamExt;
use tokio_util::io::ReaderStream;

use futures_core::Stream;

use pin_project_lite::pin_project;

use bytes::{Bytes, BytesMut};

pin_project! {
	pub struct BodyAsyncBytesStreamer {
		#[pin]
		inner: ConstrainedAsyncBytesStreamer<Inner>
	}
}

impl BodyAsyncBytesStreamer {
	pub(super) fn new(inner: super::Inner, constraints: Constraints) -> Self {
		let inner = match inner {
			super::Inner::Empty => Inner::Empty,
			super::Inner::Bytes(b) => Inner::Bytes(b),
			super::Inner::Hyper(i) => {
				Inner::Hyper(HyperBodyAsAsyncBytesStream::new(i))
			}
			super::Inner::SyncReader(r) => Inner::SyncReader {
				reader: r,
				buf: BytesMut::zeroed(DEFAULT_CAP),
			},
			super::Inner::AsyncReader(r) => {
				Inner::AsyncReader(ReaderStream::new(r))
			}
			super::Inner::AsyncBytesStreamer(s) => Inner::AsyncBytesStreamer(s),
		};

		Self {
			inner: ConstrainedAsyncBytesStreamer::new(inner, constraints),
		}
	}
}

impl Stream for BodyAsyncBytesStreamer {
	type Item = io::Result<Bytes>;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context,
	) -> Poll<Option<io::Result<Bytes>>> {
		self.project().inner.poll_next(cx)
	}
}

const DEFAULT_CAP: usize = 4096;

enum Inner {
	Empty,
	Bytes(Bytes),
	Hyper(HyperBodyAsAsyncBytesStream),
	SyncReader {
		reader: BoxedSyncRead,
		buf: BytesMut,
	},
	AsyncReader(ReaderStream<PinnedAsyncRead>),
	AsyncBytesStreamer(PinnedAsyncBytesStream),
}

impl Stream for Inner {
	type Item = io::Result<Bytes>;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context,
	) -> Poll<Option<io::Result<Bytes>>> {
		let me = self.get_mut();

		match me {
			Self::Empty => Poll::Ready(None),
			Self::Bytes(b) => {
				let bytes = mem::take(b);
				*me = Self::Empty;
				Poll::Ready(Some(Ok(bytes)))
			}
			Self::Hyper(i) => Pin::new(i).poll_next(cx),
			Self::SyncReader { reader, buf } => {
				if buf.is_empty() {
					*buf = BytesMut::zeroed(DEFAULT_CAP);
				}

				// todo make this non blocking

				let read = match reader.read(buf) {
					Ok(r) => r,
					Err(e) => return Poll::Ready(Some(Err(e))),
				};

				Poll::Ready(Some(Ok(buf.split_to(read).into())))
			}
			Self::AsyncReader(s) => Pin::new(s).poll_next(cx),
			Self::AsyncBytesStreamer(s) => Pin::new(s).poll_next(cx),
		}
	}
}

pin_project! {
	pub(super) struct ConstrainedAsyncBytesStreamer<S> {
		#[pin]
		inner: S,
		#[pin]
		timeout: Option<Sleep>,
		size_limit: Option<usize>
	}
}

impl<S> ConstrainedAsyncBytesStreamer<S> {
	pub fn new(streamer: S, constraints: Constraints) -> Self {
		Self {
			inner: streamer,
			timeout: constraints.timeout.map(tokio::time::sleep),
			size_limit: constraints.size,
		}
	}
}

impl<S> Stream for ConstrainedAsyncBytesStreamer<S>
where
	S: Stream<Item = io::Result<Bytes>>,
{
	type Item = io::Result<Bytes>;

	fn poll_next(
		self: Pin<&mut Self>,
		cx: &mut Context,
	) -> Poll<Option<io::Result<Bytes>>> {
		let mut me = self.project();

		if let Poll::Ready(r) = me.inner.poll_next(cx) {
			let bytes = match r {
				Some(Ok(b)) => b,
				Some(Err(e)) => return Poll::Ready(Some(Err(e))),
				None => return Poll::Ready(None),
			};

			// validate size_limit
			if let Some(size_limit) = &mut me.size_limit {
				match size_limit.checked_sub(bytes.len()) {
					Some(ns) => *size_limit = ns,
					None => {
						return Poll::Ready(Some(Err(size_limit_reached(
							"async bytes streamer to big",
						))))
					}
				}
			}

			return Poll::Ready(Some(Ok(bytes)));
		}

		// pending
		if let Some(timeout) = Option::as_pin_mut(me.timeout) {
			if timeout.poll(cx).is_ready() {
				return Poll::Ready(Some(Err(timed_out(
					"async bytes streamer took to long",
				))));
			}
		}

		Poll::Pending
	}
}

pub(super) async fn async_bytes_streamer_into_bytes(
	s: impl Stream<Item = io::Result<Bytes>>,
	constraints: Constraints,
) -> io::Result<Bytes> {
	let stream = ConstrainedAsyncBytesStreamer::new(s, constraints);
	tokio::pin!(stream);

	let mut v = BytesMut::new();
	while let Some(bytes) = stream.next().await {
		let bytes = bytes?;
		v.extend(bytes);
	}

	Ok(v.into())
}
