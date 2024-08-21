#[derive(Clone)]
pub struct DetailsPageError {
	pub name: String,
	pub registry: String,
	pub image_name: String,
	pub image_tag: String,
	pub runner: String,
}

impl DetailsPageError {
	pub const fn new() -> Self {
		DetailsPageError {
			name: String::new(),
			runner: String::new(),
			image_tag: String::new(),
			image_name: String::new(),
			registry: String::new(),
		}
	}
}

#[derive(Clone)]
pub struct RunnerPageError {
	pub ports: String,
}

impl RunnerPageError {
	pub const fn new() -> Self {
		RunnerPageError {
			ports: String::new(),
		}
	}
}
