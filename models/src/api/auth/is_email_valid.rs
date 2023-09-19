macros::declare_api_endpoint!(
	/// Definition of a route to validate user's entered email ID is available or not
	IsEmailValid,
	GET "/auth/email-valid",
	query = {
		/// The email that has to be verified
		pub email: String,
	},
	response = {
		/// A boolean response corresponding to the availability of the email
		pub available: bool,
	}
);
