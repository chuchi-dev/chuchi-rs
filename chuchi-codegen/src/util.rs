use proc_macro2::{Ident, Span, TokenStream};
use quote::quote;
use syn::{
	punctuated, Attribute, Error, FnArg, Pat, Result, Signature, Type,
	TypeReference,
};

use proc_macro_crate::{crate_name, FoundCrate};

pub(crate) fn validate_signature(sig: &Signature) -> Result<()> {
	if let Some(con) = &sig.constness {
		return Err(Error::new(con.span, "const not allowed"));
	}

	if let Some(unsf) = &sig.unsafety {
		return Err(Error::new(unsf.span, "unsafe not allowed"));
	}

	if let Some(abi) = &sig.abi {
		return Err(Error::new_spanned(abi, "abi not allowed"));
	}

	if !sig.generics.params.is_empty() {
		return Err(Error::new_spanned(&sig.generics, "generics not allowed"));
	}

	if let Some(variadic) = &sig.variadic {
		return Err(Error::new_spanned(variadic, "variadic not allowed"));
	}

	Ok(())
}

#[allow(dead_code)]
pub(crate) fn ref_type(ty: &Type) -> Option<&TypeReference> {
	match ty {
		Type::Reference(r) => Some(r),
		_ => None,
	}
}

fn name_from_pattern(pat: &Pat) -> Option<String> {
	match pat {
		Pat::Ident(ident) => Some(ident.ident.to_string()),
		_ => None,
	}
}

#[allow(dead_code)]
#[allow(clippy::type_complexity)]
pub(crate) fn validate_inputs(
	inputs: punctuated::Iter<'_, FnArg>,
) -> Result<Vec<(String, Vec<Attribute>, Box<Type>)>> {
	let mut v = vec![];

	for fn_arg in inputs {
		let (name, attrs, ty) = match fn_arg {
			FnArg::Receiver(r) => {
				return Err(Error::new_spanned(r, "self not allowed"))
			}
			FnArg::Typed(t) => (
				name_from_pattern(&t.pat).unwrap_or_default(),
				t.attrs.clone(),
				t.ty.clone(),
			),
		};

		if let Some(reff) = ref_type(&ty) {
			if let Some(lifetime) = &reff.lifetime {
				return Err(Error::new_spanned(
					lifetime,
					"lifetimes not \
					supported",
				));
			}
		}

		v.push((name, attrs, ty));
	}

	Ok(v)
}

pub(crate) fn chuchi_crate() -> Result<TokenStream> {
	let name =
		crate_name("chuchi").map_err(|e| Error::new(Span::call_site(), e))?;

	Ok(match name {
		// if it get's used inside chuchi it is a test or an example
		FoundCrate::Itself => quote!(chuchi),
		FoundCrate::Name(n) => {
			let ident = Ident::new(&n, Span::call_site());
			quote!(#ident)
		}
	})
}
