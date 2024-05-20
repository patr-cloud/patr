use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route to validate user's entered email ID is available or not
	IsEmailValid,
	GET "/auth/email-valid",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	query = {
		/// The email that has to be verified
		#[preprocess(trim, email)]
		pub email: String,
	},
	response = {
		/// A boolean response corresponding to the availability of the email
		pub available: bool,
	}
);
