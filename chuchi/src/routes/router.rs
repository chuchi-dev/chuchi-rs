use matchit::Match;

use crate::header::Method;

fn method_to_num(method: Option<&Method>) -> usize {
	match method {
		Some(&Method::GET) => 0,
		Some(&Method::POST) => 1,
		Some(&Method::PUT) => 2,
		Some(&Method::DELETE) => 3,
		Some(&Method::HEAD) => 4,
		Some(&Method::OPTIONS) => 5,
		Some(&Method::CONNECT) => 6,
		Some(&Method::PATCH) => 7,
		Some(&Method::TRACE) => 8,
		_ => 9,
	}
}

enum Route<T> {
	Value(T),
	// This needs to point to a route with a value
	OtherRoute(String),
}

pub struct Router<T> {
	inner: Box<[matchit::Router<Route<T>>]>,
}

impl<T> Router<T> {
	pub fn new() -> Self {
		Self {
			inner: (0..=9).map(|_| matchit::Router::new()).collect(),
		}
	}

	pub fn insert(
		&mut self,
		method: Option<&Method>,
		path: impl Into<String>,
		value: T,
	) -> Result<(), matchit::InsertError> {
		let num = method_to_num(method);
		// todo we wan't syntax {*?rest} to be supported
		let path = path.into();

		if let Some((root_path, new_path)) = optional_rest_path(&path) {
			// make sure we handle all possible requests
			// /wild/rest /wild/ /wild

			self.inner[num].insert(&root_path, Route::Value(value))?;
			if root_path.ends_with('/') && root_path.len() > 1 {
				self.inner[num].insert(
					&root_path[..root_path.len() - 1],
					Route::OtherRoute(root_path.clone()),
				)?;
			}
			self.inner[num].insert(new_path, Route::OtherRoute(root_path))
		} else {
			self.inner[num].insert(path, Route::Value(value))
		}
	}

	pub fn at<'a, 'b>(
		&'a self,
		method: Option<&Method>,
		path: &'b str,
	) -> Option<(&'a T, matchit::Params<'a, 'b>)> {
		let num = method_to_num(method);

		let (route, params) = self.inner[num]
			.at(path)
			.map(|mat| (mat.value, mat.params))
			.ok()?;

		match route {
			Route::Value(v) => Some((v, params)),
			Route::OtherRoute(path) => match self.inner[num].at(path) {
				Ok(Match {
					value: Route::Value(v),
					..
				}) => Some((v, params)),
				Ok(_) => unreachable!(),
				Err(_) => None,
			},
		}
	}
}

fn optional_rest_path(path: &str) -> Option<(String, String)> {
	if !path.ends_with('}') {
		return None;
	}

	// check if {*? exists
	let pat_start = path.rfind("{*?")?;

	Some((
		path[..pat_start].to_string(),
		format!("{}{{*{}", &path[..pat_start], &path[pat_start + 3..]),
	))
}
