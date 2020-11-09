use proc_macro::TokenStream;
use syn::{
	export::quote::quote,
	parse::{Parse, ParseStream},
	parse_macro_input,
	Expr,
	Ident,
	LitStr,
	Token,
};

struct QueryAsParser {
	ty_name: Ident,
	query: LitStr,
	params: Vec<Expr>,
}

impl Parse for QueryAsParser {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let ty_name = input.parse()?;
		input.parse::<Token![,]>()?;
		let query = input.parse()?;
		let params = if input.parse::<Token![,]>().is_ok() {
			input
				.parse_terminated::<Expr, Token![,]>(Expr::parse)?
				.into_iter()
				.collect()
		} else {
			vec![]
		};

		Ok(QueryAsParser {
			ty_name,
			query,
			params,
		})
	}
}

pub fn parse(input: TokenStream) -> TokenStream {
	let QueryAsParser {
		ty_name,
		query,
		params,
	} = parse_macro_input!(input as QueryAsParser);
	let mut simplified_query = query
		.value()
		.replace("\n", " ")
		.replace("\r", " ")
		.replace("\t", "  ");
	while simplified_query.contains("  ") {
		simplified_query = simplified_query.replace("  ", " ");
	}
	simplified_query = simplified_query.trim().to_string();

	let expanded = quote! {
		{
			log::info!(target: "api::queries", #simplified_query);
			sqlx::query_as!(#ty_name, #query, #(#params), *)
		}
	};
	TokenStream::from(expanded)
}
