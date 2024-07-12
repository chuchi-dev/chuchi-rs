use syn::parse::{Parse, ParseStream, Result};
use syn::LitStr;

#[cfg(feature = "api")]
pub(crate) use api::*;

#[derive(Debug, Clone)]
pub(crate) struct Args {
	pub uri: String,
}

impl Parse for Args {
	fn parse(input: ParseStream) -> Result<Self> {
		// parse a string
		let uri: LitStr = input.parse()?;

		Ok(Self { uri: uri.value() })
	}
}

#[cfg(feature = "api")]
mod api {
	use super::*;

	use syn::{Ident, LitBool, Token, Type};

	#[derive(Clone)]
	pub(crate) struct ApiArgs {
		pub ty: Type,
		pub impl_extractor: bool,
	}

	impl Parse for ApiArgs {
		fn parse(input: ParseStream) -> Result<Self> {
			// parse a string
			let ty: Type = input.parse()?;

			// might contain a comma
			let impl_extractor = if input.is_empty() {
				true
			} else {
				input.parse::<Token![,]>()?;
				let ident = input.parse::<Ident>()?;
				let _eq = input.parse::<Token![=]>()?;
				let value = input.parse::<LitBool>()?;

				if ident != "impl_extractor" {
					return Err(input.error("expected `impl_extractor`"));
				}

				value.value
			};

			// make sure we parsed everything
			if !input.is_empty() {
				return Err(input.error("unexpected token"));
			}

			Ok(Self { ty, impl_extractor })
		}
	}
}
