use models::prelude::*;

pub struct AppState {
	pub patr_token: String,
	pub region_id: Uuid,
}

impl AppState {
	pub fn try_default() -> Self {
		let patr_token = std::env::var("PATR_TOKEN");
		let region_id = std::env::var("REGION_ID");

		let patr_token = if cfg!(debug_assertions) {
			patr_token.unwrap_or_default()
		} else {
			patr_token.expect(concat!(
				"could not find environment variable PATR_TOKEN. ",
				"Please generate a token for your cluster at ",
				"the Patr dashboard and use it as an environment variable."
			))
		};
		let region_id = Uuid::parse_str(
			&if cfg!(debug_assertions) {
				region_id.unwrap_or_default()
			} else {
				region_id.expect(concat!(
					"could not find environment variable REGION_ID. ",
					"Please set the region ID of your cluster as an environment variable."
				))
			},
		);
		let region_id = if cfg!(debug_assertions) {
			region_id.unwrap_or_default()
		} else {
			region_id.expect("malformed region ID")
		};

		Self {
			patr_token,
			region_id,
		}
	}
}
