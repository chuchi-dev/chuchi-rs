use std::{
	collections::{HashMap, HashSet},
	str::FromStr,
};

use byte_parser::{ParseIterator, StrParser};
use matchit::Params;

#[derive(Debug, Clone)]
pub struct PathParams {
	inner: HashMap<String, String>,
}

impl PathParams {
	pub(crate) fn new(params: Params) -> Self {
		let mut inner = HashMap::new();

		for (key, value) in params.iter() {
			inner.insert(key.to_string(), value.to_string());
		}

		Self { inner }
	}

	pub fn exists(&self, key: impl AsRef<str>) -> bool {
		self.inner.contains_key(key.as_ref())
	}

	/// ## Note
	/// Due to the fact that {*?rest} exists, a key which does not exist
	/// will be treated as an empty string
	pub fn parse<T>(&self, key: impl AsRef<str>) -> Result<T, T::Err>
	where
		T: FromStr,
	{
		self.inner
			.get(key.as_ref())
			.map(AsRef::as_ref)
			.unwrap_or("")
			.parse()
	}

	/// ## Note
	/// Due to the fact that {*?rest} exists the rest might not always
	/// be set, so none might be returned even if ParamsNames says it exists
	pub fn get(&self, key: impl AsRef<str>) -> Option<&str> {
		self.inner.get(key.as_ref()).map(|s| s.as_str())
	}
}

/* pub struct Parser {
	pub path: &'a str
} */

// we need to parse {}
// and allow it to be escaped with {{ or }}

#[derive(Debug, Clone)]
pub struct ParamsNames<'a> {
	list: HashSet<&'a str>,
}

impl<'a> ParamsNames<'a> {
	pub fn parse(s: &'a str) -> Self {
		let mut parser = StrParser::new(s);

		let mut list = HashSet::new();

		#[allow(clippy::never_loop)]
		'template_loop: loop {
			parser.consume_while_byte_fn(|&b| b != b'{');
			// either we're at the end or we found a {
			// we need to check for escapes
			let Some(b) = parser.next() else {
				return Self { list };
			};
			debug_assert_eq!(b, b'{');

			// handle escapes
			if parser.next_if(|&b| b == b'{').is_some() {
				continue 'template_loop;
			}

			let mut parser = parser.record();

			loop {
				parser.consume_while_byte_fn(|&b| b != b'}' && b != b'{');
				match parser.peek() {
					Some(b'{') => {
						panic!("unexpected {{");
					}
					Some(b'}') => {
						assert!(
							!matches!(parser.peek_at(2), Some(b) if b == b'}'),
							"escapping does not work in template string"
						);

						let s = parser
							.to_str()
							// trim `{*?name}` name trimming this at once makes sure * is not
							// where it doesn't work
							.trim_start_matches("*?")
							// trim `{*name}`
							.trim_start_matches('*');
						list.insert(s);

						parser.next().unwrap();

						continue 'template_loop;
					}
					Some(b) => unreachable!("reached byte {b}"),
					None => {
						panic!("unexpected end of string");
					}
				}
			}
		}
	}

	pub fn exists(&self, key: impl AsRef<str>) -> bool {
		self.list.contains(key.as_ref())
	}

	pub fn is_empty(&self) -> bool {
		self.list.is_empty()
	}
}
