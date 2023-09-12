macros::declare_api_endpoint!(
	/// The route to check if a user's email ID is available to be used to create an account or not
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
