use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::format_ident;
use syn::{
	Error,
	Expr,
	Field,
	Fields,
	FieldsNamed,
	FieldsUnnamed,
	GenericArgument,
	Index,
	ItemStruct,
	LitStr,
	Meta,
	PathArguments,
	Token,
	Type,
	parse_quote,
	punctuated::Punctuated,
};

/// A macro to generate the same struct but with all fields optional.
/// This is useful for creating a struct that can be used to update an existing
/// struct, where all fields are optional.
/// ## Example usage:
/// ```rust
/// # use macros::optionalize;
/// #[optionalize]
/// pub struct User {
///     pub name: String,
///     pub age: u32,
/// }
/// ```
/// This will generate a struct `UserOptional` with all fields optional.
/// The generated struct will have the same fields as the original struct, but
/// all fields will be wrapped in `Option`. The generated struct will also
/// have a few utility methods, such as `any_field_set` to check if any field is
/// set, `all_fields_set` to check if all fields are set, and an implementation
/// of `models::utils::Optionalizable` for the original struct.
///
/// You can skip a field in the generated optional struct by annotating it with
/// `#[optionalize(skip)]`.
/// You can keep an already-optional field unchanged (avoid `Option<Option<T>>`)
/// by annotating it with `#[optionalize(keep)]`.
///
/// Note: place `#[optionalize]` before other active struct attributes like
/// `#[derive(...)]` if you want them to also apply to the generated
/// `*Optional` struct.
pub(crate) fn parse(args: TokenStream, input: TokenStream) -> TokenStream {
	if let Some(token) = args.into_iter().next() {
		return Error::new(
			token.span().into(),
			"this macro does not accept any arguments",
		)
		.into_compile_error()
		.into();
	}

	let result = (|| -> syn::Result<TokenStream2> {
		let mut input = syn::parse::<ItemStruct>(input)?;
		let mut field_accessors = Vec::<TokenStream2>::with_capacity(input.fields.len());

		let optional_fields = match &mut input.fields {
			Fields::Named(fields_named) => {
				let mut named = Punctuated::new();

				for field in fields_named.named.iter_mut() {
					let action = parse_field_attribute(field)?;
					if action == FieldAction::Skip {
						continue;
					}

					let mut optional_field = field.clone();
					match action {
						FieldAction::None => {
							let ty = optional_field.ty.clone();
							optional_field.ty = parse_quote!(Option<#ty>);
							rewrite_serde_skip_serializing_if_for_optional(&mut optional_field)?;
						}
						FieldAction::Keep => {
							validate_keep_attribute_target(&optional_field.ty)?;
						}
						FieldAction::Skip => {}
					}

					let field_ident = optional_field
						.ident
						.clone()
						.expect("named fields always have an identifier");
					field_accessors.push(quote::quote!(self.#field_ident));
					named.push(optional_field);
				}

				Fields::Named(FieldsNamed {
					brace_token: fields_named.brace_token,
					named,
				})
			}
			Fields::Unnamed(fields_unnamed) => {
				let mut unnamed = Punctuated::new();

				for field in fields_unnamed.unnamed.iter_mut() {
					let action = parse_field_attribute(field)?;
					if action == FieldAction::Skip {
						continue;
					}

					let mut optional_field = field.clone();
					match action {
						FieldAction::None => {
							let ty = optional_field.ty.clone();
							optional_field.ty = parse_quote!(Option<#ty>);
							rewrite_serde_skip_serializing_if_for_optional(&mut optional_field)?;
						}
						FieldAction::Keep => {
							validate_keep_attribute_target(&optional_field.ty)?;
						}
						FieldAction::Skip => {}
					}
					unnamed.push(optional_field);

					let optional_index = Index::from(unnamed.len() - 1);
					field_accessors.push(quote::quote!(self.#optional_index));
				}

				Fields::Unnamed(FieldsUnnamed {
					paren_token: fields_unnamed.paren_token,
					unnamed,
				})
			}
			Fields::Unit => Fields::Unit,
		};

		let mut optional_struct = input.clone();
		optional_struct.ident = format_ident!("{}Optional", input.ident);
		optional_struct.fields = optional_fields;

		let original_ident = input.ident.clone();
		let optional_ident = optional_struct.ident.clone();
		let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

		Ok(quote::quote! {
			#input

			#optional_struct

			impl #impl_generics ::models::utils::Optionalizable for #original_ident #ty_generics #where_clause {
				type Optionalized = #optional_ident #ty_generics;
			}

			impl #impl_generics #optional_ident #ty_generics #where_clause {
				/// Checks if any field is set.
				pub fn any_field_set(&self) -> bool {
					#(#field_accessors.is_some() || )* false
				}

				/// Checks if all fields are set.
				pub fn all_fields_set(&self) -> bool {
					#(#field_accessors.is_some() && )* true
				}
			}
		})
	})();

	match result {
		Ok(tokens) => tokens.into(),
		Err(error) => error.into_compile_error().into(),
	}
}

fn parse_field_attribute(field: &mut Field) -> syn::Result<FieldAction> {
	let mut action = FieldAction::None;
	let mut attrs = Vec::with_capacity(field.attrs.len());

	for attr in std::mem::take(&mut field.attrs) {
		if !attr.path().is_ident("optionalize") {
			attrs.push(attr);
			continue;
		}

		let Meta::List(meta_list) = &attr.meta else {
			return Err(Error::new_spanned(
				attr,
				"expected `#[optionalize(skip)]` or `#[optionalize(keep)]`",
			));
		};

		let mut parsed_action = FieldAction::None;
		meta_list.parse_nested_meta(|meta| {
			if meta.path.is_ident("skip") {
				if parsed_action != FieldAction::None {
					return Err(meta.error("expected a single argument: `skip` or `keep`"));
				}

				parsed_action = FieldAction::Skip;
				return Ok(());
			}

			if meta.path.is_ident("keep") {
				if parsed_action != FieldAction::None {
					return Err(meta.error("expected a single argument: `skip` or `keep`"));
				}

				parsed_action = FieldAction::Keep;
				return Ok(());
			}

			Err(meta.error("unsupported argument, expected `skip` or `keep`"))
		})?;

		if parsed_action == FieldAction::None {
			return Err(Error::new_spanned(
				meta_list,
				"expected `#[optionalize(skip)]` or `#[optionalize(keep)]`",
			));
		}

		if action != FieldAction::None {
			return Err(Error::new_spanned(
				attr,
				"duplicate `optionalize(...)` attribute on field",
			));
		}

		action = parsed_action;
	}

	field.attrs = attrs;
	Ok(action)
}

fn validate_keep_attribute_target(ty: &Type) -> syn::Result<()> {
	if is_option_type(ty) {
		return Ok(());
	}

	Err(Error::new_spanned(
		ty,
		"`#[optionalize(keep)]` is only allowed on fields of type `Option<T>`",
	))
}

fn rewrite_serde_skip_serializing_if_for_optional(field: &mut Field) -> syn::Result<()> {
	let mut attrs = Vec::with_capacity(field.attrs.len());

	for attr in std::mem::take(&mut field.attrs) {
		if !attr.path().is_ident("serde") {
			attrs.push(attr);
			continue;
		}

		let Meta::List(meta_list) = &attr.meta else {
			attrs.push(attr);
			continue;
		};

		let mut args = Vec::<TokenStream2>::new();
		let mut has_skip_serializing_if = false;

		meta_list.parse_nested_meta(|meta| {
			let path = meta.path.clone();

			if path.is_ident("skip_serializing_if") {
				has_skip_serializing_if = true;
				if meta.input.peek(Token![=]) {
					let _: Token![=] = meta.input.parse()?;
					let _: LitStr = meta.input.parse()?;
					return Ok(());
				}

				return Err(meta.error("expected `skip_serializing_if = \"...\"`"));
			}

			if meta.input.peek(Token![=]) {
				let _: Token![=] = meta.input.parse()?;
				let expr: Expr = meta.input.parse()?;
				args.push(quote::quote!(#path = #expr));
				return Ok(());
			}

			if meta.input.peek(syn::token::Paren) {
				let content;
				syn::parenthesized!(content in meta.input);
				let inner: TokenStream2 = content.parse()?;
				args.push(quote::quote!(#path(#inner)));
				return Ok(());
			}

			args.push(quote::quote!(#path));
			Ok(())
		})?;

		if has_skip_serializing_if {
			args.push(quote::quote!(skip_serializing_if = "Option::is_none"));
		} else {
			attrs.push(attr);
		}
	}

	field.attrs = attrs;
	Ok(())
}

fn is_option_type(ty: &Type) -> bool {
	let Type::Path(type_path) = ty else {
		return false;
	};

	if type_path.qself.is_some() {
		return false;
	}

	let Some(segment) = type_path.path.segments.last() else {
		return false;
	};

	if segment.ident != "Option" {
		return false;
	}

	let PathArguments::AngleBracketed(args) = &segment.arguments else {
		return false;
	};

	args.args
		.iter()
		.any(|argument| matches!(argument, GenericArgument::Type(_)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldAction {
	None,
	Skip,
	Keep,
}
