use crate::util::{chuchi_crate, validate_inputs, validate_signature};
use crate::Args;
use crate::{Method, TransformOutput};

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemFn, Result};

pub(crate) fn expand(
	args: Args,
	item: ItemFn,
	method: Method,
	output: TransformOutput,
) -> Result<TokenStream> {
	let chuchi = chuchi_crate()?;

	validate_signature(&item.sig)?;

	// parse inputs to get the data types
	// should ignore request and check that the request gets passed

	// (is_mut, ty)
	// TypeReference
	let inputs = validate_inputs(item.sig.inputs.iter())?;

	let extractor_type =
		quote!(#chuchi::extractor::Extractor<&mut #chuchi::Request>);

	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	let valid_data_fn = {
		let mut asserts = vec![];

		for (name, ty) in &inputs {
			asserts.push(quote!({
				let validate = #chuchi::extractor::Validate::new(
					#name, params, &mut state, resources
				);

				<#ty as #extractor_type>::validate(
					validate
				);
			}));
		}

		quote!(
			fn validate_requirements(
				&self,
				params: &#chuchi::routes::ParamsNames,
				resources: &#chuchi::resources::Resources
			) {
				#[allow(unused_mut, dead_code)]
				let mut state = #chuchi::state::StateValidation::new();

				#(#asserts)*
			}
		)
	};

	let path_fn = {
		let uri = &args.uri;
		let method = format_ident!("{}", method.as_str());

		quote!(
			fn path(&self) -> #chuchi::routes::RoutePath {
				#chuchi::routes::RoutePath {
					method: Some(#chuchi::header::Method::#method),
					path: #uri.into()
				}
			}
		)
	};

	let route_fn = {
		let asyncness = &item.sig.asyncness;
		let inputs = &item.sig.inputs;
		let output = &item.sig.output;
		let block = &item.block;

		quote!(
			#asyncness fn handle_route( #inputs ) #output
				#block
		)
	};

	let call_fn = {
		let async_route_fn = item.sig.asyncness.is_some();
		let await_kw = if async_route_fn {
			quote!(.await)
		} else {
			quote!()
		};

		let mut call_route_args = vec![];
		let mut prepare_extractors = vec![];

		for (i, (name, ty)) in inputs.iter().enumerate() {
			prepare_extractors.push(quote!({
				let prepare = #chuchi::extractor::Prepare::new(
					#name, req.header(), params, &mut state, resources
				);

				let res = <#ty as #extractor_type>::prepare(
					prepare
				).await;

				match res {
					Ok(res) => res,
					Err(e) => {
						return Err(#chuchi::Error::new(
							#chuchi::extractor::ExtractorError::error_kind(&e),
							#chuchi::extractor::ExtractorError::into_std(e)
						));
					}
				}
			}));

			let i = Literal::usize_unsuffixed(i + 1);

			call_route_args.push(quote!({
				let extract = #chuchi::extractor::Extract::new(
					prepared.#i, #name, &mut req, params, &state, resources
				);

				let res = <#ty as #extractor_type>::extract(
					extract
				);

				match res {
					Ok(res) => res,
					Err(err) => return Err(#chuchi::Error::new(
						#chuchi::extractor::ExtractorError::error_kind(&err),
						#chuchi::extractor::ExtractorError::into_std(err)
					))
				}
			}));
		}

		let process_ret_ty = match output {
			TransformOutput::No => quote!(
				#chuchi::into::IntoRouteResult::into_route_result(ret)
			),
			TransformOutput::Json => quote!(
				let ret = #chuchi::json::IntoRouteResult::into_route_result(ret)?;
				#chuchi::json::serialize_to_response(&ret)
			),
		};

		quote!(
			fn call<'a>(
				&'a self,
				#[allow(unused_mut)]
				mut req: &'a mut #chuchi::Request,
				params: &'a #chuchi::routes::PathParams,
				resources: &'a #chuchi::resources::Resources
			) -> #chuchi::util::PinnedFuture<'a, #chuchi::Result<#chuchi::Response>> {
				#route_fn

				#chuchi::util::PinnedFuture::new(async move {
					#[allow(unused_mut, dead_code)]
					let mut state = #chuchi::state::State::new();

					// prepare extractions
					let prepared = (0,// this is a placeholder
						#(#prepare_extractors),*
					);

					let mut req = Some(req);

					let ret = handle_route(
						#(#call_route_args),*
					)#await_kw;

					#process_ret_ty
				})
			}
		)
	};

	Ok(quote!(
		#struct_gen

		impl #chuchi::routes::Route for #struct_name {
			#valid_data_fn

			#path_fn

			#call_fn
		}
	))
}

pub(crate) fn generate_struct(item: &ItemFn) -> TokenStream {
	let struct_name = &item.sig.ident;
	let attrs = &item.attrs;
	let vis = &item.vis;

	quote!(
		#(#attrs)*
		#[allow(non_camel_case_types)]
		#vis struct #struct_name;
	)
}
