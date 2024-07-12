use crate::header::{Mime, StatusCode};
use crate::{Body, Response};

use bytes::Bytes;

pub trait IntoResponse {
	fn into_response(self) -> Response;
}

macro_rules! into_response {
	($self:ident: $type:ty $b:block) => (
		impl IntoResponse for $type {
			fn into_response($self) -> Response { $b }
		}
	)
}

into_response!(self: Response { self });
into_response!(self: StatusCode { self.into() });
into_response!(self: Body { self.into() });

into_response!(self: &'static str {
	Response::builder()
		.content_type(Mime::TEXT)
		.body(self)
		.build()
});

into_response!(self: String {
	Response::builder()
		.content_type(Mime::TEXT)
		.body(self)
		.build()
});

into_response!(self: Vec<u8> {
	Response::builder()
		.content_type(Mime::BINARY)
		.body(self)
		.build()
});

into_response!(self: Bytes {
	Response::builder()
		.content_type(Mime::BINARY)
		.body(self)
		.build()
});

into_response!(self: () {
	Body::new().into()
});
