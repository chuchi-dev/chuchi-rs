mod raw_route;
pub use raw_route::{HyperBody, HyperRequest, RawRoute};

mod route;
pub use route::{Route, RoutePath};

mod router;
use router::Router;

mod catcher;
pub use catcher::Catcher;

mod path_params;
pub use path_params::{ParamsNames, PathParams};

use crate::header::Method;

use std::slice;

type BoxedRawRoute = Box<dyn RawRoute>;
type BoxedRoute = Box<dyn Route>;
type BoxedCatcher = Box<dyn Catcher>;

pub struct Routes {
	raw: Router<BoxedRawRoute>,
	basic: Router<BoxedRoute>,
	catcher: Vec<BoxedCatcher>,
}

impl Routes {
	pub fn new() -> Self {
		Self {
			raw: Router::new(),
			basic: Router::new(),
			catcher: vec![],
		}
	}

	#[track_caller]
	pub fn push_raw<R>(&mut self, path: RoutePath, route: R)
	where
		R: RawRoute + 'static,
	{
		self.raw
			.insert(path.method.as_ref(), path.path, Box::new(route))
			.unwrap();
	}

	#[track_caller]
	pub fn push<R>(&mut self, path: RoutePath, route: R)
	where
		R: Route + 'static,
	{
		self.basic
			.insert(path.method.as_ref(), path.path, Box::new(route))
			.unwrap();
	}

	pub fn push_catcher<C>(&mut self, catcher: C)
	where
		C: Catcher + 'static,
	{
		self.catcher.push(Box::new(catcher))
	}

	pub fn route_raw<'a>(
		&'a self,
		method: &Method,
		path: &str,
	) -> Option<(&'a BoxedRawRoute, PathParams)> {
		let (route, params) = self
			.raw
			// first try with the correct method
			.at(Some(method), path)
			.or_else(|| self.raw.at(None, path))?;

		Some((route, PathParams::new(params)))
	}

	pub fn route<'a>(
		&'a self,
		method: &Method,
		path: &str,
	) -> Option<(&'a BoxedRoute, PathParams)> {
		let (route, params) = self
			.basic
			// first try with the correct method
			.at(Some(method), path)
			.or_else(|| self.basic.at(None, path))?;

		Some((route, PathParams::new(params)))
	}

	pub fn catchers(&self) -> slice::Iter<'_, BoxedCatcher> {
		self.catcher.iter()
	}
}

impl Default for Routes {
	fn default() -> Self {
		Self::new()
	}
}
