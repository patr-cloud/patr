use proc_macro::TokenStream;
use quote::format_ident;
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	punctuated::Punctuated,
	Error,
	FieldValue,
	Ident,
	LitStr,
	Token,
};

pub struct ApiEndpoint {
	name: Ident,
	method: Ident,
	path: LitStr,
	path_body: Option<Punctuated<FieldValue, Token![,]>>,
	query: Option<(bool, Punctuated<FieldValue, Token![,]>)>,
	request: Option<Punctuated<FieldValue, Token![,]>>,
	response: Option<Punctuated<FieldValue, Token![,]>>,
}

impl Parse for ApiEndpoint {
	fn parse(input: ParseStream) -> Result<Self, Error> {
		let name = input.parse()?;
		input.parse::<Token![,]>()?;

		let method = input.parse()?;
		input.parse::<Token![,]>()?;

		let path = input.parse()?;
		let path_body = if input.peek(Token![,]) {
			input.parse::<Token![,]>()?;
			None
		} else {
			let content;
			syn::braced!(content in input);
			let path_body =
				content.parse_terminated(FieldValue::parse, Token![,])?;
			Some(path_body)
		};

		let mut query = None;
		let mut request = None;
		let mut response = None;

		for _ in 0..3 {
			if input.is_empty() {
				break;
			}
			let ident = input.parse::<Ident>()?;
			if ident == "query" || ident == "paginated_query" {
				if query.is_some() {
					return Err(Error::new(ident.span(), "Duplicate field"));
				}
				input.parse::<Token![=]>()?;
				let content;
				syn::braced!(content in input);
				query = Some((
					ident == "paginated_query",
					content.parse_terminated(FieldValue::parse, Token![,])?,
				));
			} else if ident == "request" {
				if request.is_some() {
					return Err(Error::new(ident.span(), "Duplicate field"));
				}
				input.parse::<Token![=]>()?;
				let content;
				syn::braced!(content in input);
				request = Some(
					content.parse_terminated(FieldValue::parse, Token![,])?,
				);
			} else if ident == "response" {
				if response.is_some() {
					return Err(Error::new(ident.span(), "Duplicate field"));
				}
				input.parse::<Token![=]>()?;
				let content;
				syn::braced!(content in input);
				response = Some(
					content.parse_terminated(FieldValue::parse, Token![,])?,
				);
			} else {
				return Err(Error::new(ident.span(), "Unknown field"));
			}
			input.parse::<Token![,]>()?;
		}

		Ok(Self {
			name,
			method,
			path,
			path_body,
			query,
			request,
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
		query,
		request,
		response,
	} = parse_macro_input!(input as ApiEndpoint);

	let path_body = if let Some(body) = path_body {
		quote::quote! {
			{
				#body
			}
		}
	} else {
		quote::quote! {
			;
		}
	};
	let path_name = format_ident!("{}Path", name);

	let request_name = format_ident!("{}Request", name);
	let request_body = if let Some(body) = request {
		quote::quote! {
			{
				#body
			}
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
				Eq
				Debug,
				Clone,
				PartialEq,
				serde::Serialize,
				serde::Deserialize,
			)]
			#[serde(rename_all = "camelCase")]
			pub struct #query_name #query
		}
	} else {
		quote::quote!()
	};

	let response_name = format_ident!("{}Response", name);
	let response_body = if let Some(body) = response {
		quote::quote! {
			{
				#body
			}
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
			PartialEq,
			PartialOrd,
			serde::Serialize,
			serde::Deserialize,
			axum_extra::routing::TypedPath,
		)]
		#[typed_path(#path)]
		pub struct #path_name #path_body

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

		#query_decl

		impl crate::api::ApiEndpoint for #request_name {
			const METHOD: reqwest::Method = reqwest::Method::#method;

			type RequestPath = #path_name;
			type RequestQuery = #query_name;
			type RequestBody = Self;

			type ResponseBody = #response_name;
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
		pub struct #response_name #response_body
	}
	.into()
}
