use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::{ToTokens, format_ident};
use syn::{
	Data,
	DataStruct,
	DeriveInput,
	Error,
	Ident,
	LitStr,
	Meta,
	MetaList,
	Path,
	Type,
	parse::Parse,
	parse_macro_input,
	spanned::Spanned,
};

/// The type of search that can be performed on a field
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
enum SearchType {
	/// Search by a resource identifier
	Resource(Ident),
	/// Search by a set of enum values
	Enum(Ident),
	/// Search by a range (e.g., a date range)
	Range,
	/// Search by a boolean value
	Bool,
	/// Search by a string value
	String,
	/// Search by a custom type
	Custom(Ident),
}

impl SearchType {
	/// Get the type to search by
	fn get_ty(&self) -> syn::Type {
		match self {
			Self::Resource(ident) => {
				syn::parse_str(format!("models::utils::ResourceSearcher<{}>", ident).as_str())
					.expect("Failed to parse resource searcher type")
			}
			Self::Enum(ident) => syn::parse_str(&format!("{}Discriminants", ident))
				.expect("Failed to parse enum discriminants type"),
			Self::Range => {
				syn::parse_str("::std::ops::RangeInclusive").expect("Failed to parse range type")
			}
			Self::Bool => syn::parse_str("bool").expect("Failed to parse bool type"),
			Self::String => syn::parse_str("String").expect("Failed to parse string type"),
			Self::Custom(ident) => {
				syn::parse_str(&format!("{}", ident)).expect("Failed to parse custom search type")
			}
		}
	}
}

/// Provides a derive macro for the `HasHeaders` trait.
pub fn parse(input: TokenStream) -> TokenStream {
	let DeriveInput {
		data, ident, vis, ..
	} = parse_macro_input!(input as DeriveInput);
	let ident_string = ident.to_string();

	let DataStruct { fields, .. } = match data {
		Data::Struct(data) => data,
		Data::Enum(data) => {
			return Error::new(data.enum_token.span(), "expected struct")
				.into_compile_error()
				.into();
		}
		Data::Union(data) => {
			return Error::new(data.union_token.span(), "expected struct")
				.into_compile_error()
				.into();
		}
	};

	let pascal_cased_fields_list = fields
		.iter()
		.filter_map(|field| {
			let Some(name) = &field.ident else {
				return Some(Err(
					Error::new(field.span(), "expected named field").into_compile_error()
				));
			};

			const SHOULD_ALLOW_ONLY_SORTABLE_ATTRS: bool = false;

			// Allow only one `sortable` attribute per field and no data inside it.
			// If there is data inside the `sortable` attribute or if there are multiple
			// `sortable` fields, we will return an error. If the attribute is not
			// present, we will not generate a field for it.
			let sortable_attr = field
				.attrs
				.iter()
				.filter(|attr| attr.path().is_ident("sortable"))
				.collect::<Vec<_>>();

			let sortable_attr = if let [attr] = &sortable_attr[..] {
				Some(attr)
			} else if sortable_attr.len() > 1 {
				return Some(Err(Error::new(
					field.span(),
					"multiple `sortable` attributes found",
				)
				.into_compile_error()));
			} else {
				None
			};

			if let Some(attr) = sortable_attr {
				if attr.meta.require_path_only().is_err() {
					return Some(Err(Error::new(
						attr.span(),
						"expected `sortable` attribute without data",
					)
					.into_compile_error()));
				}
			} else if SHOULD_ALLOW_ONLY_SORTABLE_ATTRS {
				return None; // Skip fields without `sortable` attribute
			}

			let field = format_ident!(
				"{}",
				name.to_string()
					.as_str()
					.trim_start_matches("r#")
					.to_case(Case::Pascal)
			);
			let field_string = field.to_string();
			let original_field_string = format!("{}::{}", ident_string, name.to_string());

			Some(Ok(quote::quote! {
				#[doc = "This field represents the [`"]
				#[doc = #field_string]
				#[doc = "`]["]
				#[doc = #original_field_string]
				#[doc = "] field of the [`"]
				#[doc = #ident_string]
				#[doc = "`] struct that can be used to sort resources."]
				#field
			}))
		})
		.collect::<Result<Vec<_>, _>>();

	let pascal_cased_fields_list = match pascal_cased_fields_list {
		Ok(fields) => fields,
		Err(err) => return err.into(),
	};

	if pascal_cased_fields_list.is_empty() {
		return Error::new(ident.span(), "no sortable fields found")
			.into_compile_error()
			.into();
	}

	let fields_name = format_ident!("{ident}FieldList");

	let search_struct_name = format_ident!("{}SearchParams", ident);

	let search_struct_fields = fields
		.iter()
		.filter(|&field| {
			// Check if the attribute is a `search(skip)` attribute
			// If it's a path, it has to be skip
			// If it's not a path, allow it. If it's a skip, it needs to return
			// false.

			let should_skip = field.attrs.iter().any(|attr| {
				attr.path().is_ident("search") &&
					attr.meta
						.require_list()
						.ok()
						.and_then(|list| list.parse_args_with(Path::parse).ok())
						.map(|path| path.is_ident("skip"))
						.unwrap_or(false)
			});

			!should_skip
		})
		.map(|field| {
			let Some(name) = &field.ident else {
				return Err(Error::new(field.span(), "expected named field").into_compile_error());
			};

			let mut search_type = None;
			let mut name_ident = None;
			let mut resource_ident = None;

			for attr in &field.attrs {
				/*
				List of all allowed search types:
				- `search(skip)` - skip this field
				- `search(type = "resource", resource = Deployment)` - search by a resource identifier
				- `search(type = "enum", name = MyEnumDiscriminants)` - search by a set of enum value
				- `search(type = "custom", name = MyCustomSearch)` - search by a custom type
				- `search(type = "range")` - search by a range (e.g., a date range)
				*/

				if !attr.path().is_ident("search") {
					continue; // Skip attributes that are not `search`
				}

				let meta = attr
					.parse_args::<Meta>()
					.map_err(|err| err.to_compile_error())?;

				let list = meta
					.require_list()
					.map_err(|err| err.to_compile_error())?
					.parse_args_with(MetaList::parse)
					.map_err(|err| err.to_compile_error())?;

				if list.path.is_ident("type") {
					if search_type.is_some() {
						return Err(Error::new(
							attr.span(),
							"multiple `search(type)` attributes found",
						)
						.to_compile_error());
					}

					search_type = Some(
						list.parse_args::<LitStr>()
							.map_err(|err| err.to_compile_error())?,
					);
				}

				if list.path.is_ident("resource") {
					if resource_ident.is_some() {
						return Err(Error::new(
							attr.span(),
							"multiple `search(resource)` attributes found",
						)
						.to_compile_error());
					}

					resource_ident = Some(
						list.parse_args::<Ident>()
							.map_err(|err| err.to_compile_error())?,
					);
				}

				if list.path.is_ident("name") {
					if name_ident.is_some() {
						return Err(Error::new(
							attr.span(),
							"multiple `search(name)` attributes found",
						)
						.to_compile_error());
					}

					name_ident = Some(
						list.parse_args::<Ident>()
							.map_err(|err| err.to_compile_error())?,
					);
				}
			}

			let search_type = if let Some(search_type) = search_type {
				match search_type.value().as_str() {
					"resource" => {
						let Some(resource_ident) = resource_ident else {
							return Err(Error::new(
								field.span(),
								"expected `search(type = \"resource\", resource = ..)` attribute",
							)
							.to_compile_error());
						};
						SearchType::Resource(resource_ident)
					}
					"enum" => {
						let Some(name_ident) = name_ident else {
							return Err(Error::new(
								field.span(),
								"expected `search(type = \"enum\", name = ..)` attribute",
							)
							.to_compile_error());
						};
						SearchType::Enum(name_ident)
					}
					"range" => SearchType::Range,
					"custom" => {
						let Some(name_ident) = name_ident else {
							return Err(Error::new(
								field.span(),
								"expected `search(type = \"custom\", name = ..)` attribute",
							)
							.to_compile_error());
						};
						SearchType::Custom(name_ident)
					}
					_ => {
						return Err(
							Error::new(field.span(), "unknown search type").to_compile_error()
						);
					}
				}
			} else {
				// Try to figure out the search type based on the field type
				let Type::Path(path) = &field.ty else {
					return Err(Error::new(
						field.span(),
						"cannot infer search type. Please use `search(type = \"..\")` attribute",
					)
					.to_compile_error());
				};
				let r#type = path.path.to_token_stream().to_string().replace(" ", "");
				match r#type.as_str() {
					"bool" => SearchType::Bool,
					"String" | "Option<String>" | "::std::borrow::Cow<'static,str>" => {
						SearchType::String
					}
					"OffsetDateTime" | "Option<OffsetDateTime>" => SearchType::Range,
					"Option<Vec<IpNetwork>>" | "IpAddr" => {
						SearchType::Custom(format_ident!("IpNetwork"))
					}
					_ => {
						return Err(Error::new(
							field.span(),
							"cannot infer search type. Please use `search(type = \"..\")` attribute",
						)
						.to_compile_error());
					}
				}
			};

			let name = format_ident!(
				"{}",
				name.to_string()
					.as_str()
					.trim_start_matches("r#")
					.to_case(Case::Camel)
			);
			let name_string = name.to_string();

			let search_ty = search_type.get_ty();

			Ok(quote::quote! {
				#[doc = "This field represents the [`"]
				#[doc = #name_string]
				#[doc = "`] field of the [`"]
				#[doc = #ident_string]
				#[doc = "`] struct that can be used to search resources."]
				pub #name: Option<#search_ty>,
			})
		})
		.collect::<Result<Vec<_>, _>>();

	let search_struct_fields = match search_struct_fields {
		Ok(fields) => fields,
		Err(err) => return err.into(),
	};

	quote::quote! {
		#[doc = "This enum represents the fields of the [`"]
		#[doc = #ident_string]
		#[doc = "`] struct that can be used to list resources."]
		#[doc = "It is used to specify which fields should be included in the response."]
		#[doc = "It is automatically generated by the `ListableResource` derive macro."]
		#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, ::serde::Serialize, ::serde::Deserialize, PartialOrd, Ord)]
		#[serde(rename_all = "camelCase")]
		#vis enum #fields_name {
			#(#pascal_cased_fields_list),*
		}

		impl models::utils::ListableResource for #ident {
			type FieldList = #fields_name;
			type SearchStruct = ();
		}
	}
	.into()
}
