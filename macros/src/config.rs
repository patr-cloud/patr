use std::fs;

use heck::{CamelCase, SnakeCase};
use proc_macro::TokenStream;
use serde_json::{Map, Value};
use syn::{
	parse::{Parse, ParseStream},
	parse_macro_input,
	Error,
	LitStr,
	Token,
};

struct ConfigParser {
	config_name: LitStr,
	env_name: Option<LitStr>,
}

impl Parse for ConfigParser {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let config_name = input.parse()?;
		let env_name = if input.parse::<Token![,]>().is_ok() {
			Some(input.parse()?)
		} else {
			None
		};

		Ok(ConfigParser {
			config_name,
			env_name,
		})
	}
}

pub fn parse(input: TokenStream) -> TokenStream {
	let ConfigParser {
		config_name,
		env_name,
	} = parse_macro_input!(input as ConfigParser);

	let config_content = fs::read_to_string(format!(
		"./config/{}{}.json",
		config_name.value(),
		if let Some(env) = env_name {
			env.value()
		} else {
			String::new()
		}
	));
	if let Err(err) = config_content {
		return Error::new(
			config_name.span(),
			format!("Cannot read config `{}`: {}", config_name.value(), err),
		)
		.to_compile_error()
		.into();
	}
	let config_content = config_content.unwrap();

	let config_value: Value = serde_json::from_str(&config_content).unwrap();
	let config_value = config_value.as_object().unwrap();

	let mut struct_declarations = vec![String::from("pub struct Config {\n")];
	parse_declarations("config", &mut struct_declarations, 0, config_value);
	struct_declarations[0] += "}\n";

	let expanded = format!(
		"{}\npub const CONFIG: Config = {};",
		struct_declarations.join("\n"),
		parse_values("Config", config_value)
	);

	expanded.parse().unwrap()
}

fn parse_declarations(
	object_name: &str,
	struct_declarations: &mut Vec<String>,
	current_item: usize,
	object: &Map<String, Value>,
) {
	let object_name = object_name.to_camel_case();
	for (key, value) in object {
		let var_name = key.to_snake_case();
		match value {
			Value::Null => (),
			Value::Bool(_) => {
				let struct_declaration =
					struct_declarations.get_mut(current_item).unwrap();
				struct_declaration
					.push_str(&format!("pub {}: bool,\n", var_name));
			}
			Value::Number(number) => {
				let struct_declaration =
					struct_declarations.get_mut(current_item).unwrap();
				if number.is_i64() {
					struct_declaration
						.push_str(&format!("pub {}: i64,\n", var_name));
				} else if number.is_u64() {
					struct_declaration
						.push_str(&format!("pub {}: u64,\n", var_name));
				} else if number.is_f64() {
					struct_declaration
						.push_str(&format!("pub {}: f64,\n", var_name));
				} else {
					println!("Found unknown number: {:?}", number);
				}
			}
			Value::String(_) => {
				let struct_declaration =
					struct_declarations.get_mut(current_item).unwrap();
				struct_declaration
					.push_str(&format!("pub {}: &'static str,\n", var_name));
			}
			Value::Array(_) => {
				todo!();
			}
			Value::Object(map) => {
				let type_name =
					format!("{}{}", object_name, key.to_camel_case());
				let new_index = struct_declarations.len();
				struct_declarations
					.push(format!("pub struct {} {{\n", type_name));
				parse_declarations(
					&type_name,
					struct_declarations,
					new_index,
					map,
				);
				struct_declarations
					.get_mut(new_index)
					.unwrap()
					.push_str("}\n");
				struct_declarations
					.get_mut(current_item)
					.unwrap()
					.push_str(&format!("pub {}: {},\n", var_name, type_name));
			}
		}
	}
}

fn parse_values(object_name: &str, object: &Map<String, Value>) -> String {
	let mut values = String::new();
	let type_name = object_name.to_camel_case();
	values.push_str(&format!("{} {{\n", type_name));
	for (key, value) in object {
		match value {
			Value::Null => (),
			Value::Bool(value) => {
				values.push_str(&format!(
					"{}: {},",
					key.to_snake_case(),
					value
				));
			}
			Value::Number(number) => {
				values.push_str(&format!(
					"{}: {},",
					key.to_snake_case(),
					number
				));
			}
			Value::String(value) => {
				values.push_str(&format!(
					"{}: \"{}\",",
					key.to_snake_case(),
					value
				));
			}
			Value::Array(_) => {
				todo!();
			}
			Value::Object(map) => {
				values.push_str(&format!(
					"{}: {},",
					key.to_snake_case(),
					parse_values(
						&format!("{}{}", type_name, key.to_camel_case()),
						map
					)
				));
			}
		}
	}
	values.push_str("}");
	values
}
