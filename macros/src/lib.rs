use std::fmt::Display;

use proc_macro::TokenStream;
use quote::quote;

mod closure_as_pinned_box;
mod config;
mod email_template;
mod iterable_module;
mod migrate_query;
mod migrate_query_as;
mod query;
mod query_as;
mod render;
mod settings_component;
mod version;

#[proc_macro]
pub fn query(input: TokenStream) -> TokenStream {
	query::parse(input)
}

#[proc_macro]
pub fn migrate_query(input: TokenStream) -> TokenStream {
	migrate_query::parse(input)
}

#[proc_macro]
pub fn migrate_query_as(input: TokenStream) -> TokenStream {
	migrate_query_as::parse(input)
}

#[proc_macro]
pub fn query_as(input: TokenStream) -> TokenStream {
	query_as::parse(input)
}

#[proc_macro_derive(EmailTemplate, attributes(template_path))]
pub fn email_template(input: TokenStream) -> TokenStream {
	email_template::parse(input)
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

#[proc_macro_attribute]
pub fn iterable_module(args: TokenStream, input: TokenStream) -> TokenStream {
	iterable_module::parse(args, input)
}

#[proc_macro_derive(SettingsComponent)]
pub fn settings_component(input: TokenStream) -> TokenStream {
	settings_component::parse(input)
}

#[proc_macro]
pub fn version(input: TokenStream) -> TokenStream {
	version::parse(input)
}

fn compiler_error<TDisplay: Display>(message: TDisplay) -> TokenStream {
	let message = message.to_string();
	TokenStream::from(quote! {
		compile_error!(#message);
	})
}
