

use proc_macro::TokenStream;

mod api_endpoint;

#[proc_macro]
pub fn classes(input: TokenStream) -> TokenStream {
	input
}

#[proc_macro]
pub fn api_endpoint(input: TokenStream) -> TokenStream {
	api_endpoint::parse(input)
}