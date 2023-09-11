use proc_macro::TokenStream;
use quote::quote;
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	Expr,
	LitStr,
	Token,
};

struct QueryParser {
	query: LitStr,
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

pub fn parse(input: TokenStream) -> TokenStream {
	let QueryParser { query, params } =
		parse_macro_input!(input as QueryParser);
	let mut simplified_query =
		query.value().replace(['\n', '\r'], " ").replace('\t', "  ");
	while simplified_query.contains("  ") {
		simplified_query = simplified_query.replace("  ", " ");
	}
	simplified_query = simplified_query.trim().to_string();

	let expanded = quote! {
		{
			crate::prelude::info!(target: "api::queries", "{}", #simplified_query);
			sqlx::query!(#simplified_query, #(#params), *)
		}
	};
	TokenStream::from(expanded)
}
