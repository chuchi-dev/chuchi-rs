// todo if this should ever be used outside of testing the uri parameter
// should be replaced with a hashmap or something simiar
use super::error::ApiError;

use crate::header::{HeaderValues, Method, Mime, StatusCode};
use crate::request::DeserializeError;
use crate::resources::Resources;
use crate::routes::ParamsNames;
use crate::{Body, ChuchiShared, Error, Request, Response};
use serde::de::DeserializeOwned;

pub struct ChuchiSharedApi {
	inner: ChuchiShared,
}

impl ChuchiSharedApi {
	pub fn new(inner: ChuchiShared) -> Self {
		Self { inner }
	}

	#[deprecated = "Use `ChuchiSharedApi::resources` instead"]
	pub fn data(&self) -> &Resources {
		self.inner.resources()
	}

	/// Returns a reference to the resources.
	pub fn resources(&self) -> &Resources {
		self.inner.resources()
	}

	/// Routes the request to normal routes and returns their result.
	///
	/// Useful for tests and niche applications.
	///
	/// Returns None if no route was found matching the request.
	pub async fn route(
		&self,
		req: &mut Request,
	) -> Option<Result<Response, Error>> {
		self.inner.route(req).await
	}

	pub async fn request<R>(&self, req: &R) -> Result<R::Response, R::Error>
	where
		R: super::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static,
	{
		let params = ParamsNames::parse(R::PATH);
		assert!(params.is_empty(), "path parameters are not allowed");

		self.request_with_header(R::PATH, req, HeaderValues::new())
			.await
	}

	pub async fn request_with_uri<R>(
		&self,
		uri: impl AsRef<str>,
		req: &R,
	) -> Result<R::Response, R::Error>
	where
		R: super::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static,
	{
		self.request_with_header(uri.as_ref(), req, HeaderValues::new())
			.await
	}

	pub async fn request_with_header<R>(
		&self,
		uri: impl AsRef<str>,
		req: &R,
		header: HeaderValues,
	) -> Result<R::Response, R::Error>
	where
		R: super::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static,
	{
		let mut raw_req =
			Request::builder(uri.as_ref().parse().unwrap()).method(R::METHOD);
		*raw_req.values_mut() = header;

		let mut req = if R::METHOD == &Method::GET {
			raw_req.serialize_query(req).unwrap().build()
		} else {
			raw_req
				.content_type(Mime::JSON)
				.body(Body::serialize(req).unwrap())
				.build()
		};

		self.request_raw::<R>(&mut req).await
	}

	pub async fn request_raw<R>(
		&self,
		req: &mut Request,
	) -> Result<R::Response, R::Error>
	where
		R: super::Request,
		R::Error: DeserializeOwned + Send + 'static,
		R::Response: Send + 'static,
	{
		let mut resp = self
			.route(req)
			.await
			.unwrap()
			.map_err(super::error::Error::Chuchi)
			.map_err(R::Error::from_error)?;

		if resp.header().status_code() != &StatusCode::OK {
			let e = resp
				.body
				.deserialize()
				.await
				.map_err(DeserializeError::Json)
				.map_err(super::error::Error::Deserialize)
				.map_err(R::Error::from_error)?;

			return Err(e);
		}

		resp.take_body()
			.deserialize()
			.await
			.map_err(DeserializeError::Json)
			.map_err(super::error::Error::Deserialize)
			.map_err(R::Error::from_error)
	}
}
