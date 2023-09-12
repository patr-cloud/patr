macros::declare_api_endpoint!(
	/// The route to login and start a new user session. This route will generate the
	/// the authentication token needed to access all the services on PATR.
	Login,
	POST "/auth/sign-in",
	request = {
		/// The user ID of the user
		pub user_id: String,
		/// The password of the user
		pub password: String,
		/// If a user has a multi-factor authentication enabled, the OTP to authenticate the 
		/// identity of the user
		pub mfa_otp: Option<String>,
	},
	response = {
		/// A new access token to authenticate the user
		pub access_token: String,
		/// A new refresh token for the renewal of the access token once expired
		pub refresh_token: String,
	}
);
