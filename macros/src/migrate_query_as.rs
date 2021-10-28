use proc_macro::TokenStream;
use quote::quote;
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	Expr,
	Ident,
	LitStr,
	Token,
};

struct MigrateQueryAsParser {
	ty_name: Ident,
	query: LitStr,
	params: Vec<Expr>,
}

impl Parse for MigrateQueryAsParser {
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

		Ok(MigrateQueryAsParser {
			ty_name,
			query,
			params,
		})
	}
}

pub fn parse(input: TokenStream) -> TokenStream {
	let MigrateQueryAsParser {
		ty_name,
		query,
		params,
	} = parse_macro_input!(input as MigrateQueryAsParser);
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
			sqlx::query_as::<_, #ty_name>(#simplified_query)
				#(.bind(#params)) *
		}
	};
	TokenStream::from(expanded)
}
