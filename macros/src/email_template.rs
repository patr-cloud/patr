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

/// Derive macro that generates `.into_email_body()` for email template structs.
///
/// The macro:
/// 1. Generates two internal Askama wrapper structs (one for `.mjml`, one for
///    `.txt`) — Askama validates template files exist and have valid syntax at
///    compile time.
/// 2. Generates an `.into_email_body()` method that renders subject via Askama,
///    renders MJML via Askama then converts to HTML via mrml, and renders plain
///    text via Askama.
pub fn parse(input: TokenStream) -> TokenStream {
	let EmailTemplate {
		name,
		fields,
		template_path,
		subject,
	} = syn::parse_macro_input!(input as EmailTemplate);

	// Collect field names and types for the generated wrapper structs
	let field_defs: Vec<_> = fields
		.named
		.iter()
		.filter_map(|f| {
			let ident = f.ident.as_ref()?;
			let ty = &f.ty;
			Some((ident.clone(), ty.clone()))
		})
		.collect();

	let field_names: Vec<_> = field_defs.iter().map(|(ident, _)| ident.clone()).collect();
	let field_types: Vec<_> = field_defs.iter().map(|(_, ty)| ty.clone()).collect();

	// Askama template paths are relative to the dirs configured in askama.toml
	// (../assets/emails/templates). The directory layout is:
	//   {template_path}/html.mjml
	//   {template_path}/plain.txt
	let mjml_template_path = format!("{}/html.mjml", template_path);
	let txt_template_path = format!("{}/plain.txt", template_path);

	let mjml_wrapper_name = quote::format_ident!("__{}MjmlTemplate", name);
	let txt_wrapper_name = quote::format_ident!("__{}TxtTemplate", name);
	let subject_wrapper_name = quote::format_ident!("__{}SubjectTemplate", name);

	let subject_template = subject.clone();

	quote::quote! {
		// Subject template (inline source)
		#[doc(hidden)]
		#[allow(dead_code)]
		#[derive(askama::Template)]
		#[template(source = #subject_template, ext = "txt")]
		struct #subject_wrapper_name<'a> {
			#(#field_names: &'a #field_types,)*
		}

		// MJML template (file-based)
		#[doc(hidden)]
		#[allow(dead_code)]
		#[derive(askama::Template)]
		#[template(path = #mjml_template_path)]
		struct #mjml_wrapper_name<'a> {
			#(#field_names: &'a #field_types,)*
		}

		// Plain text template (file-based)
		#[doc(hidden)]
		#[allow(dead_code)]
		#[derive(askama::Template)]
		#[template(path = #txt_template_path)]
		struct #txt_wrapper_name<'a> {
			#(#field_names: &'a #field_types,)*
		}

		impl #name {
			/// Renders the subject template into a string.
			pub fn render_subject(&self) -> Result<String, crate::prelude::ErrorType> {
				use askama::Template as _;

				Ok(#subject_wrapper_name {
					#(#field_names: &self.#field_names,)*
				}.render()?)
			}

			/// Renders the MJML template into an HTML string.
			pub fn render_html(&self) -> Result<String, crate::prelude::ErrorType> {
				use askama::Template as _;

				let mjml_source = #mjml_wrapper_name {
					#(#field_names: &self.#field_names,)*
				}.render()?;

				Ok(mrml::parse(&mjml_source)?
					.element
					.render(&mrml::prelude::render::RenderOptions::default())?)
			}

			/// Renders the plain text template into a string.
			pub fn render_text(&self) -> Result<String, crate::prelude::ErrorType> {
				use askama::Template as _;

				Ok(#txt_wrapper_name {
					#(#field_names: &self.#field_names,)*
				}.render()?)
			}
		}
	}
	.into()
}
