use proc_macro2::TokenStream;
use syn::{DeriveInput, Error};

use ::quote::quote;

use crate::util::chuchi_crate;

type Result<T> = std::result::Result<T, Error>;

pub fn expand(input: &DeriveInput) -> Result<proc_macro::TokenStream> {
	let chuchi = chuchi_crate()?;

	let ty = &input.ident;

	let (_, ty_generics, _) = input.generics.split_for_impl();

	Ok(impl_extractor(&chuchi, &quote!(#ty #ty_generics)).into())
}

pub fn impl_extractor(chuchi: &TokenStream, ty: &TokenStream) -> TokenStream {
	quote!(
		impl<'a> #chuchi::extractor::Extractor<'a, #ty> for #ty {
			type Error = std::convert::Infallible;
			type Prepared = ();

			fn validate(_validate: #chuchi::extractor::Validate<'_>) {}

			fn prepare(
				_prepare: #chuchi::extractor::Prepare<'_>,
			) -> std::pin::Pin<
				Box<
					dyn std::future::Future<
						Output = std::result::Result<Self::Prepared, Self::Error>,
					> + Send,
				>,
			> {
				Box::pin(std::future::ready(Ok(())))
			}

			fn extract(
				extract: #chuchi::extractor::Extract<'a, '_, Self::Prepared, #ty>,
			) -> std::result::Result<Self, Self::Error>
			where
				Self: Sized,
			{
				Ok(extract.request.take().unwrap())
			}
		}
	)
}
