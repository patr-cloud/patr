use proc_macro::TokenStream;
use quote::format_ident;
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	Attribute,
	Error,
	Expr,
	FieldsNamed,
	Ident,
	Lit,
	LitStr,
	Token,
};

/// A helper struct to parse an App endpoint for the frontend
pub struct AppEndpoint {
	/// The documentation for the API endpoint. This is used for all the
	/// generated structs, along with some pre-text.
	documentation: String,
	/// The name of the endpoint. All generated structs will be prefixed with
	/// this name.
	name: Ident,
	/// The URL path for the route.
	path: LitStr,
	/// The body of the URL path. This is used for typed paths.
	path_body: Option<FieldsNamed>,
	/// The query params for the route
	query: Option<FieldsNamed>,
	/// Defines if this route should be allowed only when logged in or can it be
	/// accessed by anybody
	login_required: bool,
}

impl Parse for AppEndpoint {
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

		let mut query = None;
		let mut paginate_query = None;
		let mut login_required = None;
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
				"login_required" | "requires_login" => {
					if login_required.is_some() {
						return Err(Error::new(ident.span(), "Duplicate field"));
					}
					input.parse::<Token![=]>()?;

					let Lit::Bool(lit) = input.parse()? else {
						return Err(Error::new(input.span(), "Expected boolean value"));
					};

					login_required = Some(lit.value);
				}
				_ => {
					return Err(Error::new(ident.span(), "Unknown field"));
				}
			}
			if !input.is_empty() {
				input.parse::<Token![,]>()?;
			}
		}
		let login_required = login_required.unwrap_or(false);

		Ok(Self {
			documentation,
			name,
			path,
			path_body,
			login_required,
			query,
		})
	}
}

/// Declares an API endpoint. This macro allows easy definition of an API
/// endpoint along with the request URL, headers, query, body as well as the
/// response headers and body. Generates the required structs for the endpoint.
pub fn parse(input: TokenStream) -> TokenStream {
	let AppEndpoint {
		documentation,
		name,
		path,
		path_body,
		login_required,
		query,
	} = parse_macro_input!(input as AppEndpoint);

	let route_name = format_ident!("{}Route", name);
	let path_body = if let Some(body) = path_body {
		quote::quote! {
			#body
		}
	} else {
		quote::quote! {
			{}
		}
	};
	let query_name = format_ident!("{}Query", name);
	let query_name = if query.is_some() {
		quote::quote! {
			#query_name
		}
	} else {
		quote::quote! {
			()
		}
	};

	let query = query.map(|query| {
		quote::quote! {
			#[doc = #documentation]
			#[derive(
				Debug,
				Clone,
				PartialEq,
				::serde::Serialize,
				::serde::Deserialize,
				::std::default::Default,
			)]
			#[serde(rename_all = "camelCase")]
			pub struct #query_name #query

			impl ::leptos_router::Params for #query_name {
				fn from_map(map: &::leptos_router::ParamsMap) -> Result<Self, ::leptos_router::ParamsError>{
					let Ok(value) = ::serde_json::to_value(map.clone()) else {
						return Ok(Self::default());
					};
					Ok(::serde_json::from_value(value).unwrap_or_default())
				}
			}
		}
	});

	quote::quote! {

		#[doc = #documentation]
		#[derive(
			Debug,
			Clone,
			PartialEq,
			::serde::Serialize,
			::serde::Deserialize,
			::axum_extra::routing::TypedPath,
		)]
		#[serde(rename_all = "camelCase")]
		#[typed_path(#path)]
		pub struct #route_name #path_body

		#query

		// TODO: Change this to not use crate::
		impl crate::utils::TypedRoute for #route_name {
			const REQUIRES_LOGIN: bool = #login_required;

			type Query = #query_name;
		}

		impl ::leptos_router::Params for #route_name {
			fn from_map(map: &::leptos_router::ParamsMap) -> Result<Self, ::leptos_router::ParamsError>{
				let value = ::serde_json::to_value(map.clone()).map_err(|err| {
					::leptos_router::ParamsError::Params(::std::sync::Arc::new(err))
				})?;
				Ok(::serde_json::from_value(value).map_err(|err| {
					::leptos_router::ParamsError::Params(::std::sync::Arc::new(err))
				})?)
			}
		}
	}
	.into()
}
