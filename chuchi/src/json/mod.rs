use crate::error::ServerErrorKind;
use crate::header::Mime;
use crate::{Body, Error, Response};

use serde::Serialize;

pub trait IntoRouteResult<T> {
	fn into_route_result(self) -> crate::Result<T>;
}

impl<T> IntoRouteResult<T> for crate::Result<T>
where
	T: Serialize,
{
	fn into_route_result(self) -> crate::Result<T> {
		self
	}
}

impl<T> IntoRouteResult<T> for T
where
	T: Serialize,
{
	fn into_route_result(self) -> crate::Result<T> {
		Ok(self)
	}
}

pub fn serialize_to_response<T>(data: &T) -> crate::Result<Response>
where
	T: Serialize + ?Sized,
{
	let body = Body::serialize(data)
		.map_err(|e| Error::new(ServerErrorKind::InternalServerError, e))?;

	let resp = Response::builder()
		.content_type(Mime::JSON)
		.body(body)
		.build();

	Ok(resp)
}
