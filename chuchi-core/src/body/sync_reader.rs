use super::{size_limit_reached, BodyAsyncReader, BoxedSyncRead, Constraints};

use std::io;
use std::io::Read;
use std::pin::Pin;

use tokio_util::io::SyncIoBridge;

use bytes::Bytes;

/// ## Panics
/// If not read within `task::spawn_blocking`.
pub struct BodySyncReader {
	inner: Inner,
}

impl BodySyncReader {
	pub(super) fn new(inner: super::Inner, constraints: Constraints) -> Self {
		let inner = match inner {
			super::Inner::Empty => Inner::Empty,
			super::Inner::Bytes(b) => Inner::Sync(ConstrainedSyncReader::new(
				InnerSync::Bytes(b),
				constraints,
			)),
			super::Inner::SyncReader(r) => {
				Inner::Sync(ConstrainedSyncReader::new(
					InnerSync::SyncReader(r),
					constraints,
				))
			}
			i => Inner::Async(SyncIoBridge::new(Box::pin(
				BodyAsyncReader::new(i, constraints),
			))),
		};

		Self { inner }
	}

	/// Returns true if this needs to be run within spawn_blocking.
	pub fn needs_spawn_blocking(&self) -> bool {
		matches!(self.inner, Inner::Async(_))
	}
}

impl Read for BodySyncReader {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		self.inner.read(buf)
	}
}

enum Inner {
	Empty,
	Sync(ConstrainedSyncReader<InnerSync>),
	Async(SyncIoBridge<Pin<Box<BodyAsyncReader>>>),
}

impl Read for Inner {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		match self {
			Self::Empty => Ok(0),
			Self::Sync(r) => r.read(buf),
			Self::Async(r) => r.read(buf),
		}
	}
}

enum InnerSync {
	Bytes(Bytes),
	SyncReader(BoxedSyncRead),
}

impl Read for InnerSync {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		match self {
			Self::Bytes(b) if b.is_empty() => Ok(0),
			Self::Bytes(b) => {
				let read = buf.len().min(b.len());
				let r = b.split_to(read);
				buf[..read].copy_from_slice(&r);
				Ok(read)
			}
			Self::SyncReader(r) => r.read(buf),
		}
	}
}

/// Only using size constraint
struct ConstrainedSyncReader<R> {
	inner: R,
	size_limit: Option<usize>,
}

impl<R> ConstrainedSyncReader<R> {
	pub fn new(reader: R, constraints: Constraints) -> Self {
		Self {
			inner: reader,
			size_limit: constraints.size,
		}
	}
}

impl<R: Read> Read for ConstrainedSyncReader<R> {
	fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
		let read = self.inner.read(buf)?;

		if let Some(size_limit) = &mut self.size_limit {
			match size_limit.checked_sub(read) {
				Some(ns) => *size_limit = ns,
				None => return Err(size_limit_reached("sync reader to big")),
			}
		}

		Ok(read)
	}
}

pub(super) fn sync_reader_into_bytes(
	r: BoxedSyncRead,
	constraints: Constraints,
) -> io::Result<Bytes> {
	let mut reader = ConstrainedSyncReader::new(r, constraints);

	let mut v = vec![];
	reader.read_to_end(&mut v)?;

	Ok(v.into())
}
