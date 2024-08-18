/*
Expose

req,
header,
ResponseSettings,
Resources,

*/

use crate::request_extractor::impl_extractor;
use crate::route::generate_struct;
use crate::util::{chuchi_crate, validate_inputs, validate_signature};
use crate::ApiArgs;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemFn, Result};

pub(crate) fn expand(args: ApiArgs, item: ItemFn) -> Result<TokenStream> {
	let chuchi = chuchi_crate()?;
	let chuchi_api = quote!(#chuchi::api);
	let req_ty = args.ty;

	validate_signature(&item.sig)?;

	// implement Extractor for req_ty

	let impl_extractor = if !args.impl_extractor {
		quote!()
	} else {
		impl_extractor(&chuchi, &quote!(#req_ty))
	};

	// Box<Type>
	let inputs = validate_inputs(item.sig.inputs.iter())?;

	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	//
	let ty_as_req = quote!(<#req_ty as #chuchi_api::Request>);
	let extractor_type = quote!(#chuchi::extractor::Extractor<#req_ty>);

	let valid_data_fn = {
		let mut asserts = vec![];

		for (name, attrs, ty) in &inputs {
			asserts.push(quote!(
				#(#attrs)*
				{
					let validate = #chuchi::extractor::Validate::new(
						#name, params, &mut state, resources
					);

					<#ty as #extractor_type>::validate(
						validate
					);
				}
			));
		}

		quote!(
			fn validate_requirements(
				&self,
				params: &#chuchi::routes::ParamsNames,
				resources: &#chuchi::resources::Resources
			) {
				#[allow(unused_mut, dead_code)]
				let mut state = #chuchi::state::StateValidation::new();
				state.insert::<#chuchi::state::StateRefCell<
					#chuchi_api::response::ResponseSettings
				>>();

				#(#asserts)*

				#chuchi_api::util::validate_request::<#req_ty>(stringify!(#req_ty));
			}
		)
	};

	// path fn
	let path_fn = quote!(
		fn path(&self) -> #chuchi::routes::RoutePath {
			#chuchi::routes::RoutePath {
				method: Some(#ty_as_req::METHOD),
				path: #ty_as_req::PATH.into()
			}
		}
	);

	let handler_fn = {
		let asyncness = &item.sig.asyncness;
		let inputs = &item.sig.inputs;
		let output = &item.sig.output;
		let block = &item.block;

		quote!(
			#asyncness fn handler( #inputs ) #output
				#block
		)
	};

	let call_fn = {
		let is_async = item.sig.asyncness.is_some();
		let await_kw = if is_async { quote!(.await) } else { quote!() };

		let mut handler_args_vars = vec![];
		let mut handler_args = vec![];
		let mut prepare_extractors = vec![];

		for (idx, (name, attrs, ty)) in inputs.iter().enumerate() {
			prepare_extractors.push(quote!(
				#(#attrs)*
				{
					let prepare = #chuchi::extractor::Prepare::new(
						#name, header, params, &mut state, resources
					);

					let res = <#ty as #extractor_type>::prepare(
						prepare
					).await;

					match res {
						Ok(res) => res,
						Err(e) => {
							return Err(#chuchi_api::util::extraction_error::<#req_ty>(e));
						}
					}
				}
			));

			let i = Literal::usize_unsuffixed(idx + 1);
			let var_name = format_ident!("handler_arg_{idx}");

			handler_args_vars.push(quote!(
				#(#attrs)*
				let #var_name = {
					let extract = #chuchi::extractor::Extract::new(
						prepared.#i, #name, &mut req, &params, &state, &resources
					);

					let res = <#ty as #extractor_type>::extract(
						extract
					);

					match res {
						Ok(res) => res,
						Err(e) => {
							return Err(#chuchi_api::util::extraction_error::<#req_ty>(e));
						}
					}
				};
			));
			handler_args.push(quote!(#(#attrs)* #var_name));
		}

		quote!(
			fn call<'a>(
				&'a self,
				req: &'a mut #chuchi::Request,
				params: &'a #chuchi::routes::PathParams,
				resources: &'a #chuchi::resources::Resources
			) -> #chuchi::util::PinnedFuture<'a, #chuchi::Result<#chuchi::Response>> {
				#handler_fn

				type __Response = #ty_as_req::Response;
				type __Error = #ty_as_req::Error;

				async fn route_to_body(
					fire_req: &mut #chuchi::Request,
					params: &#chuchi::routes::PathParams,
					resources: &#chuchi::resources::Resources
				) -> std::result::Result<(
					#chuchi_api::response::ResponseSettings,
					#chuchi::Body
				), __Error> {
					#chuchi_api::util::setup_request::<#req_ty>(fire_req)?;

					let req = #chuchi_api::util::deserialize_req::<#req_ty>(
						fire_req
					).await?;

					#[allow(unused_mut, dead_code)]
					let mut state = #chuchi::state::State::new();
					state.insert(#chuchi_api::response::ResponseSettings::new_for_state());

					let header = fire_req.header();

					// prepare extractions
					let prepared = (0,// this is a placeholder
						#(#prepare_extractors),*
					);

					let mut req = Some(req);

					#(#handler_args_vars)*

					let resp: __Response = handler(
							#(#handler_args),*
					)#await_kw?;

					let resp_header = state.remove::<
						#chuchi::state::StateRefCell<#chuchi_api::response::ResponseSettings>
					>().unwrap();

					#chuchi_api::util::serialize_resp::<#req_ty>(&resp)
						.map(|body| (resp_header.into_inner(), body))
				}

				#chuchi::util::PinnedFuture::new(async move {
					#chuchi_api::util::transform_body_to_response::<#req_ty>(
						route_to_body(req, params, resources).await
					)
				})
			}
		)
	};

	Ok(quote!(
		#impl_extractor

		#struct_gen

		impl #chuchi::routes::Route for #struct_name {
			#valid_data_fn

			#path_fn

			#call_fn
		}
	))
}
