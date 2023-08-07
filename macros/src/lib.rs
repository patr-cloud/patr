use proc_macro::TokenStream;

mod declare_api_endpoint;
mod has_headers;

#[proc_macro]
pub fn classes(input: TokenStream) -> TokenStream {
	input
}

#[proc_macro]
pub fn declare_api_endpoint(input: TokenStream) -> TokenStream {
	declare_api_endpoint::parse(input)
}

#[proc_macro_derive(HasHeaders)]
pub fn has_headers(input: TokenStream) -> TokenStream {
	has_headers::parse(input)
}
