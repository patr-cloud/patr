use proc_macro::TokenStream;
use quote::quote;
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	Expr,
	LitStr,
	Token,
};

/// A SQL query and a list of parameters to pass to it.
struct QueryParser {
	/// The SQL query to run.
	query: LitStr,
	/// A list of parameters to pass to the query.
	params: Vec<Expr>,
}

impl Parse for QueryParser {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let query = input.parse()?;
		let params = if input.parse::<Token![,]>().is_ok() {
			input
				.parse_terminated(Expr::parse, Token![,])?
				.into_iter()
				.collect()
		} else {
			vec![]
		};

		Ok(QueryParser { query, params })
	}
}

/// Runs an SQL query without the spaces and newlines and logs it. This is
/// mostly just a wrapper around [`sqlx::query!`].
pub fn parse(input: TokenStream) -> TokenStream {
	let QueryParser { query, params } = parse_macro_input!(input as QueryParser);
	let simplified_query = query
		.value()
		.lines()
		.map(|line| line.trim().to_string())
		.collect::<Vec<_>>()
		.join(" ")
		.replace(" )", ")")
		.replace("( ", "(")
		.trim()
		.to_string();

	let expanded = quote! {
		{
			crate::prelude::info!(target: "api::queries", "{}", #simplified_query);
			sqlx::query!(#simplified_query, #(#params), *)
		}
	};
	TokenStream::from(expanded)
}
