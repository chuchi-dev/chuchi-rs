use crate::route::generate_struct;
use crate::util::{chuchi_crate, validate_inputs, validate_signature};
use crate::Args;

use proc_macro2::{Literal, TokenStream};
use quote::{format_ident, quote};
use syn::{ItemFn, Result};

pub(crate) fn expand(args: Args, item: ItemFn) -> Result<TokenStream> {
	let chuchi = chuchi_crate()?;

	validate_signature(&item.sig)?;

	// Box<Type>
	let inputs = validate_inputs(item.sig.inputs.iter())?;

	let struct_name = &item.sig.ident;
	let struct_gen = generate_struct(&item);

	let extractor_type =
		quote!(#chuchi::extractor::Extractor<#chuchi::ws::WebSocket>);

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

		quote!(
			fn path(&self) -> #chuchi::routes::RoutePath {
				#chuchi::routes::RoutePath {
					method: Some(#chuchi::header::Method::GET),
					path: #uri.into()
				}
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

	let call_fn = {
		let is_async = item.sig.asyncness.is_some();
		let await_kw = if is_async { quote!(.await) } else { quote!() };

		let mut handler_args_vars = vec![];
		let mut handler_args = vec![];
		let mut prepare_extractors = vec![];

		for (idx, (name, ty)) in inputs.iter().enumerate() {
			prepare_extractors.push(quote!({
				let prepare = #chuchi::extractor::Prepare::new(
					#name, &header, &params, &mut state, &resources
				);

				let res = <#ty as #extractor_type>::prepare(
					prepare
				).await;

				match res {
					Ok(res) => res,
					Err(e) => {
						return Some(Err(#chuchi::Error::new(
							#chuchi::extractor::ExtractorError::error_kind(&e),
							#chuchi::extractor::ExtractorError::into_std(e)
						)));
					}
				}
			}));

			let i = Literal::usize_unsuffixed(idx + 1);
			let var_name = format_ident!("handler_arg_{idx}");

			handler_args_vars.push(quote!(
				let #var_name = {
					let extract = #chuchi::extractor::Extract::new(
						prepared.#i, #name, &mut ws, &params, &state, &resources
					);

					let res = <#ty as #extractor_type>::extract(
						extract
					);

					match res {
						Ok(res) => res,
						Err(err) => {
							#chuchi::ws::util::log_extractor_error(err);
							return
						}
					}
				};
			));
			handler_args.push(quote!(#var_name));
		}

		quote!(
			fn call<'a>(
				&'a self,
				req: &'a mut #chuchi::routes::HyperRequest,
				address: std::net::SocketAddr,
				params: &'a #chuchi::routes::PathParams,
				resources: &'a #chuchi::resources::Resources
			) -> #chuchi::util::PinnedFuture<'a,
				Option<#chuchi::Result<#chuchi::Response>>
			> {
				#handler_fn

				#chuchi::util::PinnedFuture::new(async move {
					let upgrade = #chuchi::ws::util::upgrade(req);
					let (on_upgrade, ws_accept) = match upgrade {
						Ok(o) => o,
						Err(e) => return Some(Err(e))
					};

					let header = #chuchi::ws::util::hyper_req_to_header(req, address);
					let header = match header {
						Ok(h) => h,
						Err(e) => return Some(Err(e))
					};

					let resources = resources.clone();
					let params = params.clone();

					#[allow(unused_mut, dead_code)]
					let mut state = #chuchi::state::State::new();
					// prepare extractions
					let prepared = (0,// this is a placeholder
						#(#prepare_extractors),*
					);

					#chuchi::ws::util::spawn(async move {
						match on_upgrade.await {
							Ok(upgraded) => {
								let ws = #chuchi::ws::WebSocket::new(
									upgraded
								).await;
								let mut ws = Some(ws);

								#(#handler_args_vars)*

								let ret = handler(
									#(#handler_args),*
								)#await_kw;

								#chuchi::ws::util::log_websocket_return(ret);
							},
							Err(e) => #chuchi::ws::util::upgrade_error(e)
						}
					});

					Some(Ok(#chuchi::ws::util::switching_protocols(ws_accept)))
				})
			}
		)
	};

	Ok(quote!(
		#struct_gen

		impl #chuchi::routes::RawRoute for #struct_name {
			#valid_data_fn

			#path_fn

			#call_fn
		}
	))
}
