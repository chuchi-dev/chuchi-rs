use syn::{DeriveInput, Error};

use ::quote::quote;

use crate::util::chuchi_crate;

type Result<T> = std::result::Result<T, Error>;

pub fn expand(input: &DeriveInput) -> Result<proc_macro::TokenStream> {
	let chuchi = chuchi_crate()?;

	let ty = &input.ident;

	let (_, ty_generics, _) = input.generics.split_for_impl();
	let ty = quote!(#ty #ty_generics);

	let im = quote!(
		impl<'a, R> #chuchi::extractor::Extractor<'a, R> for &'a #ty {
			type Error = std::convert::Infallible;
			type Prepared = ();

			fn validate(validate: #chuchi::extractor::Validate<'_>) {
				assert!(
					validate.resources.exists::<#ty>(),
					"Resource {} does not exist",
					stringify!(#ty)
				);
			}

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
				extract: #chuchi::extractor::Extract<'a, '_, Self::Prepared, R>,
			) -> std::result::Result<Self, Self::Error>
			where
				Self: Sized,
			{
				Ok(extract.resources.get::<#ty>().unwrap())
			}
		}
	);

	Ok(im.into())
}
