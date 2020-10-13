use proc_macro::TokenStream;
use syn::{
	export::quote::quote,
	parse::{Parse, ParseStream},
	parse_macro_input, ExprClosure,
};

struct ClosureAsPinnedBoxParser {
	block: ExprClosure,
}

impl Parse for ClosureAsPinnedBoxParser {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let block = input.parse()?;

		Ok(ClosureAsPinnedBoxParser { block })
	}
}

pub fn parse(input: TokenStream) -> TokenStream {
	let ClosureAsPinnedBoxParser { block } =
		parse_macro_input!(input as ClosureAsPinnedBoxParser);

	let body = block.body;
	let inputs = block.inputs.into_iter().collect::<Vec<_>>();

	let expanded = quote! {
		| #(#inputs), * | {
			Box::pin(async move #body)
		}
	};
	TokenStream::from(expanded)
}
