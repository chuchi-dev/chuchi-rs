use super::IntoResponse;
use crate::error::{ClientErrorKind, Error};
use crate::Response;

pub trait IntoRouteResult {
	fn into_route_result(self) -> crate::Result<Response>;
}

impl<R, E> IntoRouteResult for Result<R, E>
where
	R: IntoResponse,
	E: Into<Error>,
{
	fn into_route_result(self) -> crate::Result<Response> {
		self.map(|o| o.into_response()).map_err(|e| e.into())
	}
}

impl<R> IntoRouteResult for Option<R>
where
	R: IntoResponse,
{
	fn into_route_result(self) -> crate::Result<Response> {
		match self {
			Some(r) => Ok(r.into_response()),
			None => Err(Error::empty(ClientErrorKind::NotFound)),
		}
	}
}

impl<R> IntoRouteResult for R
where
	R: IntoResponse,
{
	fn into_route_result(self) -> crate::Result<Response> {
		Ok(self.into_response())
	}
}
