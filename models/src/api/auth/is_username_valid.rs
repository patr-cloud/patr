// ***NOT NEEDED. USING PREPROCESS MACRO***

use crate::prelude::*;

macros::declare_api_endpoint!(
	// Validate username
	IsUsernameValid,
	GET "/auth/username-valid",
	request = {
		pub username: String,
	},
	response = {
		pub available: bool,
	}
);
