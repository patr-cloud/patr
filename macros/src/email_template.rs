use std::{
	collections::{BTreeMap, HashMap},
	fs,
};

use convert_case::{Case, Casing};
use handlebars::Handlebars;
use proc_macro::TokenStream;
use regex::Regex;
use serde::{Deserialize, Serialize};
use syn::{
	Error,
	Expr,
	ExprLit,
	Fields,
	FieldsNamed,
	Ident,
	ItemStruct,
	Lit,
	Meta,
	Token,
	parse::Parse,
	punctuated::Punctuated,
	spanned::Spanned,
};

/// The data for an attachment in the email template. This struct contains the
/// MIME type of the attachment, and the file path to the attachment.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AttachmentData {
	mime: String,
	file: String,
}

/// The data that is parsed from the "template.json" file for an email template.
/// This struct contains the subject, the HTML body, the text body, and the
/// attachments for the email template.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TemplateData {
	subject: String,
	html: String,
	text: String,
	#[serde(default)]
	attachments: BTreeMap<String, AttachmentData>,
}

/// Struct that represents the input to the `EmailTemplate` derive macro.
struct EmailTemplate {
	/// The name of the struct.
	name: Ident,
	/// The fields of the struct.
	fields: FieldsNamed,
	/// The path to the email template file. This is used to load the template
	/// data from the "template.json" file.
	template_path: String,
}

impl Parse for EmailTemplate {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let data = input.parse::<ItemStruct>()?;
		let span = data.span();
		let name = data.ident;
		let Fields::Named(fields) = data.fields else {
			return Err(syn::Error::new(span, "Expected a struct with named fields"));
		};
		let template_name = data
			.attrs
			.iter()
			.find_map(|attr| {
				if attr.path().is_ident("template") {
					Some(attr)
				} else {
					None
				}
			})
			.ok_or_else(|| syn::Error::new(span, "Missing `template` attribute"))?
			.meta
			.require_list()?
			.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?
			.into_iter()
			.next()
			.ok_or_else(|| syn::Error::new(span, "Expected `template` attribute to have a value"))?
			.require_name_value()?
			.value
			.clone();

		let Expr::Lit(ExprLit {
			lit: Lit::Str(template_path),
			..
		}) = template_name
		else {
			return Err(syn::Error::new(
				span,
				"Expected `template` attribute value to be a string literal",
			));
		};

		Ok(Self {
			name,
			fields,
			template_path: template_path.value(),
		})
	}
}

/// Derive macro that generates the following methods:
/// - A `subject` method that returns the subject of the email
/// - A `html_body` method that returns the HTML body of the email
/// - A `text_body` method that returns the text body of the email
/// - An `inline_attachments` method that returns all inline attachments.
/// - An `attachments` method that returns a tuple of all attachment names.
///
/// This is used to generate email templates for the background worker.
pub fn parse(input: TokenStream) -> TokenStream {
	let EmailTemplate {
		name,
		fields,
		template_path,
	} = syn::parse_macro_input!(input as EmailTemplate);

	let emails_dir = format!(
		"{}/../assets/emails",
		std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set")
	);
	let template_dir = format!(
		"{}/{}",
		emails_dir,
		template_path.trim_matches('"') // Remove quotes from the template path
	);
	let shared_dir = format!("{}/shared", emails_dir);
	let images_dir = format!("{}/shared/images", emails_dir);
	let images = match fs::read_dir(&images_dir) {
		Ok(entries) => entries,
		Err(err) => {
			return Error::new(
				name.span(),
				format!("Failed to read images directory for email template `{name}`: {err}"),
			)
			.into_compile_error()
			.into();
		}
	}
	.filter_map(|entry| {
		let entry = entry.ok()?;
		let path = entry.path();
		if path.is_file() {
			Some(entry.file_name().to_string_lossy().to_string())
		} else {
			None
		}
	})
	.collect::<Vec<_>>();

	let template_data = match fs::read_to_string(format!("{}/template.json", template_dir)) {
		Ok(data) => data,
		Err(err) => {
			return Error::new(
				name.span(),
				format!("Failed to read template.json for email template `{name}`: {err}"),
			)
			.into_compile_error()
			.into();
		}
	};

	let TemplateData {
		subject,
		html,
		text,
		attachments: _,
	} = match serde_json::from_str(&template_data) {
		Ok(data) => data,
		Err(err) => {
			return Error::new(
				name.span(),
				format!("Failed to parse template.json for email template `{name}`: {err}"),
			)
			.into_compile_error()
			.into();
		}
	};

	let field_names = fields
		.named
		.iter()
		.filter_map(|f| f.ident.as_ref())
		.map(|f| (f.to_string().to_case(Case::Camel), "default".to_string()))
		.collect::<HashMap<_, _>>();

	let mut handlebars = Handlebars::new();

	handlebars.set_strict_mode(true);

	// Read the shared folder and register all files as partials, so that they can
	// be used in the email templates.
	let registration = match fs::read_dir(shared_dir) {
		Ok(entries) => entries,
		Err(err) => {
			return Error::new(
				name.span(),
				format!("Failed to read shared directory for email template `{name}`: {err}"),
			)
			.into_compile_error()
			.into();
		}
	}
	// Only files with .hbs or .handlebars extension
	.filter_map(|file| {
		let Ok(file) = file else {
			return None;
		};
        let file_name = file.file_name().to_string_lossy().to_string();
		if file_name.ends_with(".hbs") || file_name.ends_with(".handlebars") {
			Some(file)
		} else {
			None
		}
	})
	// Register each file as a partial
	.try_for_each(|file| {
		let content = match fs::read(&file.path()) {
			Ok(content) => content,
			Err(err) => {
				return Err(Error::new(
					name.span(),
					format!(
						"Failed to read file `{}` in shared directory for email template `{name}`: {err}",
						file.file_name().to_string_lossy()
					),
				));
			}
		};

		handlebars.register_partial(
			file.file_name()
				.to_string_lossy()
				.as_ref()
				.trim_end_matches(".handlebars")
				.trim_end_matches(".hbs"),
			String::from_utf8_lossy(&content),
		)
        .map_err(|err| {
            Error::new(
                name.span(),
                format!(
                    "Failed to register partial for file `{}` in shared directory for email template `{name}`: {err}",
                    file.file_name().to_string_lossy()
                ),
            )
        })
	});

	if let Err(err) = registration {
		return err.into_compile_error().into();
	}

	// Render the HTML template to:
	// - Extract all the CIDs used in the HTML template, so that we can load the
	//   corresponding attachments.
	// - Ensure that the template is valid and can be rendered successfully based on
	//   the fields present in the struct.
	let html_content = match fs::read_to_string(format!("{template_dir}/{html}")) {
		Ok(content) => content,
		Err(err) => {
			return Error::new(
				name.span(),
				format!("Failed to read HTML template for email template `{name}`: {err}"),
			)
			.into_compile_error()
			.into();
		}
	};
	let rendered_html = match handlebars.render_template(&html_content, &field_names) {
		Ok(rendered) => rendered,
		Err(err) => {
			return Error::new(
				name.span(),
				format!("Failed to render HTML template for email template `{name}`: {err}"),
			)
			.into_compile_error()
			.into();
		}
	};

	// For each CID, find the full path.
	// If there are multiple files with the same CID, throw an error, since we won't
	// know which one to use.
	let cid_parser = Regex::new(r#"cid:([a-zA-Z0-9_\.-]+)"#)
		.expect("Failed to compile regex for parsing CIDs in email templates");
	let cids = cid_parser
		.captures_iter(&rendered_html)
		.filter_map(|cap| cap.get(1).map(|m| m.as_str()))
		.map(|cid| {
			let mut matching_files = images.iter().filter(|file| file == &cid);

			let Some(matching_file) = matching_files.next() else {
				return Err(Error::new(
					name.span(),
					format!("No files found for CID `{cid}` in email template `{name}`"),
				));
			};

			Ok(matching_file.clone())
		})
		.collect::<Result<Vec<_>, _>>();
	let cids = match cids {
		Ok(cids) => cids,
		Err(err) => {
			return err.into_compile_error().into();
		}
	};

	quote::quote! {
		impl crate::prelude::EmailTemplate for #name {

			fn template_name(&self) -> &'static str {
				#template_path
			}

			fn subject(&self) -> &'static str {
				#subject
			}

			fn html_file(&self) -> &'static str {
				#html
			}

			fn text_file(&self) -> &'static str {
				#text
			}

			fn inline_attachments(&self) -> Vec<&'static str> {
				vec![
					#(#cids,)*
				]
			}
		}
	}
	.into()
}
