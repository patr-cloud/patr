use std::fs;

use proc_macro::TokenStream;
use quote::quote;
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

	let expanded = quote! {
		{
			let content = #config_content;
			serde_json::from_str(&content).unwrap()
		}
	};
	TokenStream::from(expanded)
}
