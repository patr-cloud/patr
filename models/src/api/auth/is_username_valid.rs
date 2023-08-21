macros::declare_api_endpoint!(
	IsUsernameValid,
	GET "/auth/username-valid",
	query = {
		pub username: String,
	},
	response = {
		pub available: bool,
	}
);
