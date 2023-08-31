use proc_macro::TokenStream;
use quote::format_ident;
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	Error,
	Fields,
	FieldsNamed,
	Ident,
	LitStr,
	Token,
	Variant,
};

pub struct ApiEndpoint {
	name: Ident,
	method: Ident,
	path: LitStr,
	path_body: Option<FieldsNamed>,
	auth_type: Option<Variant>,

	query: Option<(bool, FieldsNamed)>,
	request: Option<FieldsNamed>,
	request_headers: Option<FieldsNamed>,

	response_headers: Option<FieldsNamed>,
	response: Option<FieldsNamed>,
}

impl Parse for ApiEndpoint {
	fn parse(input: ParseStream) -> Result<Self, Error> {
		let name = input.parse()?;
		input.parse::<Token![,]>()?;

		let method = input.parse()?;

		let path = input.parse()?;
		let path_body = if input.peek(Token![,]) {
			input.parse::<Token![,]>()?;
			None
		} else if input.is_empty() {
			None
		} else {
			input.parse::<Token![,]>()?;

			Some(input.parse()?)
		};

		let mut auth_type = None;
		let mut query = None;
		let mut request = None;
		let mut request_headers = None;
		let mut response_headers = None;
		let mut response = None;

		while !input.is_empty() {
			let ident = input.parse::<Ident>()?;
			match ident.to_string().as_str() {
				"query" | "paginated_query" => {
					if query.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					query = Some((ident == "paginated_query", input.parse()?));
				}
				"request_headers" => {
					if request_headers.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					request_headers = Some(input.parse()?);
				}
				"request" => {
					if request.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					request = Some(input.parse()?);
				}
				"response_headers" => {
					if response_headers.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					response_headers = Some(input.parse()?);
				}
				"response" => {
					if response.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					response = Some(input.parse()?);
				}
				"authentication" | "auth" | "authenticator" => {
					if auth_type.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					auth_type = Some(input.parse()?);
				}
				_ => {
					return Err(Error::new(ident.span(), "Unknown field"));
				}
			}
			if !input.is_empty() {
				input.parse::<Token![,]>()?;
			}
		}

		Ok(Self {
			name,
			method,
			path,
			path_body,
			auth_type,

			query,
			request_headers,
			request,

			response_headers,
			response,
		})
	}
}

pub fn parse(input: TokenStream) -> TokenStream {
	let ApiEndpoint {
		name,
		method,
		path,
		path_body,

		auth_type,
		query,
		request_headers,
		request,

		response_headers,
		response,
	} = parse_macro_input!(input as ApiEndpoint);

	let (path_default_impl, path_body) = if let Some(body) = path_body {
		(
			quote::quote! {},
			quote::quote! {
				#body
			},
		)
	} else {
		(
			quote::quote! {
				Default,
			},
			quote::quote! {
				;
			},
		)
	};
	let path_name = format_ident!("{}Path", name);

	let request_name = format_ident!("{}Request", name);
	let request_body = if let Some(body) = request {
		quote::quote! {
			#body
		}
	} else {
		quote::quote! {
			;
		}
	};

	let query_name = if let Some((paginated, _)) = &query {
		let name = format_ident!("{}Query", name);
		if *paginated {
			quote::quote! {
				crate::api::Paginated<#name>
			}
		} else {
			quote::quote! {
				#name
			}
		}
	} else {
		quote::quote! {
			()
		}
	};
	let query_decl = if let Some((_, query)) = query {
		quote::quote! {
			#[derive(
				Eq,
				Debug,
				Clone,
				PartialEq,
				serde::Serialize,
				serde::Deserialize,
			)]
			#[serde(rename_all = "camelCase")]
			pub struct #query_name #query

			impl crate::utils::RequiresResponseHeaders for #query_name {
				type RequiredResponseHeaders = ();
			}
		}
	} else {
		quote::quote!()
	};

	let auth_type = auth_type.unwrap_or_else(|| Variant {
		attrs: vec![],
		ident: format_ident!("NoAuthentication"),
		fields: Fields::Unit,
		discriminant: None,
	});

	let request_headers_name = if request_headers.is_some() {
		let ident = format_ident!("{}RequestHeaders", name);
		quote::quote! {
			#ident
		}
	} else {
		quote::quote! {
			()
		}
	};
	let request_headers_decl = if let Some(headers) = request_headers {
		quote::quote! {
			#[derive(
				Eq,
				Debug,
				Clone,
				PartialEq,
				macros::HasHeaders,
			)]
			pub struct #request_headers_name #headers

			impl crate::utils::RequiresResponseHeaders for #request_headers_name {
				type RequiredResponseHeaders = ();
			}
		}
	} else {
		quote::quote!()
	};

	let response_headers_name = if response_headers.is_some() {
		let ident = format_ident!("{}ResponseHeaders", name);
		quote::quote! {
			#ident
		}
	} else {
		quote::quote! {
			()
		}
	};
	let response_headers_decl = if let Some(headers) = response_headers {
		quote::quote! {
			#[derive(
				Eq,
				Debug,
				Clone,
				PartialEq,
				macros::HasHeaders,
			)]
			pub struct #response_headers_name #headers
		}
	} else {
		quote::quote!()
	};

	let response_name = format_ident!("{}Response", name);
	let response_body = if let Some(body) = response {
		quote::quote! {
			#body
		}
	} else {
		quote::quote! {
				;
		}
	};

	quote::quote! {
		#[derive(
			Eq,
			Hash,
			Debug,
			Clone,
			#path_default_impl
			PartialEq,
			PartialOrd,
			serde::Serialize,
			serde::Deserialize,
			axum_extra::routing::TypedPath,
		)]
		#[typed_path(#path)]
		pub struct #path_name #path_body

		impl crate::utils::RequiresResponseHeaders for #path_name {
			type RequiredResponseHeaders = ();
		}

		#[derive(
			Eq,
			Debug,
			Clone,
			PartialEq,
			serde::Serialize,
			serde::Deserialize,
		)]
		#[serde(rename_all = "camelCase")]
		pub struct #request_name #request_body

		impl crate::utils::RequiresResponseHeaders for #request_name {
			type RequiredResponseHeaders = ();
		}

		#query_decl

		#request_headers_decl

		#response_headers_decl

		#[derive(
			Eq,
			Debug,
			Clone,
			PartialEq,
			serde::Serialize,
			serde::Deserialize,
		)]
		#[serde(rename_all = "camelCase")]
		pub struct #response_name #response_body

		impl crate::utils::RequiresRequestHeaders for #response_name {
			type RequiredRequestHeaders = ();
		}

		impl crate::utils::RequiresResponseHeaders for #response_name {
			type RequiredResponseHeaders = ();
		}

		impl crate::ApiEndpoint for #request_name {
			const METHOD: ::reqwest::Method = ::reqwest::Method::#method;
			const AUTHENTICATION: crate::utils::AuthenticationType<Self> = crate::utils::AuthenticationType::<Self>::#auth_type;

			type RequestPath = #path_name;
			type RequestQuery = #query_name;
			type RequestHeaders = #request_headers_name;
			type RequestBody = Self;

			type ResponseHeaders = #response_headers_name;
			type ResponseBody = #response_name;
		}
	}
	.into()
}
