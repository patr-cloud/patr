// ***NOT NEEDED. USING PREPROCESS MACRO***

use crate::prelude::*;

macros::declare_api_endpoint!(
	// Validate email
	IsEmailValid,
	GET "/auth/email-valid",
	request = {
		pub email: String,
	},
	response = {
		pub available: bool,
	}
);
