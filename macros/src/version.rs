use std::fs;

use proc_macro::TokenStream;
use quote::quote;
use semver::Version;
use toml::Value;

use crate::compiler_error;

pub fn parse(_: TokenStream) -> TokenStream {
	let file_content = match fs::read_to_string("./api/Cargo.toml") {
		Ok(content) => content,
		Err(error) => {
			return compiler_error(format!(
				"Unable to read file api/Cargo.toml: {}",
				error
			));
		}
	};
	let toml: Value = if let Ok(value) = toml::from_str(&file_content) {
		value
	} else {
		return compiler_error(
			"Unable to read file api/Cargo.toml as a toml file",
		);
	};
	let version = if let Some(version) = toml
		.get("package")
		.map(|value| value.as_table())
		.flatten()
		.map(|package| package.get("version"))
		.flatten()
		.map(|value| value.as_str())
		.flatten()
	{
		version
	} else {
		return compiler_error(
			"Unable to read version as a string from api/Cargo.toml",
		);
	};

	let parsed_version = Version::parse(version);

	let Version {
		major,
		minor,
		patch,
		..
	} = match parsed_version {
		Ok(version) => version,
		Err(error) => {
			return compiler_error(format!(
				"Unable to parse version `{}`: {}",
				version, error
			));
		}
	};

	let expanded = quote! {
		semver::Version::new(#major, #minor, #patch)
	};
	TokenStream::from(expanded)
}
