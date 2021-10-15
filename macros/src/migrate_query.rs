use proc_macro::TokenStream;
use quote::quote;
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	Expr,
	LitStr,
	Token,
};

struct MigrateQueryParser {
	query: LitStr,
	params: Vec<Expr>,
}

impl Parse for MigrateQueryParser {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let query = input.parse()?;
		let params = if input.parse::<Token![,]>().is_ok() {
			input
				.parse_terminated::<Expr, Token![,]>(Expr::parse)?
				.into_iter()
				.collect()
		} else {
			vec![]
		};

		Ok(MigrateQueryParser { query, params })
	}
}

pub fn parse(input: TokenStream) -> TokenStream {
	let MigrateQueryParser { query, params } =
		parse_macro_input!(input as MigrateQueryParser);
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
			log::info!(target: "api::queries::migrations", #simplified_query);
			sqlx::query(#simplified_query)
				#(.bind(#params)) *
		}
	};
	TokenStream::from(expanded)
}
