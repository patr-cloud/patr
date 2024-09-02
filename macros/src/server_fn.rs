use proc_macro::TokenStream;

/// An attribute that expands to other attributes for a server fn, adding
/// middlewares to it.
pub fn parse(args: TokenStream, _input: TokenStream) -> TokenStream {
	if let Some(arg) = args.into_iter().next() {
		return syn::Error::new(arg.span().into(), "this macro takes no arguments")
			.to_compile_error()
			.into();
	}

	todo!()
}
