use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Route when user verifies his identity/recovery-method by entering the OTP
	/// sent to their recovery method which is email/phone-number.
	/// This route will complete the sign-up process of the user.
	CompleteSignUp,
	POST "/auth/join",
	api = false,
	request_headers = {
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	request = {
		/// The username of the user verifying their account
		#[preprocess(trim, lowercase)]
		pub username: String,
		/// The OTP which will validate the verification
		#[preprocess(none)]
		pub verification_token: String,
	},
	response = {
		/// Upon login, the route responds with an access token and a refresh token.
		/// The access token is used to authenticate the user, implying that the user is logged in
		/// once the route is completed successfully.
		pub access_token: String,
		/// The access token has a expiry, and the refresh token (below) is used to
		/// renew the access token.
		/// It contains the login_id and the refresh_token concatinated together.
		pub refresh_token: String,
	}
);
