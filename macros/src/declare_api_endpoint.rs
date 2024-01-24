use proc_macro::TokenStream;
use quote::format_ident;
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	Attribute,
	Block,
	Error,
	Expr,
	FieldsNamed,
	Ident,
	Lit,
	LitStr,
	Token,
};

/// A helper struct to parse an API endpoint
pub struct ApiEndpoint {
	/// The documentation for the API endpoint. This is used for all the
	/// generated structs, along with some pre-text.
	documentation: String,
	/// The name of the endpoint. All generated structs will be prefixed with
	/// this name.
	name: Ident,
	/// The HTTP method for the endpoint.
	method: Ident,
	/// The URL path for the endpoint.
	path: LitStr,
	/// The body of the URL path. This is used for typed paths.
	path_body: Option<FieldsNamed>,
	/// The authentication for this endpoint.
	auth: Option<Block>,

	/// The query params for the endpoint
	query: Option<FieldsNamed>,
	/// Whether the query is paginated or not.
	paginate_query: Option<bool>,
	/// The body of the request.
	request: Option<FieldsNamed>,
	/// The required request headers for the endpoint.
	request_headers: Option<FieldsNamed>,

	/// The required response headers for the endpoint.
	response_headers: Option<FieldsNamed>,
	/// The body of the response.
	response: Option<FieldsNamed>,
}

impl Parse for ApiEndpoint {
	fn parse(input: ParseStream) -> Result<Self, Error> {
		let meta = Attribute::parse_outer(input)?
			.into_iter()
			.next()
			.ok_or_else(|| Error::new(input.span(), "Expected documentation"))?
			.meta;
		let Expr::Lit(ref lit) = meta.require_name_value()?.value else {
			return Err(Error::new(input.span(), "Expected documentation"));
		};

		let Lit::Str(ref lit_str) = lit.lit else {
			return Err(Error::new(input.span(), "Expected documentation"));
		};
		let documentation = lit_str.value();

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
			let body = input.parse()?;
			input.parse::<Token![,]>()?;

			Some(body)
		};

		let mut auth = None;
		let mut query = None;
		let mut paginate_query = None;
		let mut request = None;
		let mut request_headers = None;
		let mut response_headers = None;
		let mut response = None;

		while !input.is_empty() {
			let ident = input.parse::<Ident>()?;
			match ident.to_string().as_str() {
				"query" => {
					if query.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					query = Some(input.parse()?);
				}
				"pagination" => {
					if paginate_query.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					let Lit::Bool(lit) = input.parse()? else {
						return Err(Error::new(input.span(), "Expected boolean value"));
					};

					paginate_query = Some(lit.value);
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
				"authentication" | "auth" => {
					if auth.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					auth = Some(input.parse()?);
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
			documentation,
			name,
			method,
			path,
			path_body,
			auth,

			query,
			paginate_query,
			request_headers,
			request,

			response_headers,
			response,
		})
	}
}

/// Declares an API endpoint. This macro allows easy definition of an API
/// endpoint along with the request URL, headers, query, body as well as the
/// response headers and body. Generates the required structs for the endpoint.
pub fn parse(input: TokenStream) -> TokenStream {
	let ApiEndpoint {
		documentation,
		name,
		method,
		path,
		path_body,

		auth,
		query,
		paginate_query,
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

	let query_name = if query.is_some() {
		let name = format_ident!("{}Query", name);
		if paginate_query.unwrap_or(false) {
			quote::quote! {
				crate::api::Paginated<#name>
			}
		} else {
			quote::quote! {
				#name
			}
		}
	} else if paginate_query.unwrap_or(false) {
		quote::quote! {
			crate::api::Paginated<()>
		}
	} else {
		quote::quote! {
			()
		}
	};
	let query_decl = if let Some(query) = query {
		quote::quote! {
			/// The query params for the #name endpoint.
			///
			/// The documentation for the endpoint is below:
			///
			#[doc = #documentation]
			#[derive(
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

	let (auth_type, auth_impl) = auth
		.map(|block| {
			(
				quote::quote! {
					AppAuthentication::<Self>
				},
				quote::quote! {
					fn get_authenticator() -> Self::Authenticator #block
				},
			)
		})
		.unwrap_or_else(|| {
			(
				quote::quote! {
					NoAuthentication
				},
				quote::quote! {},
			)
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
			/// The required request headers for the #name endpoint.
			///
			/// The documentation for the endpoint is below:
			///
			#[doc = #documentation]
			#[derive(
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
			/// The required response headers for the #name endpoint.
			///
			/// The documentation for the endpoint is below:
			///
			#[doc = #documentation]
			#[derive(
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
		/// The URL path for the #name endpoint.
		///
		/// The documentation for the endpoint is below:
		///
		#[doc = #documentation]
		#[derive(
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

		/// The request body for the #name endpoint
		///
		/// The documentation for the endpoint is below:
		///
		#[doc = #documentation]
		#[derive(
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

		/// The response body for the #name endpoint.
		///
		/// The documentation for the endpoint is below:
		///
		#[doc = #documentation]
		#[derive(
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
			const METHOD: ::http::Method = ::http::Method::#method;

			type RequestPath = #path_name;
			type RequestQuery = #query_name;
			type RequestHeaders = #request_headers_name;
			type RequestBody = Self;
			type Authenticator = crate::utils::#auth_type;

			#auth_impl

			type ResponseHeaders = #response_headers_name;
			type ResponseBody = #response_name;
		}
	}
	.into()
}
