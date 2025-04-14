macros::declare_app_route! {
	/// Route for the page that resets the password with a token
	ResetPassword,
	"/reset-password",
	requires_login = false,
	query = {
		/// The userId to prefill the forgot password form with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub user_id: Option<String>,
		/// The reset token to prefill the forgot password form with
		#[serde(skip_serializing_if = "Option::is_none")]
		pub reset_token: Option<String>,
	},
}
