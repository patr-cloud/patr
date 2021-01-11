use std::collections::{HashMap, HashSet};

use proc_macro::TokenStream;
use quote::quote;
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	Error,
	Expr,
	Item,
	ItemFn,
	ItemMod,
	Lit,
	Token,
};

#[derive(Eq, PartialEq, Hash, Debug)]
enum IterTypes {
	Consts,
	Fns,
	Structs,
	Enums,
}

#[derive(Debug)]
struct IterableModule {
	iter_types: HashSet<IterTypes>,
	recursive: bool,
}

impl Parse for IterableModule {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let inputs = input.parse_terminated::<Expr, Token![,]>(Expr::parse)?;

		let mut iter_types = HashSet::new();
		let mut recursive = false;

		for input in inputs {
			match &input {
				Expr::Path(path) => {
					for segments in &path.path.segments {
						match segments.ident.to_string().as_ref() {
							"consts" => {
								iter_types.insert(IterTypes::Consts);
							}
							"fns" => {
								iter_types.insert(IterTypes::Fns);
							}
							"structs" => {
								iter_types.insert(IterTypes::Structs);
							}
							"enums" => {
								iter_types.insert(IterTypes::Enums);
							}
							_ => {
								return Err(Error::new_spanned(
									&input, "iterating is only allowed over consts, fns, structs and enums",
								))
							}
						}
					}
				}
				Expr::Assign(assign) => {
					if let Expr::Path(path) = assign.left.as_ref() {
						if let Some(segment) = path.path.segments.first() {
							if &segment.ident.to_string() != "recursive" {
								return Err(Error::new_spanned(
									&input, "allowed arguments must be of the format (consts, fns, structs, enums, recursive = true)",
								))
							}
						} else {
							return Err(Error::new_spanned(
								&input, "recursive must be set to either true or false",
							))
						}
					} else {
						return Err(Error::new_spanned(
							&input, "allowed arguments must be of the format (consts, fns, structs, enums, recursive = true)",
						))
					}

					if let Expr::Lit(lit) = assign.right.as_ref() {
						if let Lit::Bool(value) = &lit.lit {
							recursive = value.value;
						}
					}
				}
				_ => {
					return Err(Error::new_spanned(
						&input, "allowed arguments must be of the format (consts, fns, structs, enums, recursive = true)",
					))
				}
			}
		}
		Ok(IterableModule {
			iter_types,
			recursive,
		})
	}
}

pub fn parse(args: TokenStream, input: TokenStream) -> TokenStream {
	let args = parse_macro_input!(args as IterableModule);
	let module = parse_macro_input!(input as ItemMod);

	let mut iterators: HashMap<
		IterTypes,
		Vec<(String, String)>, // (name, value)
	> = HashMap::new();

	if let Err((item, message)) =
		parse_module(&module, &args, &mut iterators, args.recursive)
	{
		return Error::new_spanned(&item, &message)
			.to_compile_error()
			.into();
	}

	let mut new = ItemMod { ..module };

	let tokens = iterators
		.into_iter()
		.map(|(key, value)| {
			format!(
				r#"
		pub fn {}_iter() -> [(String, String); {}] {{
			[
				{}
			]
		}}
		"#,
				match key {
					IterTypes::Consts => "consts",
					IterTypes::Enums => "enums",
					IterTypes::Fns => "fns",
					IterTypes::Structs => "structs",
				},
				value.len(),
				value
					.into_iter()
					.map(|(key, value)| {
						format!(
							"(String::from(\"{}\"), String::from(\"{}\"))",
							key, value
						)
					})
					.collect::<Vec<_>>()
					.join(",\n")
			)
		})
		.collect::<Vec<_>>()
		.join("\n")
		.parse::<TokenStream>()
		.unwrap();

	let additional_tokens: ItemFn = parse_macro_input!(tokens as ItemFn);

	new.content
		.as_mut()
		.unwrap()
		.1
		.push(additional_tokens.into());

	let expanded = quote! {
		#new
	};

	TokenStream::from(expanded)
}

fn parse_module(
	module: &ItemMod,
	args: &IterableModule,
	iterators: &mut HashMap<IterTypes, Vec<(String, String)>>,
	recursive: bool,
) -> Result<(), (Item, String)> {
	for item in &module.content.as_ref().unwrap().1 {
		match &item {
			Item::Const(item_const) => {
				if args.iter_types.contains(&IterTypes::Consts) {
					let value = if let Expr::Lit(lit) = item_const.expr.as_ref()
					{
						match &lit.lit {
							Lit::Bool(value) => value.value.to_string(),
							Lit::Byte(value) => value.value().to_string(),
							Lit::ByteStr(value) => format!(
								"[{}]",
								value
									.value()
									.into_iter()
									.map(|item| item.to_string())
									.collect::<Vec<_>>()
									.join(", ")
							),
							Lit::Char(value) => value.value().to_string(),
							Lit::Float(value) => value.to_string(),
							Lit::Int(value) => value.to_string(),
							Lit::Str(value) => value.value(),
							Lit::Verbatim(value) => value.to_string(),
						}
					} else {
						return Err((item.clone(), String::from(
							"constants being iterated over must be a literal",
						)));
					};

					iterators.entry(IterTypes::Consts).or_insert_with(Vec::new);
					iterators
						.get_mut(&IterTypes::Consts)
						.unwrap()
						.push((item_const.ident.to_string(), value));
				}
			}
			Item::Fn(fn_item) => {
				if args.iter_types.contains(&IterTypes::Fns) {
					let fn_name = fn_item.sig.ident.to_string();

					iterators.entry(IterTypes::Fns).or_insert_with(Vec::new);
					iterators
						.get_mut(&IterTypes::Fns)
						.unwrap()
						.push((fn_name.clone(), fn_name));
				}
			}
			Item::Struct(struct_item) => {
				if args.iter_types.contains(&IterTypes::Structs) {
					let struct_name = struct_item.ident.to_string();

					iterators
						.entry(IterTypes::Structs)
						.or_insert_with(Vec::new);
					iterators
						.get_mut(&IterTypes::Structs)
						.unwrap()
						.push((struct_name.clone(), struct_name));
				}
			}
			Item::Enum(enum_item) => {
				if args.iter_types.contains(&IterTypes::Enums) {
					let enum_name = enum_item.ident.to_string();
					let values = &enum_item.variants;

					iterators.entry(IterTypes::Enums).or_insert_with(Vec::new);
					for value in values {
						iterators
							.get_mut(&IterTypes::Enums)
							.unwrap()
							.push((enum_name.clone(), value.ident.to_string()));
					}
				}
			}
			Item::Mod(item_mod) => {
				if recursive {
					parse_module(&item_mod, &args, iterators, recursive)?;
				}
			}
			_ => (),
		}
	}

	Ok(())
}
