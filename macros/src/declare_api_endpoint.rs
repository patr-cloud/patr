use proc_macro::TokenStream;
use quote::format_ident;
use syn::{
	Attribute,
	Block,
	Error,
	Expr,
	FieldsNamed,
	Ident,
	Lit,
	LitBool,
	LitStr,
	Token,
	parse::{Parse, ParseStream},
	parse_macro_input,
	punctuated::Punctuated,
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
	/// Should this route be allowed through APIs or only through the web-login
	api_allowed: bool,

	/// The query params for the endpoint
	query: Option<FieldsNamed>,
	/// Whether the query is paginated or not.
	listable_resource: Option<Ident>,
	/// The body of the request.
	request: Option<FieldsNamed>,
	/// The required request headers for the endpoint.
	request_headers: Option<FieldsNamed>,
	/// The audit logger for the endpoint.
	audit_logger: Expr,

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
		let mut listable_resource = None;
		let mut request = None;
		let mut request_headers = None;
		let mut response_headers = None;
		let mut response = None;
		let mut api_allowed = None;
		let mut audit_logger = None;

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
				"listable_resource" => {
					if listable_resource.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					listable_resource = Some(input.parse()?);
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
				"api" => {
					if api_allowed.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					api_allowed = Some(input.parse::<LitBool>()?.value);
				}
				"audit_logger" | "audit_log" => {
					if audit_logger.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					audit_logger = Some(input.parse()?);
				}
				_ => {
					return Err(Error::new(ident.span(), "Unknown field"));
				}
			}
			if !input.is_empty() {
				input.parse::<Token![,]>()?;
			}
		}
		let api_allowed = api_allowed.unwrap_or(true);
		let Some(audit_logger) = audit_logger else {
			return Err(Error::new(input.span(), "Missing field: audit_logger"));
		};

		Ok(Self {
			documentation,
			name,
			method,
			path,
			path_body,
			auth,
			api_allowed,

			query,
			listable_resource,
			request,
			request_headers,
			audit_logger,

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
		api_allowed,

		auth,
		query,
		listable_resource,
		request_headers,
		request,
		audit_logger,

		response_headers,
		response,
	} = parse_macro_input!(input as ApiEndpoint);

	let (path_default_impl, path_body) = if let Some(body) = path_body &&
		!body.named.is_empty()
	{
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
	let (request_rename_attr, request_body, request_default_impl) = if let Some(body) = request {
		(
			quote::quote! {
				#[serde(rename_all = "camelCase")]
			},
			quote::quote! {
				#body
			},
			quote::quote! {},
		)
	} else {
		(
			quote::quote! {},
			quote::quote! {
				;
			},
			quote::quote! {
				Default,
			},
		)
	};

	let query_type_name = format_ident!("{}Query", name);
	let query_name = if query.is_some() {
		if let Some(ident) = listable_resource {
			quote::quote! {
				models::api::ListResourceQuery<#ident, #query_type_name>
			}
		} else {
			quote::quote! {
				#query_type_name
			}
		}
	} else if let Some(ident) = listable_resource {
		quote::quote! {
			models::api::ListResourceQuery<#ident, ()>
		}
	} else {
		quote::quote! {
			()
		}
	};
	let query_decl = if let Some(query) = query {
		quote::quote! {
			#[::preprocess::sync]
			/// The query params for the #name endpoint.
			///
			/// The documentation for the endpoint is below:
			///
			#[doc = #documentation]
			#[derive(
				Debug,
				Clone,
				Default,
				PartialEq,
				::ts_rs::TS,
				serde::Serialize,
				serde::Deserialize
			)]
			#[ts(optional_fields)]
			#[serde(rename_all = "camelCase")]
			pub struct #query_type_name #query

			impl models::utils::RequiresResponseHeaders for #query_name {
				type RequiredResponseHeaders = ();
			}
		}
	} else {
		quote::quote!()
	};

	let (auth_type, auth_impl) = auth.map_or_else(
		|| {
			(
				quote::quote! {
					NoAuthentication
				},
				quote::quote! {
					fn get_authenticator() -> Self::Authenticator {
						models::utils::NoAuthentication
					}
				},
			)
		},
		|block| {
			(
				quote::quote! {
					AppAuthentication::<Self>
				},
				quote::quote! {
					fn get_authenticator() -> Self::Authenticator #block
				},
			)
		},
	);

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
		let headers = FieldsNamed {
			brace_token: headers.brace_token,
			named: headers
				.named
				.into_iter()
				.map(|mut field| {
					field.attrs.push(syn::parse_quote! {
						#[ts(type = "string")]
					});
					field
				})
				.collect::<Punctuated<_, _>>(),
		};

		let default_impl = if headers.named.is_empty() {
			quote::quote! {
				Default,
			}
		} else {
			quote::quote! {}
		};

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
				::ts_rs::TS,
				macros::HasHeaders,
				#default_impl
			)]
			#[ts(export, rename_all = "camelCase")]
			pub struct #request_headers_name #headers

			impl models::utils::RequiresResponseHeaders for #request_headers_name {
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
		let headers = FieldsNamed {
			brace_token: headers.brace_token,
			named: headers
				.named
				.into_iter()
				.map(|mut field| {
					field.attrs.push(syn::parse_quote! {
						#[ts(type = "string")]
					});
					field
				})
				.collect::<Punctuated<_, _>>(),
		};

		let default_impl = if headers.named.is_empty() {
			quote::quote! {
				Default,
			}
		} else {
			quote::quote! {}
		};

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
				::ts_rs::TS,
				macros::HasHeaders,
				#default_impl
			)]
			#[ts(export, rename_all = "camelCase")]
			pub struct #response_headers_name #headers
		}
	} else {
		quote::quote!()
	};

	let response_name = format_ident!("{}Response", name);
	let (response_rename_attr, response_body, response_default_impl) = if let Some(body) = response
	{
		(
			quote::quote! {
				#[serde(rename_all = "camelCase")]
			},
			quote::quote! {
				#body
			},
			quote::quote! {},
		)
	} else {
		(
			quote::quote! {},
			quote::quote! {
				;
			},
			quote::quote! {
				Default,
			},
		)
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
			PartialEq,
			PartialOrd,
			::ts_rs::TS,
			serde::Serialize,
			serde::Deserialize,
			axum_extra::routing::TypedPath,
			#path_default_impl
		)]
		#[typed_path(#path)]
		#[ts(export, optional_fields)]
		pub struct #path_name #path_body

		impl models::utils::RequiresResponseHeaders for #path_name {
			type RequiredResponseHeaders = ();
		}

		#[::preprocess::sync]
		/// The request body for the #name endpoint
		///
		/// The documentation for the endpoint is below:
		///
		#[doc = #documentation]
		#[derive(
			Debug,
			Clone,
			PartialEq,
			::ts_rs::TS,
			serde::Serialize,
			serde::Deserialize,
			#request_default_impl
		)]
		#request_rename_attr
		#[ts(export, optional_fields)]
		pub struct #request_name #request_body

		impl models::utils::RequiresResponseHeaders for #request_name {
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
			::ts_rs::TS,
			serde::Serialize,
			serde::Deserialize,
			#response_default_impl
		)]
		#response_rename_attr
		#[ts(export, optional_fields)]
		pub struct #response_name #response_body

		impl models::utils::RequiresRequestHeaders for #response_name {
			type RequiredRequestHeaders = ();
		}

		impl models::utils::RequiresResponseHeaders for #response_name {
			type RequiredResponseHeaders = ();
		}

		impl models::api::ApiEndpoint for #request_name {
			const METHOD: ::http::Method = ::http::Method::#method;
			const API_ALLOWED: bool = #api_allowed;

			type RequestPath = #path_name;
			type RequestQuery = #query_name;
			type RequestHeaders = #request_headers_name;
			type RequestBody = Self;
			type Authenticator = models::utils::#auth_type;

			#auth_impl

			fn get_audit_logger() -> models::utils::AuditLogger<Self> {
				models::utils::AuditLogger::<Self>::#audit_logger
			}

			type ResponseHeaders = #response_headers_name;
			type ResponseBody = #response_name;
		}
	}
	.into()
}
