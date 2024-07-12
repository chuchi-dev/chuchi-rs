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

pub struct Router<T> {
	inner: Box<[matchit::Router<T>]>,
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
		self.inner[num].insert(path, value)
	}

	pub fn at<'a, 'b>(
		&'a self,
		method: Option<&Method>,
		path: &'b str,
	) -> Option<(&'a T, matchit::Params<'a, 'b>)> {
		let num = method_to_num(method);

		self.inner[num]
			.at(path)
			.map(|mat| (mat.value, mat.params))
			.ok()
	}

	/* pub fn get(&self, method: &Method) -> &T {
		&self.inner[*method as usize]
	}

	pub fn get_mut(&mut self, method: &Method) -> &mut T {
		&mut self.inner[*method as usize]
	} */
}
