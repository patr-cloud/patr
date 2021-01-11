use std::collections::HashMap;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, Type, Visibility};

pub fn parse(input: TokenStream) -> TokenStream {
	let input: DeriveInput = syn::parse(input).unwrap();
	let mut map = HashMap::new();

	let data = match input.data {
		Data::Struct(data) => data,
		_ => {
			return Error::new(
				input.ident.span(),
				String::from("As of now, only structs are supported as settings component"),
			)
			.to_compile_error()
			.into();
		}
	};
	let fields = match data.fields {
		Fields::Named(fields) => fields,
		_ => {
			return Error::new(
				input.ident.span(),
				String::from("As of now, only named fields are supported as in structs settings component"),
			)
			.to_compile_error()
			.into();
		}
	};
	for field in fields.named {
		if let Visibility::Public(_) = field.vis {
			map.insert(
				field.ident.unwrap().to_string(),
				match field.ty {
					Type::Path(path) => {
						String::from(match path.path.get_ident().unwrap().to_string().as_ref() {
							"String" => "TextBox",
							"i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "i128" | "u128" => "Number",
							"bool" => "Checkbox",
							_ => {
								return Error::new(
									input.ident.span(),
									String::from("As of now, only strings, integers and boolean are supported as types in structs settings component"),
								)
								.to_compile_error()
								.into();
							}
						})
					}
					_ => {
						return Error::new(
							input.ident.span(),
							String::from("As of now, only paths are supported as types in structs settings component"),
						)
						.to_compile_error()
						.into();
					}
				},
			);
		}
	}
	let mut component = String::new();
	for (name, component_type) in map {
		component.push_str(&format!(
			r#"
	<{value} id="{key}">
	</{value}>"#,
			value = component_type,
			key = name
		));
	}
	println!("<div>{}\n</div>", component);

	let expanded = quote! {
		{

		}
	};
	TokenStream::from(expanded)
}
