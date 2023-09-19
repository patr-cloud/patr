macros::declare_api_endpoint!(
	/// Definition of a route when user verifies his identity/recovery-method by entering the OTP
	/// sent to their recovery method which is email/phone-number. 
	/// This route will complete the sign-up process of the user.
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
