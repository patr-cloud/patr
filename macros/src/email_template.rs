use std::{collections::HashMap, fs, path::Path};

use handlebars::Handlebars;
use lettre::message::header::ContentType;
use proc_macro::TokenStream;
use quote::quote;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use syn::{Data, DeriveInput, Error, Fields, LitStr, Type};

#[derive(Serialize, Deserialize, Debug)]
struct EmailTemplateHtml {
	file: String,
	inline: HashMap<String, EmailTemplateAttachment>,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmailTemplatePlain {
	file: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmailTemplateAttachment {
	mime: String,
	file: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct EmailTemplate {
	html: EmailTemplateHtml,
	plain: EmailTemplatePlain,
	attachments: HashMap<String, EmailTemplateAttachment>,
}

pub fn parse(input: TokenStream) -> TokenStream {
	let input: DeriveInput = syn::parse(input).unwrap();
	let ident = input.ident;

	let data = if let Data::Struct(data) = input.data {
		data
	} else {
		return Error::new(
			ident.span(),
			String::from(
				"As of now, only structs are supported as email templates",
			),
		)
		.to_compile_error()
		.into();
	};
	let file_name = if let Some(attr) = input.attrs.first() {
		attr
	} else {
		return Error::new(
			ident.span(),
			String::from("Path to template not mentioned"),
		)
		.to_compile_error()
		.into();
	};
	let path = LitStr::new(
		&file_name
			.tokens
			.to_string()
			.replace("\"", "")
			.replace("= ", ""),
		ident.span(),
	);
	let file = match fs::read_to_string(path.value()) {
		Ok(file) => file,
		Err(err) => {
			return Error::new(
				ident.span(),
				format!("Unable to read file `{}`: {}", path.value(), err),
			)
			.to_compile_error()
			.into();
		}
	};
	let template: EmailTemplate =
		if let Ok(template) = serde_json::from_str(&file) {
			template
		} else {
			return Error::new(
				ident.span(),
				String::from("Unable to parse template file as json"),
			)
			.to_compile_error()
			.into();
		};

	let html_content_location =
		Path::new(&path.value().replace("template.json", ""))
			.join(template.html.file)
			.to_str()
			.unwrap()
			.to_string();
	let html_content = match fs::read_to_string(&html_content_location) {
		Ok(html) => html,
		Err(error) => {
			return Error::new(
				ident.span(),
				format!("Unable to read html content: {}", error),
			)
			.to_compile_error()
			.into();
		}
	};
	let plain_content_location =
		Path::new(&path.value().replace("template.json", ""))
			.join(template.plain.file)
			.to_str()
			.unwrap()
			.to_string();
	let plain_content =
		if let Ok(plain) = fs::read_to_string(&plain_content_location) {
			plain
		} else {
			return Error::new(
				ident.span(),
				String::from("Unable to read plain text content"),
			)
			.to_compile_error()
			.into();
		};

	let fields = match data.fields {
		Fields::Named(fields) => fields,
		_ => {
			return Error::new(
				ident.span(),
				String::from("As of now, only named fields are supported in email templates"),
			)
			.to_compile_error()
			.into();
		}
	};

	let mut fields_map = HashMap::new();
	for field in fields.named {
		fields_map.insert(
			field.ident.unwrap().to_string(),
			match field.ty {
				Type::Path(path) => {
					match path.path.get_ident().unwrap().to_string().as_ref() {
						"String" | "str" => Value::String("".to_string()),
						"i8" |
						"u8" |
						"i16" |
						"u16" |
						"i32" |
						"u32" |
						"i64" |
						"u64" |
						"i128" |
						"u128" |
						"usize" |
						"isize" => Value::Number(0.into()),
						"bool" => Value::Bool(true),
						_ => {
							return Error::new(
								ident.span(),
								String::from("As of now, only strings, integers and boolean are supported as types in email templates"),
							)
							.to_compile_error()
							.into();
						}
					}
				}
				_ => {
					return Error::new(
						ident.span(),
						String::from("As of now, only paths are supported as types in email templates"),
					)
					.to_compile_error()
					.into();
				}
			},
		);
	}

	let mut handlebar = Handlebars::new();
	handlebar.set_strict_mode(true);

	let html_render_result =
		handlebar.render_template(&html_content, &fields_map);
	if let Err(error) = html_render_result {
		return Error::new(
			ident.span(),
			format!("Error while rendering HTML template: {}", error),
		)
		.to_compile_error()
		.into();
	}
	let plain_text_render_result =
		handlebar.render_template(&plain_content, &fields_map);
	if let Err(error) = plain_text_render_result {
		return Error::new(
			ident.span(),
			format!("Error while rendering plain text template: {}", error),
		)
		.to_compile_error()
		.into();
	}

	let mut inline_expanded = vec![];
	for (name, inline) in template.html.inline {
		let EmailTemplateAttachment { file, mime } = inline;
		let file_location =
			Path::new(&path.value().replace("template.json", ""))
				.join(file)
				.to_str()
				.unwrap()
				.to_string();
		if let Err(error) = fs::read(&file_location) {
			return Error::new(
				ident.span(),
				format!("Error while reading inline `{}`: {}", name, error),
			)
			.to_compile_error()
			.into();
		}
		if let Err(error) = ContentType::parse(&mime) {
			return Error::new(
				ident.span(),
				format!(
					"Error while reading mime type of inline `{}`: {}",
					name, error
				),
			)
			.to_compile_error()
			.into();
		}

		inline_expanded.push(quote! {
			.singlepart(
				Attachment::new_inline(String::from(#name))
					.body(
						Body::new(
							tokio::fs::read(#file_location).await?
						),
						#mime.parse().unwrap()
					)
			)
		});
	}

	let mut attachments_expanded = vec![];
	for (name, inline) in template.attachments {
		let EmailTemplateAttachment { file, mime } = inline;
		let file_location =
			Path::new(&path.value().replace("template.json", ""))
				.join(file)
				.to_str()
				.unwrap()
				.to_string();
		if let Err(error) = fs::read(&file_location) {
			return Error::new(
				ident.span(),
				format!("Error while reading attachment `{}`: {}", name, error),
			)
			.to_compile_error()
			.into();
		}
		if let Err(error) = ContentType::parse(&mime) {
			return Error::new(
				ident.span(),
				format!(
					"Error while reading mime type of attachment `{}`: {}",
					name, error
				),
			)
			.to_compile_error()
			.into();
		}

		attachments_expanded.push(quote! {
			.singlepart(
				Attachment::new(String::from(#name))
				.body(
					Body::new(
						tokio::fs::read(#file_location).await?
					),
					#mime.parse().unwrap()
				)
			)
		});
	}

	let expanded = quote! {
		#[async_trait::async_trait]
		impl crate::models::EmailTemplate for #ident {
			async fn render_body(&self) -> Result<lettre::message::MultiPart, crate::utils::Error> {
				use lettre::message::{MultiPart, SinglePart, Attachment, Body};
				use handlebars::Handlebars;

				Ok(
					MultiPart::mixed()
						.multipart(
							MultiPart::alternative()
								.singlepart(
									SinglePart::plain(String::from(
										Handlebars::new()
											.render_template(#plain_content, &self)
											.unwrap()
									))
								)
								// .multipart(
								// 	MultiPart::related()
								// 		.singlepart(
								// 			SinglePart::html(String::from(
								// 				Handlebars::new()
								// 					.render_template(#html_content, &self)
								// 					.unwrap()
								// 			))
								// 		)
								// 		#(#inline_expanded)*
								// )
						)
						#(#attachments_expanded)*
				)
			}
		}
	};
	TokenStream::from(expanded)
}
