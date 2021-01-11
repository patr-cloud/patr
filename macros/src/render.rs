use std::collections::HashMap;

use handlebars::Handlebars;
use proc_macro::TokenStream;
use quote::quote;
use serde_json::{Map, Value};
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	Error,
	Expr,
	ExprAssign,
	Lit,
	LitStr,
	Token,
};

struct RenderParser {
	expr: Expr,
	file_name: LitStr,
	params: Vec<ExprAssign>,
}

impl Parse for RenderParser {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let expr = input.parse()?;
		input.parse::<Token![,]>()?;
		let file_name = input.parse()?;
		input.parse::<Token![,]>()?;
		let params = input
			.parse_terminated::<ExprAssign, Token![,]>(ExprAssign::parse)?
			.into_iter()
			.collect();

		Ok(RenderParser {
			expr,
			file_name,
			params,
		})
	}
}

pub fn parse(input: TokenStream) -> TokenStream {
	let RenderParser {
		expr,
		file_name,
		params,
	} = parse_macro_input!(input as RenderParser);
	println!("{:#?}", expr);

	let mut handlebar = Handlebars::new();
	handlebar.set_strict_mode(true);

	let error = handlebar.register_template_file("template", file_name.value());
	if let Err(err) = error {
		return Error::new(
			file_name.span(),
			format!(
				"Unable to register template `{}`: {}",
				file_name.value(),
				err
			),
		)
		.to_compile_error()
		.into();
	}
	let mut args = HashMap::new();
	let mut default_args = Map::new();
	for param in params {
		let param_name = if let Expr::Path(path) = param.left.as_ref() {
			path.path.segments.first().unwrap().ident.to_string()
		} else {
			return Error::new(
				file_name.span(),
				format!("Variable name `{:#?}` should be a valid name", param,),
			)
			.to_compile_error()
			.into();
		};
		args.insert(param_name.clone(), param.right.as_ref().clone());
		default_args.insert(
			param_name,
			match param.right.as_ref() {
				Expr::Lit(lit) => match &lit.lit {
					Lit::Bool(value) => Value::Bool(value.value),
					Lit::Str(value) => Value::String(value.value()),
					Lit::ByteStr(value) => Value::String(format!(
						"[{}]",
						value
							.value()
							.into_iter()
							.map(|b| format!("{}", b))
							.collect::<Vec<_>>()
							.join(",")
					)),
					Lit::Byte(value) => {
						Value::String(format!("{}", value.value()))
					}
					Lit::Char(value) => {
						Value::String(format!("{}", value.value()))
					}
					Lit::Int(value) => {
						Value::String(value.base10_digits().to_string())
					}
					Lit::Float(value) => {
						Value::String(value.base10_digits().to_string())
					}
					Lit::Verbatim(value) => Value::String(value.to_string()),
				},
				// Expr::Path(path) => {
				// 	path.path.
				// }
				_ => Value::Null,
			},
		);
	}
	let value = handlebar.render("template", &Value::Object(default_args));
	if let Err(err) = value {
		return Error::new(
			file_name.span(),
			format!(
				"Unable to render template `{}`: {}",
				file_name.value(),
				err
			),
		)
		.to_compile_error()
		.into();
	}
	let value = value.unwrap();

	let expanded = quote! {
		#value
	};
	TokenStream::from(expanded)
}
