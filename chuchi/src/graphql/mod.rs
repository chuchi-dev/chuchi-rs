/// This is unstable
mod graphiql;

use crate::error::ClientErrorKind;
use crate::header::{self, Method, Mime, RequestHeader, StatusCode};
use crate::routes::{ParamsNames, PathParams, Route, RoutePath};
use crate::util::PinnedFuture;
use crate::{Body, Error, Request, Resources, Response};

use std::any::{Any, TypeId};

use juniper::http::{GraphQLBatchRequest, GraphQLRequest};
use juniper::{
	GraphQLSubscriptionType, GraphQLType, GraphQLTypeAsync, RootNode,
	ScalarValue,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphiQl {
	uri: &'static str,
	graphql_uri: &'static str,
}

impl GraphiQl {
	pub const fn new(uri: &'static str, graphql_uri: &'static str) -> Self {
		Self { uri, graphql_uri }
	}
}

impl Route for GraphiQl {
	fn validate_requirements(&self, _params: &ParamsNames, _data: &Resources) {}

	fn path(&self) -> RoutePath {
		RoutePath {
			method: Some(Method::GET),
			path: format!("{}/{{*rem}}", self.uri.trim_end_matches('/')).into(),
		}
	}

	fn call<'a>(
		&'a self,
		_req: &'a mut Request,
		_params: &'a PathParams,
		_: &'a Resources,
	) -> PinnedFuture<'a, crate::Result<Response>> {
		PinnedFuture::new(async move {
			Ok(Response::html(graphiql::graphiql_source(self.graphql_uri)))
		})
	}
}

pub struct GraphQlContext {
	data: Resources,
	request_header: RequestHeader,
}

impl GraphQlContext {
	// Gets data or RequestHeader
	pub fn get<D>(&self) -> Option<&D>
	where
		D: Any,
	{
		if TypeId::of::<D>() == TypeId::of::<RequestHeader>() {
			<dyn Any>::downcast_ref(&self.request_header)
		} else {
			self.data.get()
		}
	}
}

impl juniper::Context for GraphQlContext {}

/// This only supports POST requests
pub struct GraphQl<Q, M, Sub, S>
where
	Q: GraphQLType<S, Context = GraphQlContext>,
	M: GraphQLType<S, Context = GraphQlContext>,
	Sub: GraphQLType<S, Context = GraphQlContext>,
	S: ScalarValue,
{
	uri: &'static str,
	root_node: RootNode<'static, Q, M, Sub, S>,
}

impl<Q, M, Sub, S> GraphQl<Q, M, Sub, S>
where
	Q: GraphQLType<S, Context = GraphQlContext>,
	M: GraphQLType<S, Context = GraphQlContext>,
	Sub: GraphQLType<S, Context = GraphQlContext>,
	S: ScalarValue,
{
	pub fn new(
		uri: &'static str,
		root_node: RootNode<'static, Q, M, Sub, S>,
	) -> Self {
		Self { uri, root_node }
	}
}

impl<Q, M, Sub, S> Route for GraphQl<Q, M, Sub, S>
where
	Q: GraphQLTypeAsync<S, Context = GraphQlContext> + Send,
	Q::TypeInfo: Send + Sync,
	M: GraphQLTypeAsync<S, Context = GraphQlContext> + Send,
	M::TypeInfo: Send + Sync,
	Sub: GraphQLSubscriptionType<S, Context = GraphQlContext> + Send,
	Sub::TypeInfo: Send + Sync,
	S: ScalarValue + Send + Sync,
{
	fn validate_requirements(&self, _params: &ParamsNames, _data: &Resources) {}

	fn path(&self) -> RoutePath {
		RoutePath {
			method: Some(Method::POST),
			path: self.uri.into(),
		}
	}

	fn call<'a>(
		&'a self,
		req: &'a mut Request,
		_params: &'a PathParams,
		data: &'a Resources,
	) -> PinnedFuture<'a, crate::Result<Response>> {
		PinnedFuture::new(async move {
			// get content-type of request
			let content_type =
				req.header().value(header::CONTENT_TYPE).unwrap_or("");

			let gql_req: GraphQLBatchRequest<S> = match content_type {
				"application/json" => {
					// read json
					req.deserialize().await?
				}
				"application/graphql" => {
					let body = req
						.body
						.take()
						.into_string()
						.await
						.map_err(Error::from_client_io)?;

					GraphQLBatchRequest::Single(GraphQLRequest::new(
						body, None, None,
					))
				}
				_ => return Err(ClientErrorKind::BadRequest.into()),
			};

			let ctx = GraphQlContext {
				data: data.clone(),
				request_header: req.header().clone(),
			};
			let res = gql_req.execute(&self.root_node, &ctx).await;

			let mut resp = Response::builder().content_type(Mime::JSON);

			if !res.is_ok() {
				resp = resp.status_code(StatusCode::BAD_REQUEST);
			}

			Ok(resp.body(Body::serialize(&res).unwrap()).build())
		})
	}
}
