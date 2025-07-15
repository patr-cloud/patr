use proc_macro::TokenStream;
use quote::format_ident;
use syn::{Error, Fields, FieldsNamed, FieldsUnnamed, Ident, ItemStruct, parse_quote};

/// A macro to generate the same struct but with all fields optional.
/// This is useful for creating a struct that can be used to update an existing
/// struct, where all fields are optional.
/// ## Example usage:
/// ```rust
/// # use macros::generate_optional;
/// #[generate_optional]
/// pub struct User {
///     pub name: String,
///     pub age: u32,
/// }
/// ```
/// This will generate a struct `UserOptional` with all fields optional.
/// The generated struct will have the same fields as the original struct, but
/// all fields will be wrapped in `Option`. The generated struct will also
/// have a few utility methods, such as `any_field_set` to check if any field is
/// set, `all_fields_set` to check if all fields are set.
pub(crate) fn parse(args: TokenStream, input: TokenStream) -> TokenStream {
	if let Some(token) = args.into_iter().next() {
		return Error::new(
			token.span().into(),
			"this macro does not accept any arguments",
		)
		.into_compile_error()
		.into();
	}

	let mut input = syn::parse_macro_input!(input as ItemStruct);

	input.ident = format_ident!("{}Optional", input.ident);
	let ident = input.ident.clone();
	let mut fields = Vec::<Ident>::with_capacity(input.fields.len());
	let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

	input.fields = match input.fields {
		Fields::Named(fields_named) => Fields::Named(FieldsNamed {
			named: fields_named
				.named
				.into_iter()
				.enumerate()
				.map(|(index, mut field)| {
					let ty = field.ty;
					field.ty = parse_quote!(Option<#ty>);
					fields.push(
						field
							.ident
							.clone()
							.unwrap_or_else(|| format_ident!("{index}")),
					);
					field
				})
				.collect(),
			..fields_named
		}),
		Fields::Unnamed(fields_unnamed) => Fields::Unnamed(FieldsUnnamed {
			unnamed: fields_unnamed
				.unnamed
				.into_iter()
				.enumerate()
				.map(|(index, mut field)| {
					let ty = field.ty;
					field.ty = parse_quote!(Option<#ty>);
					fields.push(
						field
							.ident
							.clone()
							.unwrap_or_else(|| format_ident!("{index}")),
					);
					field
				})
				.collect(),
			..fields_unnamed
		}),
		Fields::Unit => Fields::Unit,
	};

	quote::quote! {
		#input

		impl #impl_generics #ident #ty_generics #where_clause {
			/// Checks if any field is set.
			pub fn any_field_set(&self) -> bool {
				#(self.#fields.is_some() || )* false
			}

			/// Checks if all fields are set.
			pub fn all_fields_set(&self) -> bool {
				#(self.#fields.is_some() && )* true
			}
		}
	}
	.into()
}
