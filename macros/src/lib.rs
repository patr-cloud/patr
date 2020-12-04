use proc_macro::TokenStream;

extern crate proc_macro;
extern crate serde;
extern crate serde_json;
extern crate syn;

mod closure_as_pinned_box;
mod config;
mod email_template;
mod iterable_module;
mod query;
mod query_as;
mod render;

#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
	query::parse(input)
}

#[proc_macro]
pub fn query_as(input: TokenStream) -> TokenStream {
	query_as::parse(input)
}

#[proc_macro]
pub fn email_template(input: TokenStream) -> TokenStream {
	email_template::parse(input)
}

#[proc_macro_attribute]
pub fn iterable_module(args: TokenStream, input: TokenStream) -> TokenStream {
	iterable_module::parse(args, input)
}

#[proc_macro]
pub fn closure_as_pinned_box(input: TokenStream) -> TokenStream {
	closure_as_pinned_box::parse(input)
}

#[proc_macro]
pub fn render(input: TokenStream) -> TokenStream {
	render::parse(input)
}

#[proc_macro]
pub fn config(input: TokenStream) -> TokenStream {
	config::parse(input)
}
