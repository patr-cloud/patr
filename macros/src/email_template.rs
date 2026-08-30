use std::path::PathBuf;

use mrml::prelude::{
	parser::{ParserOptions, memory_loader::MemoryIncludeLoader},
	render::RenderOptions,
};
use proc_macro::TokenStream;
use syn::{
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

/// Struct that represents the input to the `EmailTemplate` derive macro.
struct EmailTemplate {
	/// The name of the struct.
	name: Ident,
	/// The fields of the struct.
	fields: FieldsNamed,
	/// The path to the email template (without extension).
	template_path: String,
	/// The subject template string (may contain `{{ field }}` placeholders).
	subject: String,
}

impl Parse for EmailTemplate {
	fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
		let data = input.parse::<ItemStruct>()?;
		let span = data.span();
		let name = data.ident;
		let Fields::Named(fields) = data.fields else {
			return Err(syn::Error::new(span, "Expected a struct with named fields"));
		};

		let template_attr = data
			.attrs
			.iter()
			.find(|attr| attr.path().is_ident("template"))
			.ok_or_else(|| syn::Error::new(span, "Missing `template` attribute"))?;

		let args = template_attr
			.meta
			.require_list()?
			.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;

		let mut template_path = None;
		let mut subject = None;

		for meta in args {
			let nv = meta.require_name_value()?;
			let Expr::Lit(ExprLit {
				lit: Lit::Str(lit), ..
			}) = &nv.value
			else {
				return Err(syn::Error::new(
					nv.value.span(),
					"Expected a string literal",
				));
			};

			if nv.path.is_ident("path") {
				template_path = Some(lit.value());
			} else if nv.path.is_ident("subject") {
				subject = Some(lit.value());
			}
		}

		let template_path = template_path
			.ok_or_else(|| syn::Error::new(span, "Missing `path` in template attribute"))?;
		let subject = subject
			.ok_or_else(|| syn::Error::new(span, "Missing `subject` in template attribute"))?;

		Ok(Self {
			name,
			fields,
			template_path,
			subject,
		})
	}
}

/// Replace `{{ ... }}` template expressions with placeholders before passing
/// MJML to mrml. This prevents mrml from mangling double quotes inside Askama
/// expressions (e.g. `{{ "file"|asset_url }}`) when they appear in attribute
/// values — mrml renders double-quoted HTML attributes, so inner `"` would
/// break the quoting. Askama doesn't support single-quoted strings (it's
/// Rust-based, so `'...'` is a char literal), hence the placeholder approach.
///
/// `start_idx` ensures unique placeholder indices across multiple files.
fn extract_template_expressions(mjml: &str, start_idx: usize) -> (String, Vec<String>) {
	let re = regex::Regex::new(r"\{\{.*?\}\}").expect("invalid regex");
	let mut expressions = Vec::new();
	let result = re
		.replace_all(mjml, |caps: &regex::Captures| {
			let idx = start_idx + expressions.len();
			expressions.push(caps[0].to_string());
			format!("__TMPL_EXPR_{idx}__")
		})
		.into_owned();
	(result, expressions)
}

/// Restore the original `{{ ... }}` template expressions from placeholders.
fn restore_template_expressions(html: &str, expressions: &[String]) -> String {
	let mut result = html.to_string();
	for (i, expr) in expressions.iter().enumerate() {
		let placeholder = format!("__TMPL_EXPR_{i}__");
		result = result.replace(&placeholder, expr);
	}
	result
}

/// Derive macro that generates `.into_email_body()` for email template structs.
///
/// The macro:
/// 1. Reads the MJML template file at compile time.
/// 2. Uses mrml to compile it to HTML (resolving `<mj-include>` directives).
/// 3. Embeds the HTML as an Askama inline source template for variable substitution and filters at
///    runtime.
/// 4. Generates `render_subject()`, `render_html()`, and `render_text()` methods.
pub fn parse(input: TokenStream) -> TokenStream {
	let EmailTemplate {
		name,
		fields,
		template_path,
		subject,
	} = syn::parse_macro_input!(input as EmailTemplate);

	// Collect field names and types for the generated wrapper structs
	let field_defs = fields
		.named
		.iter()
		.filter_map(|f| {
			let ident = f.ident.as_ref()?;
			let ty = &f.ty;
			Some((ident.clone(), ty.clone()))
		})
		.collect::<Vec<_>>();

	let field_names = field_defs
		.iter()
		.map(|(ident, _)| ident.clone())
		.collect::<Vec<_>>();
	let field_types = field_defs
		.iter()
		.map(|(_, ty)| ty.clone())
		.collect::<Vec<_>>();

	// --- Compile MJML to HTML at proc-macro time ---

	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
	let assets_dir = PathBuf::from(&manifest_dir)
		.join("../assets/emails")
		.canonicalize()
		.expect("Failed to canonicalize assets/emails path");
	let mjml_path = assets_dir
		.join("templates")
		.join(&template_path)
		.join("html.mjml");
	let mjml_source = std::fs::read_to_string(&mjml_path)
		.unwrap_or_else(|e| panic!("Failed to read {}: {}", mjml_path.display(), e));

	// Extract {{ ... }} template expressions from the main template and all
	// included component files before mrml processing.
	let mut all_expressions = Vec::new();
	let (sanitized_mjml, exprs) = extract_template_expressions(&mjml_source, 0);
	all_expressions.extend(exprs);

	// Pre-read component files, extract template expressions, then feed
	// sanitized content to mrml via MemoryIncludeLoader.
	let components_dir = assets_dir.join("components");
	let mut component_entries = Vec::new();
	if let Ok(entries) = std::fs::read_dir(&components_dir) {
		for entry in entries.flatten() {
			let path = entry.path();
			if path.extension().is_some_and(|ext| ext == "mjml") {
				let file_name = entry.file_name().to_string_lossy().into_owned();
				let content = std::fs::read_to_string(&path).unwrap_or_else(|e| {
					panic!("Failed to read component {}: {}", path.display(), e)
				});
				let (sanitized, exprs) =
					extract_template_expressions(&content, all_expressions.len());
				all_expressions.extend(exprs);
				component_entries.push((file_name, sanitized));
			}
		}
	}

	let loader = MemoryIncludeLoader::from(
		component_entries
			.iter()
			.map(|(name, content)| (name.as_str(), content.as_str()))
			.collect::<Vec<_>>(),
	);
	let options = ParserOptions {
		include_loader: Box::new(loader),
	};

	// Parse MJML and render to HTML
	let parsed = mrml::parse_with_options(&sanitized_mjml, &options)
		.unwrap_or_else(|e| panic!("Failed to parse MJML {}: {:?}", mjml_path.display(), e));
	let html = parsed
		.element
		.render(&RenderOptions::default())
		.unwrap_or_else(|e| panic!("Failed to render MJML {}: {:?}", mjml_path.display(), e));

	// Restore the original {{ ... }} template expressions
	let html_source = restore_template_expressions(&html, &all_expressions);

	// Read plain text template at compile time too
	let txt_path = assets_dir
		.join("templates")
		.join(&template_path)
		.join("plain.txt");
	let txt_source = std::fs::read_to_string(&txt_path)
		.unwrap_or_else(|e| panic!("Failed to read {}: {}", txt_path.display(), e));

	let html_wrapper_name = quote::format_ident!("__{}HtmlTemplate", name);
	let txt_wrapper_name = quote::format_ident!("__{}TxtTemplate", name);
	let subject_wrapper_name = quote::format_ident!("__{}SubjectTemplate", name);

	let subject_template = subject.clone();

	// Generate snake_case module name from struct name
	let test_mod_name = quote::format_ident!(
		"__test_{}_preview",
		name.to_string()
			.chars()
			.enumerate()
			.fold(String::new(), |mut acc, (i, c)| {
				if c.is_uppercase() {
					if i > 0 {
						acc.push('_');
					}
					acc.push(c.to_ascii_lowercase());
				} else {
					acc.push(c);
				}
				acc
			})
	);

	// Generate field initializers with sample values
	let field_initializers = field_defs
		.iter()
		.map(|(ident, ty)| {
			let ty_str = quote::quote!(#ty).to_string();
			if ty_str == "String" {
				quote::quote! { #ident: stringify!(#ident).to_string() }
			} else {
				quote::quote! { #ident: Default::default() }
			}
		})
		.collect::<Vec<_>>();

	let preview_path = format!("{}.html", template_path);

	quote::quote! {
		// Subject template (inline source)
		#[doc(hidden)]
		#[allow(dead_code)]
		#[derive(askama::Template)]
		#[template(source = #subject_template, ext = "txt")]
		struct #subject_wrapper_name<'a> {
			#(#field_names: &'a #field_types,)*
			globals: &'a crate::worker::mailer::GlobalArgs,
		}

		// HTML template (inline source — pre-compiled from MJML)
		#[doc(hidden)]
		#[allow(dead_code)]
		#[derive(askama::Template)]
		#[template(source = #html_source, ext = "html")]
		struct #html_wrapper_name<'a> {
			#(#field_names: &'a #field_types,)*
			globals: &'a crate::worker::mailer::GlobalArgs,
		}

		// Plain text template (inline source)
		#[doc(hidden)]
		#[allow(dead_code)]
		#[derive(askama::Template)]
		#[template(source = #txt_source, ext = "txt")]
		struct #txt_wrapper_name<'a> {
			#(#field_names: &'a #field_types,)*
			globals: &'a crate::worker::mailer::GlobalArgs,
		}

		impl #name {
			/// Renders the subject template into a string.
			pub fn render_subject(
				&self,
				globals: &crate::worker::mailer::GlobalArgs,
			) -> Result<String, crate::prelude::ErrorType> {
				use askama::Template as _;

				Ok(#subject_wrapper_name {
					#(#field_names: &self.#field_names,)*
					globals,
				}.render()?)
			}

			/// Renders the pre-compiled HTML template into a string.
			pub fn render_html(
				&self,
				globals: &crate::worker::mailer::GlobalArgs,
			) -> Result<String, crate::prelude::ErrorType> {
				use askama::Template as _;

				Ok(#html_wrapper_name {
					#(#field_names: &self.#field_names,)*
					globals,
				}.render()?)
			}

			/// Renders the plain text template into a string.
			pub fn render_text(
				&self,
				globals: &crate::worker::mailer::GlobalArgs,
			) -> Result<String, crate::prelude::ErrorType> {
				use askama::Template as _;

				Ok(#txt_wrapper_name {
					#(#field_names: &self.#field_names,)*
					globals,
				}.render()?)
			}
		}

		#[cfg(test)]
		mod #test_mod_name {
			use super::*;

			#[test]
			fn preview_email() {
				let template = #name {
					#(#field_initializers,)*
				};
				let globals = crate::worker::mailer::GlobalArgs::default();
				let html = template
					.render_html(&globals)
					.expect("failed to render email HTML");
				let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
					.join("../target/email-previews");
				std::fs::create_dir_all(&dir).expect("failed to create preview dir");
				let path = dir.join(#preview_path);
				std::fs::write(&path, &html).expect("failed to write preview");
				eprintln!("Email preview written to: {}", path.display());
			}
		}
	}
	.into()
}
