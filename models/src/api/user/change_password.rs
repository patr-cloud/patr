use crate::{prelude::*, utils::validate_password};

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
		#[preprocess(trim, length(min = 8), custom = "validate_password")]
		pub current_password: String,
		/// The new password of the user.
		#[preprocess(trim, length(min = 8), custom = "validate_password")]
		pub new_password: String,
		/// If user has mfa enabled then mfa otp required to change password
		#[preprocess(none)]
		pub mfa_otp: Option<String>,
	},
);
