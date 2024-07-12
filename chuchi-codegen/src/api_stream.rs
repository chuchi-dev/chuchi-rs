/*
Expose

req,
streamer
Resources,

*/

use crate::request_extractor::impl_extractor;
use crate::route::generate_struct;
use crate::util::{fire_api_crate, validate_inputs, validate_signature};
use crate::ApiArgs;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemFn, Result};

pub(crate) fn expand(args: ApiArgs, item: ItemFn) -> Result<TokenStream> {
	let fire_api = fire_api_crate()?;
	let fire = quote!(#fire_api::fire);
	let stream_mod = quote!(#fire_api::stream);
	let stream_ty = args.ty;

	validate_signature(&item.sig)?;

	// implement Extractor for req_ty
	let impl_extractor = if !args.impl_extractor {
		quote!()
	} else {
		impl_extractor(&fire, &quote!(#stream_ty))
	};

	// Box<Type>
	let inputs = validate_inputs(item.sig.inputs.iter())?;

	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	//
	let ty_as_stream = quote!(<#stream_ty as #stream_mod::Stream>);
	let extractor_type = quote!(#fire::extractor::Extractor<#stream_ty>);

	let into_stream_impl = quote!(
		impl #stream_mod::server::IntoStreamHandler for #struct_name {
			type Stream = #stream_ty;
			type Handler = #struct_name;

			fn into_handler(self) -> Self::Handler { self }
		}
	);

	let valid_data_fn = {
		let mut asserts = vec![];

		for (name, ty) in &inputs {
			asserts.push(quote!({
				let validate = #fire::extractor::Validate::new(
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
				params: &#fire::routes::ParamsNames,
				resources: &#fire::resources::Resources
			) {
				#[allow(unused_mut, dead_code)]
				let mut state = #fire::state::StateValidation::new();
				state.insert::<#stream_ty>();

				#(#asserts)*
			}
		)
	};

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

	let handle_fn = {
		let is_async = item.sig.asyncness.is_some();
		let await_kw = if is_async { quote!(.await) } else { quote!() };

		let mut handler_args_vars = vec![];
		let mut handler_args = vec![];
		let mut prepare_extractors = vec![];

		for (idx, (name, ty)) in inputs.iter().enumerate() {
			prepare_extractors.push(quote!({
				let prepare = #fire::extractor::Prepare::new(
					#name, header, params, &mut state, resources
				);

				let res = <#ty as #extractor_type>::prepare(
					prepare
				).await;

				match res {
					Ok(res) => res,
					Err(e) => {
						return Err(#stream_mod::util::extraction_error::<#stream_ty>(e));
					}
				}
			}));

			let i = Literal::usize_unsuffixed(idx + 1);
			let var_name = format_ident!("handler_arg_{idx}");

			handler_args_vars.push(quote!(
				let #var_name = {
					let extract = #fire::extractor::Extract::new(
						prepared.#i, #name, &mut req, &params, &state, &resources
					);

					let res = <#ty as #extractor_type>::extract(
						extract
					);

					match res {
						Ok(res) => res,
						Err(e) => {
							return Err(#stream_mod::util::extraction_error::<#stream_ty>(e));
						}
					}
				};
			));
			handler_args.push(quote!(#var_name));
		}

		quote!(
			fn handle<'a>(
				&'a self,
				req: #stream_mod::message::MessageData,
				header: &'a #fire::header::RequestHeader,
				params: &'a #fire::routes::PathParams,
				streamer: #stream_mod::streamer::RawStreamer,
				resources: &'a #fire::resources::Resources
			) -> #stream_mod::server::PinnedFuture<'a, std::result::Result<
				#stream_mod::message::MessageData,
				#stream_mod::error::UnrecoverableError
			>> {
				#handler_fn

				type __Error = #ty_as_stream::Error;

				async fn _handle(
					streamer: #stream_mod::streamer::RawStreamer,
					req: #stream_ty,
					header: &#fire::header::RequestHeader,
					params: &#fire::routes::PathParams,
					resources: &#fire::resources::Resources
				) -> std::result::Result<(), __Error> {
					// transform streamer
					let streamer = #stream_mod::util::transform_streamer
						::<#stream_ty>(streamer);

					#[allow(unused_mut, dead_code)]
					let mut state = #fire::state::State::new();
					state.insert(streamer);

					// prepare extractions
					let prepared = (0,// this is a placeholder
						#(#prepare_extractors),*
					);

					let mut req = Some(req);

					#(#handler_args_vars)*

					handler(
						#(#handler_args),*
					)#await_kw
				}

				#stream_mod::server::PinnedFuture::new(async move {
					let req = #stream_mod::util::deserialize_req(req)?;

					let r = _handle(streamer, req, header, params, resources).await;
					#stream_mod::util::error_to_data::<#stream_ty>(r)
				})
			}
		)
	};

	Ok(quote!(
		#impl_extractor

		#struct_gen

		#into_stream_impl

		impl #stream_mod::server::StreamHandler for #struct_name {
			#valid_data_fn

			#handle_fn
		}
	))
}
