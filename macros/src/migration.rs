use std::path::Path;

use proc_macro::TokenStream;
use syn::{Error, ItemFn};

/// Parses the `#[migration]` attribute macro, extracting the migration name
/// from the source filename and the version from the parent directory name.
pub fn parse(args: TokenStream, input: TokenStream) -> TokenStream {
	if let Some(item) = args.into_iter().next() {
		return Error::new(item.span().into(), "expected no arguments")
			.to_compile_error()
			.into();
	}

	let func = match syn::parse::<ItemFn>(input.clone()) {
		Ok(f) => f,
		Err(e) => return e.to_compile_error().into(),
	};
	let fn_name = &func.sig.ident;

	let file = proc_macro::Span::call_site().file();
	let path = Path::new(&file);

	let name = path
		.file_stem()
		.expect("migration file has no stem")
		.to_str()
		.expect("migration filename is not valid UTF-8")
		.to_string();

	let version_dir = path
		.parent()
		.expect("migration file has no parent directory")
		.file_name()
		.expect("parent directory has no name")
		.to_str()
		.expect("directory name is not valid UTF-8");

	let version_parts = version_dir
		.strip_prefix('v')
		.unwrap_or_else(|| {
			panic!(
				"migration version directory must start with 'v': {}",
				version_dir
			)
		})
		.split('_')
		.map(|s| {
			s.parse::<u64>()
				.unwrap_or_else(|_| panic!("invalid version segment in {}", version_dir))
		})
		.collect::<Vec<_>>();

	assert!(
		version_parts.len() == 3,
		"version directory must be v{{major}}_{{minor}}_{{patch}}, got: {}",
		version_dir
	);

	let major = version_parts[0];
	let minor = version_parts[1];
	let patch = version_parts[2];

	let original: proc_macro2::TokenStream = input.into();

	quote::quote! {
		#original

		inventory::submit! {
			crate::migrations::Migration {
				name: #name,
				version: ::semver::Version::new(#major, #minor, #patch),
				migrate: |conn, config| ::std::boxed::Box::pin(#fn_name(conn, config)),
			}
		}
	}
	.into()
}
