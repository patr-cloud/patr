macros::declare_api_endpoint!(
	IsEmailValid,
	GET "/auth/email-valid",
	query = {
		pub email: String,
	},
	response = {
		pub available: bool,
	}
);
