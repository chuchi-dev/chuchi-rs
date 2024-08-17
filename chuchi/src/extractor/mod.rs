use std::convert::Infallible;
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use std::str::FromStr;
use std::{future::Future, ops::Deref};

use crate::error::{ClientErrorKind, ErrorKind, ServerErrorKind};
use crate::header::RequestHeader;
use crate::state::StateValidation;
use crate::{
	routes::{ParamsNames, PathParams},
	state::State,
	Request, Resources,
};

#[non_exhaustive]
pub struct Validate<'a> {
	pub name: &'a str,
	pub params: &'a ParamsNames<'a>,
	pub state: &'a mut StateValidation,
	pub resources: &'a Resources,
}

#[non_exhaustive]
pub struct Prepare<'a> {
	pub name: &'a str,
	pub header: &'a RequestHeader,
	pub params: &'a PathParams,
	pub state: &'a mut State,
	pub resources: &'a Resources,
}

#[non_exhaustive]
pub struct Extract<'a, 'b, P, R> {
	pub prepared: P,
	pub name: &'b str,
	pub request: &'b mut Option<R>,
	pub params: &'a PathParams,
	pub state: &'a State,
	pub resources: &'a Resources,
}

pub trait Extractor<'a, R> {
	type Error: ExtractorError;
	type Prepared;

	fn validate(validate: Validate<'_>);

	#[allow(clippy::type_complexity)]
	fn prepare(
		prepare: Prepare<'_>,
	) -> Pin<
		Box<
			dyn Future<Output = Result<Self::Prepared, Self::Error>>
				+ Send
				+ '_,
		>,
	>;

	fn extract(
		extract: Extract<'a, '_, Self::Prepared, R>,
	) -> Result<Self, Self::Error>
	where
		Self: Sized;
}

pub trait ExtractorError: StdError + Send + Sync {
	fn error_kind(&self) -> ErrorKind;

	fn into_std(self) -> Box<dyn StdError + Send + Sync>;
}

impl ExtractorError for Infallible {
	fn error_kind(&self) -> ErrorKind {
		unreachable!()
	}

	fn into_std(self) -> Box<dyn StdError + Send + Sync> {
		unreachable!()
	}
}

#[derive(Debug)]
pub struct InternalError<E>(pub E);

impl<E> ExtractorError for InternalError<E>
where
	E: Into<Box<dyn StdError + Send + Sync>> + StdError + Send + Sync,
{
	fn error_kind(&self) -> ErrorKind {
		ServerErrorKind::InternalServerError.into()
	}

	fn into_std(self) -> Box<dyn StdError + Send + Sync> {
		self.0.into()
	}
}

impl<E> StdError for InternalError<E> where E: StdError {}

impl<E> fmt::Display for InternalError<E>
where
	E: fmt::Display,
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.0.fmt(f)
	}
}

impl<E> From<E> for InternalError<E> {
	fn from(e: E) -> Self {
		Self(e)
	}
}

impl<'a> Validate<'a> {
	pub fn new(
		name: &'a str,
		params: &'a ParamsNames<'a>,
		state: &'a mut StateValidation,
		resources: &'a Resources,
	) -> Self {
		Self {
			name,
			params,
			state,
			resources,
		}
	}
}

impl<'a> Prepare<'a> {
	pub fn new(
		name: &'a str,
		header: &'a RequestHeader,
		params: &'a PathParams,
		state: &'a mut State,
		resources: &'a Resources,
	) -> Self {
		Self {
			name,
			header,
			params,
			state,
			resources,
		}
	}
}

impl<'a, 'b, P, R> Extract<'a, 'b, P, R> {
	pub fn new(
		prepared: P,
		name: &'b str,
		request: &'b mut Option<R>,
		params: &'a PathParams,
		state: &'a State,
		resources: &'a Resources,
	) -> Self {
		Self {
			prepared,
			name,
			request,
			params,
			state,
			resources,
		}
	}
}

pub struct Res<'a, T: ?Sized>(&'a T);

impl<'a, T, R> Extractor<'a, R> for Res<'a, T>
where
	T: Send + Sync + 'static,
{
	type Error = Infallible;
	type Prepared = ();

	extractor_validate!(|validate| {
		validate.resources.get::<T>().unwrap();
	});

	extractor_prepare!();

	extractor_extract!(|extract| {
		Ok(Res(extract.resources.get::<T>().unwrap()))
	});
}

impl<T> Deref for Res<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.0
	}
}

impl<'a> Extractor<'a, &'a mut Request> for &'a mut Request {
	type Error = Infallible;
	type Prepared = ();

	extractor_validate!();

	extractor_prepare!();

	fn extract(
		extract: Extract<'a, '_, Self::Prepared, &'a mut Request>,
	) -> Result<Self, Self::Error>
	where
		Self: Sized,
	{
		Ok(extract.request.take().unwrap())
	}
}

impl<'a, R> Extractor<'a, R> for &'a RequestHeader {
	type Error = Infallible;
	type Prepared = ();

	extractor_validate!();

	extractor_prepare!(|prepare| {
		if !prepare.state.contains::<RequestHeader>() {
			prepare
				.state
				.insert::<RequestHeader>(prepare.header.clone());
		}

		Ok(())
	});

	extractor_extract!(|extract| { Ok(extract.state.get().unwrap()) });
}

pub type PathStr = PathParam<str>;

#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(transparent)]
pub struct PathParam<T: ?Sized>(T);

impl<T: ?Sized> PathParam<T> {
	pub fn from_ref<'a>(s: &'a T) -> &'a Self {
		// safe because `PathParam` is `repr(transparent)`
		unsafe { &*(s as *const T as *const Self) }
	}
}

impl<'a, T, R> Extractor<'a, R> for PathParam<T>
where
	T: Send + Sync + FromStr + 'a,
	T::Err: StdError + Send + Sync + 'static,
{
	type Error = PathError<T::Err>;
	type Prepared = ();

	extractor_validate!(|validate| {
		assert!(
			validate.params.exists(validate.name),
			"Path parameter `{}` does not exist",
			validate.name
		);
	});

	extractor_prepare!();

	extractor_extract!(|extract| {
		extract
			.params
			.parse(extract.name)
			.map(PathParam)
			.map_err(PathError)
	});
}

impl<'a, R> Extractor<'a, R> for &'a PathParam<str> {
	type Error = Infallible;
	type Prepared = ();

	extractor_validate!(|validate| {
		assert!(
			validate.params.exists(validate.name),
			"Path parameter `{}` does not exist",
			validate.name
		);
	});

	extractor_prepare!();

	extractor_extract!(|extract| {
		let s = extract.params.get(extract.name).unwrap_or("");

		Ok(PathParam::from_ref(s))
	});
}

impl<T> Deref for PathParam<T>
where
	T: ?Sized,
{
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl<T> fmt::Display for PathParam<T>
where
	T: fmt::Display + ?Sized,
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		self.0.fmt(f)
	}
}

#[derive(Debug)]
pub struct PathError<T>(pub T);

impl<T> fmt::Display for PathError<T>
where
	T: fmt::Display,
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "Failed to parse path parameter: {}", self.0)
	}
}

impl<T> StdError for PathError<T>
where
	T: StdError + 'static,
{
	fn source(&self) -> Option<&(dyn StdError + 'static)> {
		Some(&self.0)
	}
}

impl<T> ExtractorError for PathError<T>
where
	T: StdError + Send + Sync + 'static,
{
	fn error_kind(&self) -> ErrorKind {
		ErrorKind::Client(ClientErrorKind::BadRequest)
	}

	fn into_std(self) -> Box<dyn StdError + Send + Sync> {
		Box::new(self.0)
	}
}

impl<'a, R: 'a> Extractor<'a, R> for &'a Resources {
	type Error = Infallible;
	type Prepared = ();

	extractor_validate!();

	extractor_prepare!();

	extractor_extract!(|extract| { Ok(extract.resources) });
}
