use crate::header::{
	Mime, RequestHeader, StatusCode, ACCEPT_RANGES, CONTENT_LENGTH,
	CONTENT_RANGE, RANGE,
};
use crate::into::IntoResponse;
use crate::{Body, Response};

use std::fmt;
use std::io::SeekFrom;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};

use io::AsyncSeekExt;
use tokio::{fs, io};

use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct Range {
	/// zero index and inclusive
	pub start: usize,
	/// zero index and inclusive
	pub end: Option<usize>,
}

impl Range {
	pub fn parse(header: &RequestHeader) -> Option<Self> {
		let range = header.value(RANGE)?;
		if !range.starts_with("bytes=") {
			return None;
		}

		let mut range = range[6..].split('-');

		let start: usize = range.next()?.parse().ok()?;

		let end = range.next()?;
		let end: Option<usize> = if end != "" {
			Some(end.parse().ok()?)
		} else {
			None
		};

		Some(Self { start, end })
	}
}

pub fn serve_memory_partial_file(
	path: &'static str,
	bytes: &'static [u8],
	range: Range,
) -> io::Result<Response> {
	let mime_type = path
		.rsplit('.')
		.next()
		.and_then(Mime::from_extension)
		.unwrap_or(Mime::BINARY);

	let size = bytes.len();
	let start = range.start;
	let end = range.end.unwrap_or(size - 1);

	if end >= size || start >= end {
		return Err(io::Error::new(
			io::ErrorKind::Other,
			RangeIncorrect(range),
		));
	}

	let len = (end + 1) - start;

	let response = Response::builder()
		.status_code(StatusCode::PARTIAL_CONTENT)
		.content_type(mime_type)
		.header(ACCEPT_RANGES, "bytes")
		.header(CONTENT_LENGTH, len)
		.header(CONTENT_RANGE, format!("bytes {}-{}/{}", start, end, size))
		.body(Bytes::from_static(&bytes[start..=end]))
		.build();

	Ok(response)
}

pub struct PartialFile {
	file: fs::File,
	mime_type: Mime,
	// the size in bytes of the entire file
	size: usize,
	// where to start reading
	start: usize,
	// at which byte to stop reading (inclusive)
	end: usize,
}

impl PartialFile {
	/// returns not found if the path is not a directory
	pub async fn open<P>(path: P, range: Range) -> io::Result<Self>
	where
		P: AsRef<Path>,
	{
		let extension = path.as_ref().extension().and_then(|f| f.to_str());

		let mime_type = extension
			.and_then(Mime::from_extension)
			.unwrap_or(Mime::BINARY);

		let mut file = fs::File::open(path).await?;
		let metadata = file.metadata().await?;

		// make sure we open a file
		if !metadata.is_file() {
			return Err(io::Error::new(
				io::ErrorKind::NotFound,
				"expected file found folder",
			));
		}

		let size: usize = metadata.len().try_into().map_err(|_| {
			io::Error::new(io::ErrorKind::NotFound, "file to large")
		})?;
		let start = range.start;
		let end = range.end.unwrap_or(size - 1);

		if end >= size || start >= end {
			return Err(io::Error::new(
				io::ErrorKind::Other,
				RangeIncorrect(range),
			));
		}

		file.seek(SeekFrom::Start(start as u64)).await?;

		// apache no-gzip
		// content type

		Ok(Self {
			file,
			mime_type,
			size,
			start,
			end,
		})
	}
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct RangeIncorrect(pub Range);

impl fmt::Display for RangeIncorrect {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

impl std::error::Error for RangeIncorrect {}

// TODO NEED TO CHANGE u64
// A File which streams a range
#[derive(Debug)]
struct FixedFile {
	file: fs::File,
	remaining: u64,
}

impl FixedFile {
	pub fn new(file: fs::File, len: u64) -> Self {
		Self {
			file,
			remaining: len,
		}
	}
}

impl io::AsyncRead for FixedFile {
	fn poll_read(
		mut self: Pin<&mut Self>,
		cx: &mut Context,
		buf: &mut io::ReadBuf,
	) -> Poll<io::Result<()>> {
		// if finished reading
		if self.remaining == 0 {
			return Poll::Ready(Ok(()));
		}

		// take a max amount of buffer to not write to much
		let (initialized, filled) = {
			let mut buf =
				buf.take(self.remaining.try_into().unwrap_or(usize::MAX));
			debug_assert!(buf.filled().is_empty());

			let res = Pin::new(&mut self.file).poll_read(cx, &mut buf);
			match res {
				Poll::Ready(Ok(())) => {}
				Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
				Poll::Pending => return Poll::Pending,
			}

			(buf.initialized().len(), buf.filled().len())
		};

		// this is safe since take returns a ReadBuf and it only returns
		// bytes initializes that are.
		unsafe {
			buf.assume_init(buf.filled().len() + initialized);
		}
		buf.advance(filled);

		if filled == 0 {
			return Poll::Ready(Err(io::Error::new(
				io::ErrorKind::UnexpectedEof,
				"The file is to short",
			)));
		}

		self.remaining = self.remaining.checked_sub(filled as u64).unwrap();

		Poll::Ready(Ok(()))
	}
}

impl IntoResponse for PartialFile {
	fn into_response(self) -> Response {
		let len = (self.end + 1) - self.start;

		let response = Response::builder()
			.status_code(StatusCode::PARTIAL_CONTENT)
			.content_type(self.mime_type)
			.header(ACCEPT_RANGES, "bytes")
			.header(CONTENT_LENGTH, len)
			.header(
				CONTENT_RANGE,
				format!("bytes {}-{}/{}", self.start, self.end, self.size),
			);

		// the file is already at the correct start
		// since open did that

		// if self.end points to the end of the file just return the file
		// without limiting the reading
		if self.end + 1 == self.size {
			response.body(Body::from_async_reader(self.file)).build()
		} else {
			let fixed_file = FixedFile::new(self.file, len as u64);
			response.body(Body::from_async_reader(fixed_file)).build()
		}
	}
}
