use std::fs;

use handlebars::{handlebars_helper, Handlebars};
use once_cell::sync::OnceCell;

use crate::utils::Error;

static INSTANCE: OnceCell<Handlebars> = OnceCell::new();

// helpers to use within handlebar templates
handlebars_helper!(cents_to_dollars: |cents: u64| {
	crate::utils::billing::cents_to_dollars(cents)
});
handlebars_helper!(stringify_month: |month_in_num: u8| {
	crate::utils::billing::stringify_month(month_in_num)
});
handlebars_helper!(greater_than: |value1: usize,value2: usize| {
	value1 > value2
});
handlebars_helper!(less_than: |value1: usize,value2: usize| {
	value1 < value2
});
handlebars_helper!(greater_than_or_equal_to: |value1: usize,value2: usize| {
	value1 >= value2
});
handlebars_helper!(less_than_or_equal_to: |value1: usize,value2: usize| {
	value1 <= value2
});
handlebars_helper!(equal_to: |value1: usize,value2: usize| {
	value1 == value2
});

fn initialize_handlebar_registry_helper<'a>() -> Result<Handlebars<'a>, Error> {
	let mut handlebar = Handlebars::new();
	handlebar.set_strict_mode(true);

	handlebar.register_helper("stringify-month", Box::new(stringify_month));
	handlebar.register_helper("cents-to-dollars", Box::new(cents_to_dollars));
	handlebar.register_helper("greater-than", Box::new(greater_than));
	handlebar.register_helper("less-than", Box::new(greater_than));
	handlebar.register_helper("greater-than-or-equal-to", Box::new(greater_than));
	handlebar.register_helper("less-than-or-equal-to", Box::new(greater_than));
	handlebar.register_helper("equal-to", Box::new(greater_than));

	let shared_template_folder =
		concat!(env!("CARGO_MANIFEST_DIR"), "/../assets/emails/shared");

	for entry in fs::read_dir(shared_template_folder)? {
		let file_path = entry?.path();

		if !file_path.is_file() {
			continue;
		}

		let Some("hbs" | "handlebars") = file_path
			.extension()
			.and_then(|osstr| osstr.to_str()) else {
			continue;
		};

		let Some(partial_name) = file_path
			.file_stem()
			.and_then(|file_name| file_name.to_str()) else {
			continue;
		};

		let file_content = fs::read_to_string(&file_path)?;
		handlebar.register_partial(partial_name, file_content)?;
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
