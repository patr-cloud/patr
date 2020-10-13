use std::{collections::HashMap, fs, io::ErrorKind};

use proc_macro::TokenStream;
use serde::{Deserialize, Serialize};
use syn::{
	export::{
		quote::{format_ident, quote},
		TokenStream2,
	},
	parse::{Parse, ParseStream},
	parse_macro_input, Error, LitStr,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct TemplateContent {
	name: String,
	html: String,
	plain: String,
	variables: HashMap<String, String>,
}

struct EmailParser {
	template_name: LitStr,
}

impl Parse for EmailParser {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		Ok(EmailParser {
			template_name: input.parse()?,
		})
	}
}

pub fn parse(input: TokenStream) -> TokenStream {
	let EmailParser { template_name } =
		parse_macro_input!(input as EmailParser);

	let file_name =
		format!("./assets/emails/{}/template.json", template_name.value());

	let file_content = fs::read_to_string(&file_name);
	if let Err(error) = file_content {
		if error.kind() == ErrorKind::NotFound {
			return Error::new(
				template_name.span(),
				format!("the file `{}` cannot be found", file_name),
			)
			.to_compile_error()
			.into();
		}
		return Error::new(
			template_name.span(),
			format!("error reading template file: {}", error.to_string()),
		)
		.to_compile_error()
		.into();
	}
	let file_content = file_content.unwrap();

	let result = serde_json::from_str(&file_content);
	if let Err(err) = result {
		return Error::new(
			template_name.span(),
			format!("error parsing template.json: {}", err.to_string()),
		)
		.to_compile_error()
		.into();
	}
	let TemplateContent {
		name,
		html,
		plain,
		variables,
	} = result.unwrap();
	let variable_and_types: TokenStream2 = variables
		.iter()
		.map(|(key, value)| format!("\t{}: {}", key, value))
		.collect::<Vec<_>>()
		.join("\n")
		.parse()
		.unwrap();
	let variable_names: TokenStream2 = variables
		.iter()
		.map(|(key, _)| key.clone())
		.collect::<Vec<_>>()
		.join(", ")
		.parse()
		.unwrap();

	let struct_name = format_ident!("{}", name);

	let html_file_name =
		format!("./assets/emails/{}/{}", template_name.value(), html);
	let html_content = fs::read_to_string(&html_file_name);
	if let Err(error) = html_content {
		if error.kind() == ErrorKind::NotFound {
			return Error::new(
				template_name.span(),
				format!("the file `{}` cannot be found", file_name),
			)
			.to_compile_error()
			.into();
		}
		return Error::new(
			template_name.span(),
			format!(
				"error reading template file `{}`: {}",
				html_file_name,
				error.to_string()
			),
		)
		.to_compile_error()
		.into();
	}
	let html_content = html_content.unwrap();

	let plain_file_name =
		format!("./assets/emails/{}/{}", template_name.value(), plain);
	let plain_content = fs::read_to_string(&plain_file_name);
	if let Err(error) = plain_content {
		if error.kind() == ErrorKind::NotFound {
			return Error::new(
				template_name.span(),
				format!("the file `{}` cannot be found", file_name),
			)
			.to_compile_error()
			.into();
		}
		return Error::new(
			template_name.span(),
			format!(
				"error reading template file `{}`: {}",
				plain_file_name,
				error.to_string()
			),
		)
		.to_compile_error()
		.into();
	}
	let plain_content = plain_content.unwrap();

	let expanded = quote! {

		#[derive(askama::Template)]
		#[template(source = #html_content, ext = "html")]
		struct #struct_name {
			#variable_and_types
		}

		impl #struct_name {
			fn render(
				#variable_and_types
			) -> lettre::message::MultiPart {
				let template = #struct_name {
					#variable_names
				};
				lettre::message::MultiPart::alternative()
					.singlepart(
						lettre::message::SinglePart::quoted_printable()
							.header(lettre::header::ContentType(
								"text/plain; charset=utf-8".parse().unwrap(),
							))
							.body(#plain_content)
					)
					.singlepart(
						lettre::message::SinglePart::eight_bit()
							.header(lettre::header::ContentType(
								"text/html; charset=utf-8".parse().unwrap(),
							))
							.body(askama::Template::render(&template).unwrap())
					)
			}
		}
	};

	TokenStream::from(expanded)
}
