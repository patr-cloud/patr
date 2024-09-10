/// Contains all the failstates and errors for the deployment page
#[derive(Clone, Debug)]
pub struct DetailsPageError {
	/// Errors for the name field
	pub name: String,
	/// Errors for the registry field
	pub registry: String,
	/// Errors for the image name field
	pub image_name: String,
	/// Errors for the image tag field
	pub image_tag: String,
	/// Errors for the runner field
	pub runner: String,
}

impl DetailsPageError {
	/// Creates a new instance of the details page error, or wipe all the errors
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

/// Contains all the failstates and errors for the runner page
#[derive(Clone, Debug)]
pub struct RunnerPageError {
	/// Errors for the ports field
	pub ports: String,
}

impl RunnerPageError {
	/// Creates a new instance of the runner page error, or wipe all the errors
	pub const fn new() -> Self {
		RunnerPageError {
			ports: String::new(),
		}
	}
}