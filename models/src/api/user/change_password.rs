use crate::prelude::*;

macros::declare_api_endpoint!(
	/// Change the password of the currently logged in user. This will require the
	/// user to enter their current password, and then their new password. This
	/// will then change the password of the user to the new password. Unlike
	/// forgot password, this does not require the user to enter an OTP.
	ChangePassword,
	POST "/user/change-password",
	api = false,
	request_headers = {
		/// The authorization token
		pub authorization: BearerToken,
		/// The user-agent used to access this API
		pub user_agent: UserAgent,
	},
	authentication = {
		AppAuthentication::<Self>::PlainTokenAuthenticator
	},
	request = {
		/// The current password of the user.
		pub current_password: String,
		/// The new password of the user.
		pub new_password: String,
	},
);
