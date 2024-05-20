use proc_macro::TokenStream;
use regex::Regex;
use syn::{parse::Parse, LitStr};

struct VerifyRegexInput {
	regex: LitStr,
}

impl Parse for VerifyRegexInput {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		Ok(Self {
			regex: input.parse()?,
		})
	}
}

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
