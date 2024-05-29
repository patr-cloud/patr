use proc_macro::TokenStream;
use regex::Regex;
use syn::{parse::Parse, LitStr};

/// Input for the `verify_regex` macro.
struct VerifyRegexInput {
	/// The regex to verify.
	regex: LitStr,
}

impl Parse for VerifyRegexInput {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		Ok(Self {
			regex: input.parse()?,
		})
	}
}

/// Verifies that the input is a valid regex.
pub fn parse(input: TokenStream) -> TokenStream {
	let VerifyRegexInput { regex } = syn::parse_macro_input!(input as VerifyRegexInput);
	match Regex::new(&regex.value()) {
		Ok(_) => quote::quote! {
			#regex
		},
		Err(e) => syn::Error::new(regex.span(), e.to_string()).to_compile_error(),
	}
	.into()
}
