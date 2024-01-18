use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use syn::{parse_macro_input, spanned::Spanned, Data, DataStruct, DeriveInput, Error, Field};

/// Provides a derive macro for the `HasHeaders` trait.
pub fn parse(input: TokenStream) -> TokenStream {
	let DeriveInput { data, ident, .. } = parse_macro_input!(input as DeriveInput);

	let DataStruct { fields, .. } = match data {
		Data::Struct(data) => data,
		Data::Enum(data) => {
			return Error::new(data.enum_token.span(), "expected struct")
				.into_compile_error()
				.into()
		}
		Data::Union(data) => {
			return Error::new(data.union_token.span(), "expected struct")
				.into_compile_error()
				.into()
		}
	};

	let has_header_impls = fields
		.clone()
		.into_iter()
		.map(|field| {
			let Field {
				ty,
				ident: field_ident,
				..
			} = field;
			quote::quote! {
				impl models::utils::HasHeader<#ty> for #ident {
					fn get_header(&self) -> &#ty {
						&self.#field_ident
					}
				}
			}
		})
		.collect::<TokenStream2>();

	let headers_impl = fields
		.clone()
		.into_iter()
		.map(|field| {
			let Field {
				ident: field_ident, ..
			} = field;
			quote::quote! {
				::headers::HeaderMapExt::typed_insert(&mut map, self.#field_ident.clone());
			}
		})
		.collect::<TokenStream2>();

	let from_headers_impl = fields
		.into_iter()
		.map(|field| {
			let Field { ident, ty, .. } = field;
			quote::quote! {
				#ident: ::headers::HeaderMapExt::typed_get::<#ty>(map)
					.ok_or_else(|| {
						tracing::debug!(
							"Failed to parse header `{}`",
							<#ty as ::headers::Header>::name().as_str()
						);
						::headers::Error::invalid()
					})?,
			}
		})
		.collect::<TokenStream2>();

	quote::quote! {
		#has_header_impls

		impl models::utils::Headers for #ident {
			fn to_header_map(&self) -> ::http::HeaderMap {
				let mut map = ::http::HeaderMap::new();
				#headers_impl
				map
			}

			fn from_header_map(map: &::http::HeaderMap) -> Result<Self, ::headers::Error> {
				let value = Self {
					#from_headers_impl
				};
				Ok(value)
			}
		}
	}
	.into()
}
