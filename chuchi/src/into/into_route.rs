use crate::routes::Route;

pub trait IntoRoute {
	type IntoRoute: Route;

	fn into_route(self) -> Self::IntoRoute;
}

impl<R> IntoRoute for R
where
	R: Route,
{
	type IntoRoute = Self;

	fn into_route(self) -> Self {
		self
	}
}
