#[cfg(feature = "api")]
mod api;
#[cfg(all(feature = "api", feature = "stream"))]
mod api_stream;
mod args;
#[cfg(feature = "api")]
mod request_extractor;
mod resource;
mod route;
mod util;
#[cfg(feature = "ws")]
mod ws;

#[cfg(feature = "api")]
use args::ApiArgs;
use args::Args;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

enum Method {
	Get,
	Post,
	Put,
	Delete,
	Head,
}

impl Method {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Get => "GET",
			Self::Post => "POST",
			Self::Put => "PUT",
			Self::Delete => "DELETE",
			Self::Head => "HEAD",
		}
	}
}

#[allow(dead_code)]
enum TransformOutput {
	No,
	Json,
}

fn to_compile_error(error: syn::Error) -> TokenStream {
	let compile_error = error.to_compile_error();
	quote!(#compile_error).into()
}

macro_rules! attribute_route {
	($name:ident, $method:ident, No) => {
		attribute_route!(_; $name, $method, No,);
	};
	($name:ident, $method:ident, Json) => {
		attribute_route!(_; $name, $method, Json, #[cfg(feature = "json")]);
	};
	(_; $name:ident, $method:ident, $output:ident, $(#[$attr:meta])*) => {
		#[proc_macro_attribute]
		$(#[$attr])*
		pub fn $name(attrs: TokenStream, item: TokenStream) -> TokenStream {
			let args = parse_macro_input!(attrs as Args);
			let item = parse_macro_input!(item as ItemFn);

			let stream = route::expand(
				args,
				item,
				Method::$method,
				TransformOutput::$output
			);

			stream
				.map(|stream| stream.into())
				.unwrap_or_else(to_compile_error)
		}
	}
}

attribute_route!(get, Get, No);
attribute_route!(post, Post, No);
attribute_route!(put, Put, No);
attribute_route!(delete, Delete, No);
attribute_route!(head, Head, No);

attribute_route!(get_json, Get, Json);
attribute_route!(post_json, Post, Json);
attribute_route!(put_json, Put, Json);
attribute_route!(delete_json, Delete, Json);
attribute_route!(head_json, Head, Json);

#[proc_macro_attribute]
#[cfg(feature = "ws")]
pub fn ws(attrs: TokenStream, item: TokenStream) -> TokenStream {
	let args = parse_macro_input!(attrs as Args);
	let item = parse_macro_input!(item as ItemFn);

	let stream = ws::expand(args, item);

	stream
		.map(|stream| stream.into())
		.unwrap_or_else(to_compile_error)
}

#[proc_macro_attribute]
#[cfg(feature = "api")]
pub fn api(attrs: TokenStream, item: TokenStream) -> TokenStream {
	let args = parse_macro_input!(attrs as ApiArgs);
	let item = parse_macro_input!(item as ItemFn);

	let stream = api::expand(args, item);

	stream
		.map(|stream| stream.into())
		.unwrap_or_else(to_compile_error)
}

#[proc_macro_attribute]
#[cfg(all(feature = "api", feature = "stream"))]
pub fn api_stream(attrs: TokenStream, item: TokenStream) -> TokenStream {
	let args = parse_macro_input!(attrs as ApiArgs);
	let item = parse_macro_input!(item as ItemFn);

	let stream = api_stream::expand(args, item);

	stream
		.map(|stream| stream.into())
		.unwrap_or_else(to_compile_error)
}

// todo should this be renamed to request?
#[proc_macro_derive(RequestExtractor)]
#[cfg(feature = "api")]
pub fn derive_request_extractor(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as syn::DeriveInput);

	request_extractor::expand(&input).unwrap_or_else(to_compile_error)
}

#[proc_macro_derive(Resource)]
pub fn derive_resource(input: TokenStream) -> TokenStream {
	let input = parse_macro_input!(input as syn::DeriveInput);

	resource::expand(&input).unwrap_or_else(to_compile_error)
}
