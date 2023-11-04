use std::env;

use proc_macro::TokenStream;
use syn::Error;

/// Parses the current crate version and returns it as a [`semver::Version`]
pub fn parse(input: TokenStream) -> TokenStream {
	if let Some(item) = input.into_iter().next() {
		return Error::new(item.span().into(), "expected no arguments")
			.to_compile_error()
			.into();
	}

	let Ok(major) = env::var("CARGO_PKG_VERSION_MAJOR") else {
		return quote::quote!(
			compiler_error!("CARGO_PKG_VERSION_MAJOR is not set");
		)
		.into();
	};
	let Ok(minor) = env::var("CARGO_PKG_VERSION_MINOR") else {
		return quote::quote!(
			compiler_error!("CARGO_PKG_VERSION_MINOR is not set");
		)
		.into();
	};
	let Ok(patch) = env::var("CARGO_PKG_VERSION_PATCH") else {
		return quote::quote!(
			compiler_error!("CARGO_PKG_VERSION_PATCH is not set");
		)
		.into();
	};

	let Ok(major) = major.parse::<u64>() else {
		return quote::quote!(
			compiler_error!("CARGO_PKG_VERSION_MAJOR is not a valid integer");
		)
		.into();
	};
	let Ok(minor) = minor.parse::<u64>() else {
		return quote::quote!(
			compiler_error!("CARGO_PKG_VERSION_MINOR is not a valid integer");
		)
		.into();
	};
	let Ok(patch) = patch.parse::<u64>() else {
		return quote::quote!(
			compiler_error!("CARGO_PKG_VERSION_PATCH is not a valid integer");
		)
		.into();
	};

	quote::quote! {
		::semver::Version::new(#major, #minor, #patch)
	}
	.into()
}
