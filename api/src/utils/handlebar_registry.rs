use std::{ffi::OsStr, fs};

use handlebars::{handlebars_helper, Handlebars};
use once_cell::sync::OnceCell;

use crate::utils::Error;

static INSTANCE: OnceCell<Handlebars> = OnceCell::new();

// helpers to use within handlebar templates
handlebars_helper!(cents_to_dollars: |cents: u64| crate::utils::billing::cents_to_dollars(cents));
handlebars_helper!(stringify_month: |month_in_num: u8| crate::utils::billing::stringify_month(month_in_num));

fn initialize_handlebar_registry_helper<'a>() -> Result<Handlebars<'a>, Error> {
	let mut handlebar = Handlebars::new();
	handlebar.set_strict_mode(true);

	handlebar.register_helper("stringify-month", Box::new(stringify_month));
	handlebar.register_helper("cents-to-dollars", Box::new(cents_to_dollars));

	let shared_template_folder =
		concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/emails/shared");

	for entry in fs::read_dir(shared_template_folder)? {
		let file_path = entry?.path();

		if file_path.is_file() &&
			file_path.extension() == Some(OsStr::new("hbs"))
		{
			if let Some(partial_name) = file_path
				.file_stem()
				.and_then(|file_name| file_name.to_str())
			{
				let file_content = fs::read_to_string(&file_path)?;
				handlebar.register_partial(partial_name, file_content)?;
			}
		}
	}

	Ok(handlebar)
}

pub fn initialize_handlebar_registry() {
	let handlebar = initialize_handlebar_registry_helper()
		.expect("Handler templates should be valid");

	INSTANCE
		.set(handlebar)
		.expect("Handlebar should be initialized only once");
}

pub fn get_handlebar_registry() -> &'static Handlebars<'static> {
	INSTANCE
		.get()
		.expect("Handlebar should be initialized before getting it")
}
