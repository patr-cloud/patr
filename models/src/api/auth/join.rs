macros::declare_api_endpoint!(
	/// The route to call when a user has received an OTP and wants to complete the sign up process.
	CompleteSignUp,
	POST "/auth/join",
	request = {
		/// The username of the user verifying their account
		pub username: String,
		/// The OTP which will validate the verification
		pub verification_token: String,
	},
	response = {
		/// A new access token to authenticate the user
		pub access_token: String,
		/// A new refresh token for the renewal of the access token once expired
		pub refresh_token: String,
	}
);
