use proc_macro::TokenStream;
use quote::format_ident;
use syn::{
	braced,
	parse::{Parse, ParseStream},
	parse_macro_input,
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
	Variant,
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
	paginate_query: Option<bool>,
	/// The message that the client sends
	client_msg: Option<Vec<Variant>>,
	/// The required request headers for the endpoint.
	request_headers: Option<FieldsNamed>,

	/// The required response headers for the endpoint.
	response_headers: Option<FieldsNamed>,
	/// The body of the response.
	server_msg: Option<Vec<Variant>>,
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
		let mut client_msg = None;
		let mut request_headers = None;
		let mut response_headers = None;
		let mut server_msg = None;
		let mut api_allowed = None;

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
				"client_msg" => {
					if client_msg.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					let content;
					braced!(content in input);
					client_msg = Some(
						content
							.parse_terminated(Variant::parse, Token![,])?
							.into_iter()
							.collect(),
					);
				}
				"response_headers" => {
					if response_headers.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					response_headers = Some(input.parse()?);
				}
				"server_msg" => {
					if server_msg.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					let content;
					braced!(content in input);
					server_msg = Some(
						content
							.parse_terminated(Variant::parse, Token![,])?
							.into_iter()
							.collect(),
					);
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
				_ => {
					return Err(Error::new(ident.span(), "Unknown field"));
				}
			}
			if !input.is_empty() {
				input.parse::<Token![,]>()?;
			}
		}
		let api_allowed = api_allowed.unwrap_or(true);

		Ok(Self {
			documentation,
			name,
			method,
			path,
			path_body,
			auth,
			api_allowed,

			query,
			paginate_query,
			request_headers,
			client_msg,

			response_headers,
			server_msg,
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
		paginate_query,
		request_headers,
		server_msg,

		response_headers,
		client_msg,
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

	let server_msg_name = format_ident!("{}ServerMsg", name);
	let server_msg_body = if let Some(body) = server_msg {
		quote::quote! {
			#(#body),*
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
				models::api::Paginated<#name>
			}
		} else {
			quote::quote! {
				#name
			}
		}
	} else if paginate_query.unwrap_or(false) {
		quote::quote! {
			models::api::Paginated<()>
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

			impl models::utils::RequiresResponseHeaders for #query_name {
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

	let client_msg_name = format_ident!("{}ClientMsg", name);
	let client_msg_body = if let Some(body) = client_msg {
		quote::quote! {
			#(#body),*
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

		impl models::utils::RequiresResponseHeaders for #path_name {
			type RequiredResponseHeaders = ();
		}

		/// The request body for the endpoint.
		///
		/// The documentation for the endpoint is below:
		///
		#[doc = #documentation]
		#[derive(
			Copy,
			Debug,
			Clone,
			PartialEq,
		)]
		pub struct #request_name;

		#[::preprocess::sync]
		/// The message that the server sends to the client.
		///
		/// The documentation for the endpoint is below:
		///
		#[doc = #documentation]
		#[derive(
			Debug,
			Clone,
			PartialEq,
			::serde::Serialize,
			::serde::Deserialize,
		)]
		pub enum #server_msg_name {
			#server_msg_body
		}

		#query_decl

		#request_headers_decl

		#response_headers_decl

		#[::preprocess::sync]
		/// The response body for the #name endpoint.
		///
		/// The documentation for the endpoint is below:
		///
		#[doc = #documentation]
		#[derive(
			Debug,
			Clone,
			PartialEq,
			::serde::Serialize,
			::serde::Deserialize,
		)]
		pub enum #client_msg_name {
			#client_msg_body
		}

		impl models::ApiEndpoint for #request_name {
			const METHOD: ::http::Method = ::http::Method::#method;
			const API_ALLOWED: bool = #api_allowed;

			type RequestPath = #path_name;
			type RequestQuery = #query_name;
			type RequestHeaders = #request_headers_name;
			type RequestBody = models::utils::WebSocketUpgrade<#server_msg_name, #client_msg_name>;
			type Authenticator = models::utils::#auth_type;

			#auth_impl

			type ResponseHeaders = #response_headers_name;
			type ResponseBody = models::utils::GenericResponse;
		}
	}
	.into()
}
